use crate::Cli;
use crate::sse::connector_sse_response_head;

fn sample_cli() -> Cli {
    Cli {
        bind: "127.0.0.1:0".to_string(),
        local_api_base: "http://127.0.0.1:8080".to_string(),
        agent_id: "codexw-lab".to_string(),
        deployment_id: "mac-mini-01".to_string(),
        connector_token: Some("connector-secret".to_string()),
        local_api_token: Some("local-api-secret".to_string()),
    }
}

#[test]
fn connector_sse_response_head_includes_adapter_and_identity_headers() {
    let head = connector_sse_response_head(&sample_cli(), None);

    assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(head.contains("Content-Type: text/event-stream\r\n"));
    assert!(head.contains("Cache-Control: no-cache\r\n"));
    assert!(head.contains("Connection: close\r\n"));
    assert!(head.contains("X-Codexw-Agent-Id: codexw-lab\r\n"));
    assert!(head.contains("X-Codexw-Deployment-Id: mac-mini-01\r\n"));
    assert!(head.contains("X-Codexw-Broker-Adapter-Version: "));
    assert!(head.ends_with("\r\n\r\n"));
}

#[test]
fn connector_sse_response_head_projects_local_api_version_when_present() {
    let head = connector_sse_response_head(&sample_cli(), Some("2026-03-15"));

    assert!(head.contains("X-Codexw-Local-Api-Version: 2026-03-15\r\n"));
}

#[test]
fn connector_sse_response_head_omits_local_api_version_when_absent() {
    let head = connector_sse_response_head(&sample_cli(), None);

    assert!(!head.contains("X-Codexw-Local-Api-Version:"));
}
