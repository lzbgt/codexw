use serde_json::Value;
use serde_json::json;

use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::local_api::server::HttpResponse;

pub(super) fn json_ok_response(body: serde_json::Value) -> HttpResponse {
    let body = match body {
        Value::Object(mut object) => {
            object.insert(
                "local_api_version".to_string(),
                Value::String(CODEXW_LOCAL_API_VERSION.to_string()),
            );
            Value::Object(object)
        }
        other => other,
    };
    HttpResponse {
        status: 200,
        reason: "OK",
        headers: vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string(),
        )],
        body: serde_json::to_vec_pretty(&body).unwrap_or_else(|_| b"{\"ok\":false}".to_vec()),
    }
}

pub(super) fn json_error_response(status: u16, code: &str, message: &str) -> HttpResponse {
    json_error_response_with_details(status, code, message, json!({}))
}

pub(super) fn json_error_response_with_details(
    status: u16,
    code: &str,
    message: &str,
    details: serde_json::Value,
) -> HttpResponse {
    let reason = match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Error",
    };
    json_ok_response(json!({
        "ok": false,
        "error": {
            "status": status,
            "code": code,
            "message": message,
            "retryable": status >= 500,
            "details": details,
        }
    }))
    .with_status(status, reason)
}

impl HttpResponse {
    fn with_status(mut self, status: u16, reason: &'static str) -> Self {
        self.status = status;
        self.reason = reason;
        self
    }
}
