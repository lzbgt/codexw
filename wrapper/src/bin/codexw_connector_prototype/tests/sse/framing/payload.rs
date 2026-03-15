use serde_json::Value;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::sse::wrap_event_payload;

#[test]
fn wrap_event_payload_preserves_json_and_adds_broker_metadata() {
    let wrapped = wrap_event_payload(
        vec![r#"{"session_id":"sess_1","value":1}"#.to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");
    assert_eq!(json["source"], "codexw");
    assert_eq!(json["broker"]["agent_id"], "codexw-lab");
    assert_eq!(json["broker"]["deployment_id"], "mac-mini-01");
    assert_eq!(
        json["broker"]["adapter_version"],
        CODEXW_BROKER_ADAPTER_VERSION
    );
    assert_eq!(json["data"]["session_id"], "sess_1");
    assert_eq!(json["data"]["value"], 1);
}

#[test]
fn wrap_event_payload_falls_back_to_string_for_non_json_data() {
    let wrapped = wrap_event_payload(
        vec!["plain text update".to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");
    assert_eq!(json["data"], "plain text update");
}

#[test]
fn wrap_event_payload_preserves_supervision_and_backpressure_contract_fields() {
    let wrapped = wrap_event_payload(
        vec![r#"{"session":{"supervision_notice":{"classification":"tool_wedged","recommended_action":"interrupt_or_exit_resume","recovery_policy":{"kind":"operator_interrupt_or_exit_resume","automation_ready":false},"recovery_options":[{"kind":"observe_status"},{"kind":"interrupt_turn"},{"kind":"exit_and_resume"}],"request_id":"7","thread_name":"codexw-async-7","owner":"wrapper_background_shell","source_call_id":"call-7","target_background_shell_reference":"dev.api","target_background_shell_job_id":"bg-1","observation_state":"recent_output_observed","output_state":"recent_output_observed","observed_background_shell_job":{"job_id":"bg-1","status":"running","command":"npm run dev"}},"async_tool_backpressure":{"abandoned_request_count":2,"saturated":true,"recommended_action":"interrupt_or_exit_resume","recovery_policy":{"kind":"operator_interrupt_or_exit_resume","automation_ready":false},"recovery_options":[{"kind":"observe_status"},{"kind":"interrupt_turn"},{"kind":"exit_and_resume"}],"oldest_request_id":"7","oldest_thread_name":"codexw-async-7","oldest_source_call_id":"call-7","oldest_target_background_shell_reference":"dev.api","oldest_target_background_shell_job_id":"bg-1","oldest_observation_state":"recent_output_observed","oldest_output_state":"recent_output_observed","oldest_observed_background_shell_job":{"job_id":"bg-1","status":"running","command":"npm run dev"}}}}"#.to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");

    assert_eq!(
        json["data"]["session"]["supervision_notice"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        json["data"]["session"]["supervision_notice"]["recovery_policy"]["kind"],
        "operator_interrupt_or_exit_resume"
    );
    assert_eq!(
        json["data"]["session"]["supervision_notice"]["thread_name"],
        "codexw-async-7"
    );
    assert_eq!(
        json["data"]["session"]["async_tool_backpressure"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        json["data"]["session"]["async_tool_backpressure"]["recovery_options"][2]["kind"],
        "exit_and_resume"
    );
    assert_eq!(
        json["data"]["session"]["async_tool_backpressure"]["oldest_request_id"],
        "7"
    );
    assert_eq!(
        json["data"]["session"]["async_tool_backpressure"]["oldest_observed_background_shell_job"]
            ["job_id"],
        "bg-1"
    );
}
