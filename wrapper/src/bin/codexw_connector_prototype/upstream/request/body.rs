#[path = "body/json.rs"]
mod json;
#[path = "body/policy.rs"]
mod policy;

use super::super::super::http::HttpRequest;
use super::super::super::routing::ProxyTarget;
use super::super::ForwardRequestError;

pub(super) fn prepare_upstream_body(
    request: &HttpRequest,
    target: &ProxyTarget,
) -> std::result::Result<(Option<String>, Vec<u8>), ForwardRequestError> {
    let plan = policy::build_injection_plan(request, target);
    if plan.passes_through() {
        return Ok((plan.content_type.clone(), request.body.clone()));
    }

    json::prepare_injected_json_body(&request.body, &plan)
}
