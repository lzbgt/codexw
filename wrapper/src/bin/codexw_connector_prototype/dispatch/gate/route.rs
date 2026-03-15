use serde_json::json;

use super::super::super::Cli;
use super::super::super::http;
use super::super::super::routing;

pub(crate) struct ProxyRequest {
    pub(crate) request: http::HttpRequest,
    pub(crate) target: routing::ProxyTarget,
}

pub(super) fn resolve_proxy_request(
    request: http::HttpRequest,
    cli: &Cli,
) -> std::result::Result<ProxyRequest, http::HttpResponse> {
    let Some(target) = routing::resolve_proxy_target(&request.method, &request.path, &cli.agent_id)
    else {
        return Err(http::json_error_response(
            404,
            "not_found",
            "unknown connector route",
            None,
        ));
    };

    if target.is_sse && request.method != "GET" {
        return Err(http::json_error_response(
            405,
            "method_not_allowed",
            "unsupported method for SSE route",
            None,
        ));
    }

    if !routing::is_allowed_local_proxy_target(&request.method, &target.local_path, target.is_sse) {
        return Err(http::json_error_response(
            403,
            "route_not_allowed",
            "connector route is outside the allowed local API surface",
            Some(json!({
                "method": request.method,
                "local_path": target.local_path,
                "is_sse": target.is_sse,
            })),
        ));
    }

    Ok(ProxyRequest { request, target })
}
