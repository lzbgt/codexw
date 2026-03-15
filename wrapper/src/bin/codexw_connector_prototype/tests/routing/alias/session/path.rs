use crate::routing::decoded_session_ref_action_path;
use crate::routing::decoded_session_ref_path;
use crate::routing::local_session_path;
use crate::routing::percent_decode_path_segment;

#[test]
fn percent_decode_path_segment_decodes_valid_percent_encoded_values() {
    assert_eq!(
        percent_decode_path_segment("%40frontend.dev").as_deref(),
        Some("@frontend.dev")
    );
    assert_eq!(
        percent_decode_path_segment("dev%2Eapi").as_deref(),
        Some("dev.api")
    );
}

#[test]
fn percent_decode_path_segment_rejects_invalid_percent_sequences() {
    assert!(percent_decode_path_segment("%").is_none());
    assert!(percent_decode_path_segment("%2").is_none());
    assert!(percent_decode_path_segment("%ZZ").is_none());
}

#[test]
fn local_session_path_joins_session_and_suffix_without_extra_logic() {
    assert_eq!(
        local_session_path("sess_1", "services/dev.api/run"),
        "/api/v1/session/sess_1/services/dev.api/run"
    );
}

#[test]
fn decoded_session_ref_helpers_apply_percent_decoding_before_joining() {
    assert_eq!(
        decoded_session_ref_path("sess_1", "capabilities", "%40frontend.dev").as_deref(),
        Some("/api/v1/session/sess_1/capabilities/@frontend.dev")
    );
    assert_eq!(
        decoded_session_ref_action_path("sess_1", "services", "dev%2Eapi", "attach").as_deref(),
        Some("/api/v1/session/sess_1/services/dev.api/attach")
    );
}
