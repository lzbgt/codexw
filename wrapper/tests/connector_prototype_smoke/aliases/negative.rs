use super::*;

#[test]
fn connector_returns_not_found_for_unknown_broker_alias_route() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/orchestration/plans HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(response.contains("\"code\":\"not_found\""));
    assert!(response.contains("unknown connector route"));

    Ok(())
}

#[test]
fn connector_returns_not_found_for_out_of_scope_broker_scene_routes() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let scene_get = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/scene HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(scene_get.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(scene_get.contains("\"code\":\"not_found\""));
    assert!(scene_get.contains("unknown connector route"));

    let body = "{\"ops\":[]}";
    let scene_apply = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/scene/apply HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let scene_apply_response = send_raw_request(connector_port, &scene_apply)?;
    assert!(scene_apply_response.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(scene_apply_response.contains("\"code\":\"not_found\""));
    assert!(scene_apply_response.contains("unknown connector route"));

    Ok(())
}

#[test]
fn connector_returns_not_found_for_out_of_scope_global_broker_route() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/events HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(response.contains("\"code\":\"not_found\""));
    assert!(response.contains("unknown connector route"));

    Ok(())
}

#[test]
fn connector_rejects_raw_proxy_route_outside_allowed_surface() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/proxy/api/v1/session/sess_1/scene HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 403 Error\r\n"));
    assert!(response.contains("\"code\":\"route_not_allowed\""));
    assert!(
        response
            .contains("\"message\":\"connector route is outside the allowed local API surface\"")
    );
    assert!(response.contains("\"local_path\":\"/api/v1/session/sess_1/scene\""));
    assert!(response.contains("\"is_sse\":false"));

    Ok(())
}

#[test]
fn connector_rejects_raw_proxy_sse_route_outside_allowed_surface() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/proxy_sse/api/v1/session/sess_1/transcript HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 403 Error\r\n"));
    assert!(response.contains("\"code\":\"route_not_allowed\""));
    assert!(response.contains("\"local_path\":\"/api/v1/session/sess_1/transcript\""));
    assert!(response.contains("\"is_sse\":true"));

    Ok(())
}
