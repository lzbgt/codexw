#[path = "sse/framing.rs"]
mod framing;
#[path = "sse/proxy.rs"]
mod proxy;

pub(super) fn handle_sse_proxy(
    client_stream: std::net::TcpStream,
    request: &super::http::HttpRequest,
    cli: &super::Cli,
    target: &super::routing::ProxyTarget,
) -> anyhow::Result<()> {
    proxy::handle_sse_proxy(client_stream, request, cli, target)
}

#[cfg(test)]
pub(super) fn wrap_event_payload(
    data_lines: Vec<String>,
    agent_id: &str,
    deployment_id: &str,
) -> String {
    framing::wrap_event_payload(data_lines, agent_id, deployment_id)
}

#[cfg(test)]
pub(super) fn complete_sse_lines(text: &str, pending_line_fragment: &mut String) -> Vec<String> {
    framing::complete_sse_lines(text, pending_line_fragment)
}

#[cfg(test)]
pub(super) fn connector_sse_response_head(
    cli: &super::Cli,
    local_api_version: Option<&str>,
) -> String {
    proxy::connector_sse_response_head(cli, local_api_version)
}
