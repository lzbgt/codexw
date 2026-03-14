use super::*;
use std::io::Write;

#[test]
fn node_broker_client_fixture_publishes_client_event_and_observes_replay() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..4 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_node_events");
                    assert_eq!(body["client_id"], "node-events");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_events",
                                "attachment": {
                                    "client_id": "node-events",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_events/client_event"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse client-event body")?;
                    assert_eq!(body["client_id"], "node-events");
                    assert_eq!(body["lease_seconds"], 45);
                    assert_eq!(body["event"], "selection.changed");
                    assert_eq!(body["data"]["selection"], "services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session_id": "sess_node_events",
                            "client_id": "node-events",
                            "event": "selection.changed",
                            "data": {
                                "selection": "services"
                            },
                            "operation": {
                                "kind": "client.event"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_events/events");
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 41\n",
                        "event: client.event\n",
                        "data: {\"session_id\":\"sess_node_events\",\"client_id\":\"node-events\",\"event\":\"selection.changed\",\"data\":{\"selection\":\"services\"}}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write node client-event stream")?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_events/events");
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("41")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 42\n",
                        "event: client.event\n",
                        "data: {\"session_id\":\"sess_node_events\",\"client_id\":\"node-events\",\"event\":\"selection.confirmed\",\"data\":{\"selection\":\"services\"}}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed node client-event stream")?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let create_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-events",
        "--lease-seconds",
        "45",
        "session-create",
        "--thread-id",
        "thread_node_events",
    ])?;
    let create_json: Value =
        serde_json::from_str(&create_output).context("parse node create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_node_events"
    );

    let publish_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-events",
        "--lease-seconds",
        "45",
        "client-event",
        "--session-id",
        "sess_node_events",
        "--event",
        "selection.changed",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let publish_json: Value =
        serde_json::from_str(&publish_output).context("parse node client-event output")?;
    assert_eq!(publish_json["status"], 200);
    assert_eq!(publish_json["body"]["operation"]["kind"], "client.event");
    assert_eq!(publish_json["body"]["data"]["selection"], "services");

    let events_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_node_events",
        "--limit",
        "1",
    ])?;
    let events_json: Value =
        serde_json::from_str(&events_output).context("parse node events output")?;
    assert_eq!(events_json["status"], 200);
    assert_eq!(events_json["body"][0]["event"], "client.event");
    assert_eq!(
        events_json["body"][0]["data"]["data"]["event"],
        "selection.changed"
    );
    assert_eq!(
        events_json["body"][0]["data"]["data"]["data"]["selection"],
        "services"
    );

    let resumed_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_node_events",
        "--last-event-id",
        "41",
        "--limit",
        "1",
    ])?;
    let resumed_json: Value =
        serde_json::from_str(&resumed_output).context("parse resumed node events output")?;
    assert_eq!(
        resumed_json["status"], 200,
        "resumed node events output: {resumed_output}"
    );
    assert_eq!(resumed_json["body"][0]["event"], "client.event");
    assert_eq!(
        resumed_json["body"][0]["data"]["data"]["event"],
        "selection.confirmed"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn node_broker_client_fixture_observes_supervision_and_backpressure_status_updates() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..3 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_node_status");
                    assert_eq!(body["client_id"], "node-status");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_status",
                                "attachment": {
                                    "client_id": "node-status",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_status/events");
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        "id: 61\n",
                        "event: status.updated\n",
                        "data: {\"session\":{\"supervision_notice\":{\"classification\":\"tool_slow\",\"recommended_action\":\"observe_or_interrupt\",\"recovery_policy\":{\"kind\":\"warn_only\",\"automation_ready\":false},\"recovery_options\":[{\"kind\":\"observe_status\"},{\"kind\":\"interrupt_turn\"}],\"request_id\":\"7\",\"thread_name\":\"codexw-bgtool-background_shell_start-7\",\"owner\":\"wrapper_background_shell\",\"source_call_id\":\"call_7\",\"target_background_shell_reference\":\"dev.api\",\"target_background_shell_job_id\":\"bg-1\",\"observation_state\":\"recent_output_observed\",\"output_state\":\"recent_output_observed\",\"observed_background_shell_job\":{\"job_id\":\"bg-1\",\"status\":\"running\",\"command\":\"npm run dev\"}},\"async_tool_backpressure\":{\"abandoned_request_count\":1,\"saturation_threshold\":2,\"saturated\":false,\"recommended_action\":\"observe_or_interrupt\",\"recovery_policy\":{\"kind\":\"warn_only\",\"automation_ready\":false},\"recovery_options\":[{\"kind\":\"observe_status\"},{\"kind\":\"interrupt_turn\"},{\"kind\":\"exit_and_resume\"}],\"oldest_request_id\":\"8\",\"oldest_thread_name\":\"codexw-bgtool-background_shell_start-8\",\"oldest_source_call_id\":\"call_8\",\"oldest_target_background_shell_reference\":\"dev.api\",\"oldest_target_background_shell_job_id\":\"bg-1\",\"oldest_observation_state\":\"recent_output_observed\",\"oldest_output_state\":\"recent_output_observed\",\"oldest_observed_background_shell_job\":{\"job_id\":\"bg-1\",\"status\":\"running\",\"command\":\"npm run dev\"}}}}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write node status event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_status/events");
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("61")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        "id: 62\n",
                        "event: status.updated\n",
                        "data: {\"session\":{\"supervision_notice\":{\"classification\":\"tool_wedged\",\"recommended_action\":\"interrupt_or_exit_resume\",\"recovery_policy\":{\"kind\":\"operator_interrupt_or_exit_resume\",\"automation_ready\":false},\"recovery_options\":[{\"kind\":\"observe_status\"},{\"kind\":\"interrupt_turn\"},{\"kind\":\"exit_and_resume\"}],\"request_id\":\"7\",\"thread_name\":\"codexw-bgtool-background_shell_start-7\",\"owner\":\"wrapper_background_shell\",\"source_call_id\":\"call_7\",\"target_background_shell_reference\":\"dev.api\",\"target_background_shell_job_id\":\"bg-1\",\"observation_state\":\"stale_output_observed\",\"output_state\":\"stale_output_observed\",\"observed_background_shell_job\":{\"job_id\":\"bg-1\",\"status\":\"running\",\"command\":\"npm run dev\"}},\"async_tool_backpressure\":{\"abandoned_request_count\":2,\"saturation_threshold\":2,\"saturated\":true,\"recommended_action\":\"interrupt_or_exit_resume\",\"recovery_policy\":{\"kind\":\"operator_interrupt_or_exit_resume\",\"automation_ready\":false},\"recovery_options\":[{\"kind\":\"observe_status\"},{\"kind\":\"interrupt_turn\"},{\"kind\":\"exit_and_resume\"}],\"oldest_request_id\":\"8\",\"oldest_thread_name\":\"codexw-bgtool-background_shell_start-8\",\"oldest_source_call_id\":\"call_8\",\"oldest_target_background_shell_reference\":\"dev.api\",\"oldest_target_background_shell_job_id\":\"bg-1\",\"oldest_observation_state\":\"stale_output_observed\",\"oldest_output_state\":\"stale_output_observed\",\"oldest_observed_background_shell_job\":{\"job_id\":\"bg-1\",\"status\":\"running\",\"command\":\"npm run dev\"}}}}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed node status stream")?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let create_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-status",
        "--lease-seconds",
        "45",
        "session-create",
        "--thread-id",
        "thread_node_status",
    ])?;
    let create_json: Value =
        serde_json::from_str(&create_output).context("parse node status create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_node_status"
    );

    let events_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_node_status",
        "--limit",
        "1",
    ])?;
    let events_json: Value =
        serde_json::from_str(&events_output).context("parse node status events output")?;
    assert_eq!(events_json["status"], 200);
    assert_eq!(events_json["body"][0]["event"], "status.updated");
    assert_eq!(
        events_json["body"][0]["data"]["data"]["session"]["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events_json["body"][0]["data"]["data"]["session"]["supervision_notice"]["recovery_policy"]
            ["kind"],
        "warn_only"
    );
    assert_eq!(
        events_json["body"][0]["data"]["data"]["session"]["async_tool_backpressure"]["oldest_request_id"],
        "8"
    );
    assert_eq!(
        events_json["body"][0]["data"]["data"]["session"]["async_tool_backpressure"]["recovery_options"]
            [2]["kind"],
        "exit_and_resume"
    );

    let resumed_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_node_status",
        "--last-event-id",
        "61",
        "--limit",
        "1",
    ])?;
    let resumed_json: Value =
        serde_json::from_str(&resumed_output).context("parse resumed node status output")?;
    assert_eq!(resumed_json["status"], 200);
    assert_eq!(resumed_json["body"][0]["event"], "status.updated");
    assert_eq!(
        resumed_json["body"][0]["data"]["data"]["session"]["supervision_notice"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        resumed_json["body"][0]["data"]["data"]["session"]["async_tool_backpressure"]["saturated"],
        true
    );
    assert_eq!(
        resumed_json["body"][0]["data"]["data"]["session"]["async_tool_backpressure"]["recovery_policy"]
            ["kind"],
        "operator_interrupt_or_exit_resume"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
