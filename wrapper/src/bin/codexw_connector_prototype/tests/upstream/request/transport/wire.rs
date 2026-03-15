use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use crate::upstream::write_upstream_request;

fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = TcpStream::connect(addr).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    (client, server)
}

#[test]
fn write_upstream_request_serializes_headers_and_body() {
    let (mut client, mut server) = connected_pair();
    let handle = thread::spawn(move || {
        write_upstream_request(
            &mut server,
            "POST",
            "/api/v1/session/new",
            Some("application/json"),
            Some("secret"),
            br#"{"ok":true}"#,
            Some("evt_123"),
        )
        .expect("write request")
    });

    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).expect("read request");
    handle.join().expect("server thread");
    let text = String::from_utf8(bytes).expect("utf8");

    assert!(text.starts_with("POST /api/v1/session/new HTTP/1.1\r\n"));
    assert!(text.contains("Host: localhost\r\n"));
    assert!(text.contains("Connection: close\r\n"));
    assert!(text.contains("Content-Length: 11\r\n"));
    assert!(text.contains("Content-Type: application/json\r\n"));
    assert!(text.contains("Authorization: Bearer secret\r\n"));
    assert!(text.contains("Last-Event-ID: evt_123\r\n"));
    assert!(text.ends_with("\r\n\r\n{\"ok\":true}"));
}
