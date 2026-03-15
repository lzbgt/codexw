use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use crate::http::read_request;

fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = TcpStream::connect(addr).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    (client, server)
}

#[test]
fn read_request_parses_method_path_headers_and_body() {
    let (mut client, mut server) = connected_pair();
    let handle = thread::spawn(move || {
        client
            .write_all(
                b"POST /v1/agents/codexw-lab/proxy/api/v1/session/new HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 17\r\n\r\n{\"ok\":true,\"x\":1}",
            )
            .expect("write request");
    });

    let request = read_request(&mut server).expect("read request");
    handle.join().expect("client thread");

    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/v1/agents/codexw-lab/proxy/api/v1/session/new"
    );
    assert_eq!(
        request.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    assert_eq!(request.body, br#"{"ok":true,"x":1}"#);
}
