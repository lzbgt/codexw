use crate::routing::supports_client_lease_injection;

use super::ALLOWED_HTTP_ROUTES;

#[test]
fn supported_post_routes_remain_client_lease_injection_eligible() {
    for (method, path) in ALLOWED_HTTP_ROUTES {
        let expected = *method == "POST";
        assert_eq!(
            supports_client_lease_injection(method, path),
            expected,
            "unexpected client/lease injection eligibility for {method} {path}"
        );
    }
}

#[test]
fn injection_support_is_limited_to_post_routes() {
    assert!(supports_client_lease_injection(
        "POST",
        "/api/v1/session/sess_1/services/bg-1/run"
    ));
    assert!(!supports_client_lease_injection(
        "GET",
        "/api/v1/session/sess_1/services/bg-1/run"
    ));
}
