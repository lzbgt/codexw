use serde_json::Value;
use serde_json::json;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;

use super::super::super::HttpResponse;

pub(super) fn json_ok_response(body: Value) -> HttpResponse {
    let body = match body {
        Value::Object(mut object) => {
            object.insert(
                "broker_adapter_version".to_string(),
                Value::String(CODEXW_BROKER_ADAPTER_VERSION.to_string()),
            );
            Value::Object(object)
        }
        other => other,
    };
    HttpResponse {
        status: 200,
        reason: "OK",
        headers: vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string(),
            ),
        ],
        body: serde_json::to_vec(&body).expect("serialize ok response"),
    }
}

pub(super) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> HttpResponse {
    let mut error = json!({
        "status": status,
        "code": code,
        "message": message,
    });
    if let Some(details) = details {
        error["details"] = details;
    }
    let body = json!({
        "ok": false,
        "broker_adapter_version": CODEXW_BROKER_ADAPTER_VERSION,
        "error": error,
    });
    HttpResponse {
        status,
        reason: match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            502 => "Bad Gateway",
            _ => "Error",
        },
        headers: vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string(),
            ),
        ],
        body: serde_json::to_vec(&body).expect("serialize error response"),
    }
}
