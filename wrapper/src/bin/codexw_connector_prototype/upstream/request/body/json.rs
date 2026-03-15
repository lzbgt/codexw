use serde_json::Value;
use serde_json::json;

use super::super::super::ForwardRequestError;
use super::policy::BodyInjectionPlan;

pub(super) fn prepare_injected_json_body(
    body: &[u8],
    plan: &BodyInjectionPlan,
) -> std::result::Result<(Option<String>, Vec<u8>), ForwardRequestError> {
    let mut object = parse_object_body(body)?;

    if let Some(session_id) = &plan.session_id_hint {
        object
            .entry("session_id".to_string())
            .or_insert(Value::String(session_id.clone()));
    }

    if let Some(client_id) = &plan.requested_client_id {
        object
            .entry("client_id".to_string())
            .or_insert(Value::String(client_id.clone()));
    }

    if let Some(lease_seconds) = &plan.requested_lease_seconds {
        let parsed = lease_seconds.parse::<u64>().map_err(|_| {
            ForwardRequestError::validation(
                "x-codexw-lease-seconds must be a positive integer header",
                Some(json!({
                    "field": "x-codexw-lease-seconds",
                    "expected": "positive integer header",
                })),
            )
        })?;
        object
            .entry("lease_seconds".to_string())
            .or_insert(Value::Number(parsed.into()));
    }

    Ok((
        Some("application/json".to_string()),
        serde_json::to_vec(&Value::Object(object))
            .map_err(anyhow::Error::from)
            .map_err(ForwardRequestError::Transport)?,
    ))
}

fn parse_object_body(
    body: &[u8],
) -> std::result::Result<serde_json::Map<String, Value>, ForwardRequestError> {
    if body.is_empty() {
        return Ok(serde_json::Map::new());
    }

    let value: Value = serde_json::from_slice(body).map_err(|_| invalid_json_object_error())?;
    let Some(object) = value.as_object() else {
        return Err(invalid_json_object_error());
    };
    Ok(object.clone())
}

fn invalid_json_object_error() -> ForwardRequestError {
    ForwardRequestError::validation(
        "connector JSON injection requires a JSON object body",
        Some(json!({
            "field": "body",
            "expected": "json object",
        })),
    )
}
