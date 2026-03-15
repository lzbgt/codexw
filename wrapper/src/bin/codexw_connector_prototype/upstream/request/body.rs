use serde_json::Value;
use serde_json::json;

use super::super::super::http::HttpRequest;
use super::super::super::routing::ProxyTarget;
use super::super::super::routing::supports_client_lease_injection;
use super::super::ForwardRequestError;

pub(super) fn prepare_upstream_body(
    request: &HttpRequest,
    target: &ProxyTarget,
) -> std::result::Result<(Option<String>, Vec<u8>), ForwardRequestError> {
    let requested_client_id = request
        .headers
        .get("x-codexw-client-id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let requested_lease_seconds = request
        .headers
        .get("x-codexw-lease-seconds")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    let requires_object_body = target.session_id_hint.is_some()
        || supports_client_lease_injection(&request.method, &target.local_path);
    if request.method != "POST" || !requires_object_body {
        return Ok((
            request.headers.get("content-type").cloned(),
            request.body.clone(),
        ));
    }

    if requested_client_id.is_none()
        && requested_lease_seconds.is_none()
        && target.session_id_hint.is_none()
    {
        return Ok((
            request.headers.get("content-type").cloned(),
            request.body.clone(),
        ));
    }

    let mut object = if request.body.is_empty() {
        serde_json::Map::new()
    } else {
        let value: Value = serde_json::from_slice(&request.body).map_err(|_| {
            ForwardRequestError::validation(
                "connector JSON injection requires a JSON object body",
                Some(json!({
                    "field": "body",
                    "expected": "json object",
                })),
            )
        })?;
        let Some(object) = value.as_object() else {
            return Err(ForwardRequestError::validation(
                "connector JSON injection requires a JSON object body",
                Some(json!({
                    "field": "body",
                    "expected": "json object",
                })),
            ));
        };
        object.clone()
    };

    if let Some(session_id) = &target.session_id_hint {
        object
            .entry("session_id".to_string())
            .or_insert(Value::String(session_id.clone()));
    }

    if let Some(client_id) = requested_client_id {
        object
            .entry("client_id".to_string())
            .or_insert(Value::String(client_id));
    }
    if let Some(lease_seconds) = requested_lease_seconds {
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
