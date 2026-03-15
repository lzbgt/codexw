use super::super::super::Cli;
use super::super::super::http;

pub(super) fn auth_error(request: &http::HttpRequest, cli: &Cli) -> Option<http::HttpResponse> {
    if let Some(expected_token) = &cli.connector_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => None,
            _ => Some(http::json_error_response(
                401,
                "unauthorized",
                "missing or invalid connector bearer token",
                None,
            )),
        }
    } else {
        None
    }
}
