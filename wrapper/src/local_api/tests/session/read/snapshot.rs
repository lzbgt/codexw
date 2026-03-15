use serde_json::Value;

use super::super::super::get_request;
use super::super::super::json_body;
use super::super::super::new_command_queue;
use super::super::super::route_request;
use super::super::super::sample_snapshot;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

#[test]
fn session_snapshot_is_returned_with_valid_token() {
    let mut request = get_request("/api/v1/session");
    request
        .headers
        .insert("authorization".to_string(), "Bearer secret".to_string());
    let response = route_request(
        &request,
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string()
        )]
    );
    let body = json_body(&response.body);
    assert_eq!(body["local_api_version"], CODEXW_LOCAL_API_VERSION);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["session"]["id"], "sess_test");
    assert_eq!(body["session"]["scope"], "process");
    assert_eq!(body["session"]["attachment"]["id"], "attach:sess_test");
    assert_eq!(body["session"]["attachment"]["client_id"], "client_web");
    assert_eq!(body["session"]["attachment"]["lease_seconds"], 300);
    assert_eq!(body["session"]["attached_thread_id"], "thread_123");
    assert_eq!(body["thread_id"], "thread_123");
    assert_eq!(body["working"], Value::Bool(true));
    assert_eq!(
        body["async_tool_supervision"]["classification"],
        "tool_slow"
    );
    assert_eq!(
        body["async_tool_supervision"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        body["async_tool_supervision"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(body["async_tool_supervision"]["request_id"], "7");
    assert_eq!(
        body["async_tool_supervision"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(body["async_tool_supervision"]["source_call_id"], "call_1");
    assert_eq!(
        body["async_tool_supervision"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        body["async_tool_supervision"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        body["async_tool_supervision"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(
        body["async_tool_supervision"]["recovery_options"][0]["kind"],
        "observe_status"
    );
    assert_eq!(
        body["async_tool_supervision"]["recovery_options"][1]["local_api_path"],
        "/api/v1/session/sess_test/turn/interrupt"
    );
    assert_eq!(
        body["async_tool_supervision"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["async_tool_supervision"]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["async_tool_supervision"]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        body["async_tool_supervision"]["observed_background_shell_job"]["command"],
        "npm run dev"
    );
    assert_eq!(
        body["async_tool_supervision"]["observed_background_shell_job"]["last_output_age_seconds"],
        2
    );
    assert_eq!(body["async_tool_supervision"]["next_check_in_seconds"], 9);
    assert_eq!(
        body["async_tool_backpressure"]["abandoned_request_count"],
        1
    );
    assert_eq!(
        body["async_tool_backpressure"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        body["async_tool_backpressure"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(
        body["async_tool_backpressure"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        body["async_tool_backpressure"]["recovery_options"][0]["kind"],
        "observe_status"
    );
    assert_eq!(
        body["async_tool_backpressure"]["recovery_options"][1]["local_api_path"],
        "/api/v1/session/sess_test/turn/interrupt"
    );
    assert_eq!(
        body["async_tool_backpressure"]["recovery_options"][2]["cli_command"],
        "codexw --cwd /tmp/repo resume thread_123"
    );
    assert_eq!(body["async_tool_backpressure"]["oldest_request_id"], "8");
    assert_eq!(
        body["async_tool_backpressure"]["oldest_thread_name"],
        "codexw-bgtool-background_shell_start-8"
    );
    assert_eq!(
        body["async_tool_backpressure"]["saturation_threshold"],
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_hard_timeout_seconds"],
        15
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_source_call_id"],
        "call_2"
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["async_tool_backpressure"]["oldest_observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(body["async_tool_workers"][0]["request_id"], "7");
    assert_eq!(body["async_tool_workers"][0]["lifecycle_state"], "running");
    assert_eq!(
        body["async_tool_workers"][0]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        body["async_tool_workers"][0]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["async_tool_workers"][0]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["async_tool_workers"][0]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(body["async_tool_workers"][0]["source_call_id"], "call_1");
    assert_eq!(
        body["async_tool_workers"][0]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        body["async_tool_workers"][0]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        body["async_tool_workers"][0]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        body["async_tool_workers"][0]["observed_background_shell_job"]["last_output_age_seconds"],
        2
    );
    assert_eq!(body["async_tool_workers"][0]["next_check_in_seconds"], 9);
    assert_eq!(
        body["async_tool_workers"][1]["lifecycle_state"],
        "abandoned_after_timeout"
    );
    assert_eq!(body["async_tool_workers"][1]["source_call_id"], "call_2");
    assert_eq!(
        body["async_tool_workers"][1]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        body["async_tool_workers"][1]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        body["async_tool_workers"][1]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["async_tool_workers"][1]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["async_tool_workers"][1]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(body["async_tool_backpressure"]["saturated"], false);
    assert_eq!(body["supervision_notice"]["classification"], "tool_slow");
    assert_eq!(body["supervision_notice"]["request_id"], "7");
    assert_eq!(
        body["supervision_notice"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        body["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(body["supervision_notice"]["source_call_id"], "call_1");
    assert_eq!(
        body["supervision_notice"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        body["supervision_notice"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        body["supervision_notice"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["supervision_notice"]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["supervision_notice"]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["tool"],
        "background_shell_start"
    );
    assert_eq!(
        body["session"]["async_tool_backpressure"]["oldest_tool"],
        "background_shell_start"
    );
    assert_eq!(
        body["session"]["async_tool_backpressure"]["oldest_observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["session"]["async_tool_backpressure"]["oldest_request_id"],
        "8"
    );
    assert_eq!(
        body["session"]["async_tool_backpressure"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(
        body["session"]["async_tool_backpressure"]["recovery_options"][2]["kind"],
        "exit_and_resume"
    );
    assert_eq!(
        body["session"]["async_tool_workers"][0]["supervision_classification"],
        "tool_slow"
    );
    assert_eq!(
        body["session"]["async_tool_workers"][0]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["session"]["async_tool_workers"][0]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["session"]["async_tool_workers"][0]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        body["session"]["async_tool_workers"][1]["supervision_classification"],
        Value::Null
    );
    assert_eq!(
        body["session"]["supervision_notice"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        body["session"]["supervision_notice"]["recovery_options"][0]["kind"],
        "observe_status"
    );
    assert_eq!(body["orchestration"]["main_agent_state"], "blocked");
}

#[test]
fn session_id_route_reuses_same_snapshot_payload() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string()
        )]
    );
    let body = json_body(&response.body);
    assert_eq!(body["local_api_version"], CODEXW_LOCAL_API_VERSION);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["session"]["active_turn_id"], "turn_456");
    assert_eq!(body["session"]["attachment"]["scope"], "process");
    assert_eq!(body["session"]["attachment"]["client_id"], "client_web");
    assert_eq!(
        body["session"]["async_tool_supervision"]["classification"],
        "tool_slow"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(body["session"]["async_tool_supervision"]["request_id"], "7");
    assert_eq!(
        body["session"]["async_tool_supervision"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(
        body["session"]["async_tool_supervision"]["recovery_options"][1]["kind"],
        "interrupt_turn"
    );
    assert_eq!(
        body["session"]["async_tool_backpressure"]["abandoned_request_count"],
        1
    );
    assert_eq!(
        body["session"]["async_tool_workers"][0]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["classification"],
        "tool_slow"
    );
    assert_eq!(body["session"]["supervision_notice"]["request_id"], "7");
    assert_eq!(
        body["session"]["supervision_notice"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["source_call_id"],
        "call_1"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        body["session"]["supervision_notice"]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(body["active_turn_id"], "turn_456");
}
