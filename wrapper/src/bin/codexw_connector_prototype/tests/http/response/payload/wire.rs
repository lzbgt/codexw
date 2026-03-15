use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use crate::http::HttpResponse;
use crate::http::write_response;

fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = TcpStream::connect(addr).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    (client, server)
}

#[test]
fn write_response_serializes_status_headers_and_body() {
    let (mut client, mut server) = connected_pair();
    let response = HttpResponse {
        status: 202,
        reason: "Accepted",
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: br#"{"ok":true}"#.to_vec(),
    };
    let handle = thread::spawn(move || write_response(&mut server, &response).expect("write"));

    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).expect("read response");
    handle.join().expect("server thread");
    let text = String::from_utf8(bytes).expect("utf8");

    assert!(text.starts_with("HTTP/1.1 202 Accepted\r\n"));
    assert!(text.contains("Content-Length: 11\r\n"));
    assert!(text.contains("Content-Type: application/json\r\n"));
    assert!(text.ends_with("\r\n\r\n{\"ok\":true}"));
}
