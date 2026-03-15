use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

use super::response::json_error_response;
use super::response::json_error_response_with_details;

pub(super) fn json_request_body(request: &HttpRequest) -> std::result::Result<Value, HttpResponse> {
    serde_json::from_slice(&request.body)
        .map_err(|_| json_error_response(400, "invalid_json", "request body must be valid JSON"))
}

pub(super) fn parse_optional_client_id(
    body: &Value,
) -> Result<Option<String>, crate::local_api::server::HttpResponse> {
    let Some(value) = body.get("client_id") else {
        return Ok(None);
    };
    let Some(client_id) = value.as_str() else {
        return Err(json_error_response_with_details(
            400,
            "validation_error",
            "client_id must be a string",
            json!({
                "field": "client_id",
                "expected": "string",
            }),
        ));
    };
    let trimmed = client_id.trim();
    if trimmed.is_empty() {
        return Err(json_error_response_with_details(
            400,
            "validation_error",
            "client_id must not be empty",
            json!({
                "field": "client_id",
                "expected": "non-empty string",
            }),
        ));
    }
    Ok(Some(trimmed.to_string()))
}
