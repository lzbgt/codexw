#[path = "dispatch/gate.rs"]
mod gate;
#[path = "dispatch/proxy.rs"]
mod proxy;

use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use crate::Cli;
use crate::handle_connection;

pub(super) fn sample_cli() -> Cli {
    Cli {
        bind: "127.0.0.1:0".to_string(),
        local_api_base: "http://127.0.0.1:8080".to_string(),
        local_api_token: Some("secret".to_string()),
        connector_token: Some("connector".to_string()),
        agent_id: "codexw-lab".to_string(),
        deployment_id: "mac-mini-01".to_string(),
    }
}

fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = TcpStream::connect(addr).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    (client, server)
}

pub(super) fn run_connection(raw_request: &'static [u8], cli: &Cli) -> String {
    let (mut client, server) = connected_pair();
    let cli = cli.clone();
    let writer = thread::spawn(move || {
        client.write_all(raw_request).expect("write request");
        let _ = client.shutdown(std::net::Shutdown::Write);
        let mut bytes = Vec::new();
        client.read_to_end(&mut bytes).expect("read response");
        String::from_utf8(bytes).expect("utf8")
    });
    handle_connection(server, &cli).expect("handle connection");
    writer.join().expect("client thread")
}
