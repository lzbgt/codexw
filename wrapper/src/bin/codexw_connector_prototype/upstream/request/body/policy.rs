use super::super::super::super::http::HttpRequest;
use super::super::super::super::routing::ProxyTarget;
use super::super::super::super::routing::supports_client_lease_injection;

#[derive(Debug, Clone)]
pub(super) struct BodyInjectionPlan {
    pub(super) content_type: Option<String>,
    pub(super) session_id_hint: Option<String>,
    pub(super) requested_client_id: Option<String>,
    pub(super) requested_lease_seconds: Option<String>,
    requires_object_body: bool,
}

impl BodyInjectionPlan {
    pub(super) fn passes_through(&self) -> bool {
        !self.requires_object_body
            || (self.session_id_hint.is_none()
                && self.requested_client_id.is_none()
                && self.requested_lease_seconds.is_none())
    }
}

pub(super) fn build_injection_plan(
    request: &HttpRequest,
    target: &ProxyTarget,
) -> BodyInjectionPlan {
    BodyInjectionPlan {
        content_type: request.headers.get("content-type").cloned(),
        session_id_hint: target.session_id_hint.clone(),
        requested_client_id: request
            .headers
            .get("x-codexw-client-id")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        requested_lease_seconds: request
            .headers
            .get("x-codexw-lease-seconds")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        requires_object_body: request.method == "POST"
            && (target.session_id_hint.is_some()
                || supports_client_lease_injection(&request.method, &target.local_path)),
    }
}
