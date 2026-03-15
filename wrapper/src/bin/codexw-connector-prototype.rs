use std::net::Shutdown;
use std::net::TcpStream;

use anyhow::Result;
use clap::Parser;

#[path = "../adapter_contract.rs"]
mod adapter_contract;
#[path = "codexw_connector_prototype/dispatch.rs"]
mod dispatch;
#[path = "codexw_connector_prototype/http.rs"]
mod http;
#[path = "../http_request_reader.rs"]
mod http_request_reader;
#[path = "codexw_connector_prototype/routing.rs"]
mod routing;
#[path = "codexw_connector_prototype/server.rs"]
mod server;
#[path = "codexw_connector_prototype/sse.rs"]
mod sse;
#[path = "codexw_connector_prototype/tests.rs"]
#[cfg(test)]
mod tests;
#[path = "codexw_connector_prototype/upstream.rs"]
mod upstream;

const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
const MAX_REQUEST_BYTES: usize = 65536;

#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "Prototype broker-facing connector for the codexw local API"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: String,

    #[arg(long, default_value = "http://127.0.0.1:8080")]
    local_api_base: String,

    #[arg(long)]
    local_api_token: Option<String>,

    #[arg(long)]
    connector_token: Option<String>,

    #[arg(long)]
    agent_id: String,

    #[arg(long)]
    deployment_id: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    server::run_connector_server(&cli)
}

fn handle_connection(mut stream: TcpStream, cli: &Cli) -> Result<()> {
    dispatch::handle_connection(&mut stream, cli)?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}
