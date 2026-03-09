use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub(crate) use anyhow::Context;
pub(crate) use anyhow::Result;
pub(crate) use serde_json::Value;
pub(crate) use serde_json::json;
pub(crate) use std::net::TcpListener;
pub(crate) use std::thread;

pub(crate) const READ_TIMEOUT: Duration = Duration::from_secs(5);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct ChildGuard {
    child: Child,
    stderr_path: PathBuf,
    _serial_guard: MutexGuard<'static, ()>,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.stderr_path);
    }
}

#[derive(Debug)]
pub(crate) struct ParsedRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) _headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

pub(crate) struct BrokerClient {
    port: u16,
    agent_id: &'static str,
}

impl BrokerClient {
    pub(crate) fn new(port: u16, agent_id: &'static str) -> Self {
        Self { port, agent_id }
    }

    pub(crate) fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        let mut request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n");
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        match body {
            Some(body) => {
                request.push_str(&format!(
                    "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                ));
            }
            None => request.push_str("Connection: close\r\n\r\n"),
        }
        send_raw_request(self.port, &request)
    }

    pub(crate) fn session_request(
        &self,
        method: &str,
        session_id: &str,
        suffix: &str,
        body: Option<&str>,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        self.request(
            method,
            &format!("/v1/agents/{}/sessions/{session_id}{suffix}", self.agent_id),
            body,
            headers,
        )
    }

    pub(crate) fn create_session(&self, body: &str, headers: &[(&str, &str)]) -> Result<String> {
        self.request(
            "POST",
            &format!("/v1/agents/{}/sessions", self.agent_id),
            Some(body),
            headers,
        )
    }
}

pub(crate) fn spawn_connector(port: u16, local_api_port: u16) -> Result<ChildGuard> {
    let serial_guard = smoke_test_lock();
    let binary = connector_binary()?;
    let stderr_path = connector_stderr_path(port);
    let stderr_file = File::create(&stderr_path).context("create connector stderr log")?;
    let child = Command::new(binary)
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        .arg("--local-api-base")
        .arg(format!("http://127.0.0.1:{local_api_port}"))
        .arg("--agent-id")
        .arg("codexw-lab")
        .arg("--deployment-id")
        .arg("mac-mini-01")
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .context("spawn connector prototype")?;
    Ok(ChildGuard {
        child,
        stderr_path,
        _serial_guard: serial_guard,
    })
}

fn connector_binary() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_codexw-connector-prototype") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe().context("resolve current test executable")?;
    let debug_dir = current_exe
        .parent()
        .and_then(|path| path.parent())
        .context("resolve cargo target debug directory")?;
    let mut fallback = debug_dir.join("codexw-connector-prototype");
    if cfg!(windows) {
        fallback.set_extension("exe");
    }
    if fallback.exists() {
        return Ok(fallback);
    }

    anyhow::bail!("resolve connector prototype test binary")
}

pub(crate) fn reserve_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind ephemeral port")?;
    let port = listener.local_addr().context("ephemeral addr")?.port();
    drop(listener);
    Ok(port)
}

pub(crate) fn wait_for_healthz(connector: &mut ChildGuard, port: u16) -> Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if Instant::now() > deadline {
            anyhow::bail!(
                "connector healthz did not become reachable; stderr:\n{}",
                connector_stderr(connector)
            );
        }
        if let Some(status) = connector
            .child
            .try_wait()
            .context("poll connector prototype process")?
        {
            anyhow::bail!(
                "connector prototype exited before healthz with status {status}; stderr:\n{}",
                connector_stderr(connector)
            );
        }
        match send_raw_request(
            port,
            concat!(
                "GET /healthz HTTP/1.1\r\n",
                "Host: localhost\r\n",
                "Connection: close\r\n",
                "\r\n"
            ),
        ) {
            Ok(response) if response.starts_with("HTTP/1.1 200 OK\r\n") => return Ok(()),
            _ => thread::sleep(POLL_INTERVAL),
        }
    }
}

fn smoke_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("connector smoke lock poisoned")
}

fn connector_stderr_path(port: u16) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!(
        "codexw-connector-smoke-{}-{}-{}.log",
        std::process::id(),
        port,
        millis
    ))
}

fn connector_stderr(connector: &ChildGuard) -> String {
    fs::read_to_string(&connector.stderr_path)
        .map(|text| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                "<empty>".to_string()
            } else {
                trimmed.to_string()
            }
        })
        .unwrap_or_else(|_| "<unavailable>".to_string())
}

pub(crate) fn send_raw_request(port: u16, request: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("connect to 127.0.0.1:{port}"))?;
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set client read timeout")?;
    stream
        .write_all(request.as_bytes())
        .context("write raw request")?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .context("read raw response")?;
    String::from_utf8(bytes).context("decode raw response")
}

fn broker_client_script() -> Result<PathBuf> {
    let current_exe = std::env::current_exe().context("resolve current test executable")?;
    for candidate in current_exe.ancestors() {
        let script = candidate.join("scripts").join("codexw_broker_client.py");
        if script.exists() {
            return Ok(script);
        }
    }
    anyhow::bail!(
        "resolve broker client fixture script from test executable {}",
        current_exe.display()
    )
}

pub(crate) fn run_broker_client(args: &[&str]) -> Result<String> {
    let script = broker_client_script()?;
    let output = Command::new("python3")
        .arg(script)
        .args(args)
        .output()
        .context("run broker client fixture")?;
    if !output.status.success() {
        anyhow::bail!(
            "broker client fixture failed with status {}:\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).context("decode broker client fixture stdout")
}

pub(crate) fn read_http_request(stream: &mut TcpStream) -> Result<ParsedRequest> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set fake local api read timeout")?;
    let mut buffer = [0_u8; 1024];
    let mut request_bytes = Vec::new();
    let header_end = loop {
        let read = stream
            .read(&mut buffer)
            .context("read fake local api request")?;
        if read == 0 {
            anyhow::bail!("request closed before headers");
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            break index + 4;
        }
    };
    let request_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = request_text.split("\r\n");
    let request_line = lines.next().context("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().context("missing request method")?.to_string();
    let path = parts.next().context("missing request path")?.to_string();
    let _version = parts.next().context("missing request version")?;

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut buffer).context("read request body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    body.truncate(content_length);

    Ok(ParsedRequest {
        method,
        path,
        _headers: headers,
        body,
    })
}

pub(crate) fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<()> {
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    stream
        .write_all(response.as_bytes())
        .context("write fake local api head")?;
    stream
        .write_all(body)
        .context("write fake local api body")?;
    Ok(())
}

pub(crate) fn json_bytes(value: Value) -> Result<Vec<u8>> {
    serde_json::to_vec(&value).context("encode json body")
}
