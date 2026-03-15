#[path = "dispatch/gate.rs"]
mod gate;
#[path = "dispatch/proxy.rs"]
mod proxy;

use std::net::TcpStream;

use anyhow::Result;

use super::Cli;
use super::http;

pub(super) fn handle_connection(stream: &mut TcpStream, cli: &Cli) -> Result<()> {
    match gate::prepare_request(stream, cli)? {
        gate::ConnectionAction::Respond(response) => http::write_response(stream, &response)?,
        gate::ConnectionAction::Proxy { request, target } => {
            proxy::handle_proxy(stream, cli, &request, &target)?
        }
    }
    Ok(())
}
