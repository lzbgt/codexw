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
