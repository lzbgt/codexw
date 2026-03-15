use super::super::super::get_request;
use super::super::super::json_body;
use super::super::super::new_command_queue;
use super::super::super::route_request;
use super::super::super::sample_snapshot;
use super::super::assert_json_path_eq;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

#[test]
fn unknown_session_id_returns_not_found() {
    let response = route_request(
        &get_request("/api/v1/session/sess_other"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        response.headers,
        vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string()
        )]
    );
    let body = json_body(&response.body);
    assert_eq!(body["local_api_version"], CODEXW_LOCAL_API_VERSION);
    assert_eq!(body["error"]["code"], "session_not_found");
}

#[test]
fn session_lifecycle_and_inspection_routes_have_explicit_contract_coverage() {
    let get_cases = [
        (
            "/api/v1/session",
            Some(("session_id", "sess_test")),
            Some(("session.scope", "process")),
        ),
        (
            "/api/v1/session/sess_test",
            Some(("session.id", "sess_test")),
            Some(("session.attachment.id", "attach:sess_test")),
        ),
    ];

    for (path, first_expectation, second_expectation) in get_cases {
        let response = route_request(
            &get_request(path),
            &sample_snapshot(),
            &new_command_queue(),
            None,
        );
        assert_eq!(
            response.status, 200,
            "expected GET contract success for {path}"
        );
        assert_eq!(
            response.headers,
            vec![(
                HEADER_LOCAL_API_VERSION.to_string(),
                CODEXW_LOCAL_API_VERSION.to_string()
            )],
            "expected local API version header for {path}"
        );
        let body = json_body(&response.body);
        assert_eq!(
            body["local_api_version"], CODEXW_LOCAL_API_VERSION,
            "expected local API version body field for {path}"
        );
        if let Some((field, value)) = first_expectation {
            assert_json_path_eq(&body, field, value, path);
        }
        if let Some((field, value)) = second_expectation {
            assert_json_path_eq(&body, field, value, path);
        }
    }
}
