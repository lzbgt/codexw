use serde_json::json;

use super::super::super::Cli;
use super::super::super::http;

pub(super) fn health_response(
    request: &http::HttpRequest,
    cli: &Cli,
) -> Option<http::HttpResponse> {
    if request.method == "GET" && request.path == "/healthz" {
        Some(http::json_ok_response(json!({
            "ok": true,
            "agent_id": cli.agent_id,
            "deployment_id": cli.deployment_id,
        })))
    } else {
        None
    }
}
