use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

use super::super::super::Cli;
use super::super::super::upstream::UpstreamResponse;
use super::super::HttpResponse;

pub(super) fn from_upstream_response(upstream: UpstreamResponse, cli: &Cli) -> HttpResponse {
    let mut headers = Vec::new();
    if let Some(content_type) = upstream.headers.get("content-type") {
        headers.push(("Content-Type".to_string(), content_type.clone()));
    } else {
        headers.push((
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ));
    }
    headers.push(("X-Codexw-Agent-Id".to_string(), cli.agent_id.clone()));
    headers.push((
        "X-Codexw-Deployment-Id".to_string(),
        cli.deployment_id.clone(),
    ));
    headers.push((
        HEADER_BROKER_ADAPTER_VERSION.to_string(),
        CODEXW_BROKER_ADAPTER_VERSION.to_string(),
    ));
    if let Some(local_api_version) = upstream.headers.get("x-codexw-local-api-version") {
        headers.push((
            HEADER_LOCAL_API_VERSION.to_string(),
            local_api_version.clone(),
        ));
    }
    HttpResponse {
        status: upstream.status,
        reason: Box::leak(upstream.reason.into_boxed_str()),
        headers,
        body: upstream.body,
    }
}
