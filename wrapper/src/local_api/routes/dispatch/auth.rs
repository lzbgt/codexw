use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

use super::super::json_error_response;
use super::super::json_ok_response;

pub(super) fn authorize_request(
    request: &HttpRequest,
    auth_token: Option<&str>,
) -> Option<HttpResponse> {
    if request.path == "/healthz" && request.method == "GET" {
        return Some(json_ok_response(serde_json::json!({ "ok": true })));
    }

    if let Some(expected_token) = auth_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => {}
            _ => {
                return Some(json_error_response(
                    401,
                    "unauthorized",
                    "missing or invalid bearer token",
                ));
            }
        }
    }
    None
}
