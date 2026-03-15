use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;

use super::Cli;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(super) fn run_connector_server(cli: &Cli) -> Result<()> {
    let listener = TcpListener::bind(&cli.bind)
        .with_context(|| format!("bind connector listener on `{}`", cli.bind))?;
    listener
        .set_nonblocking(true)
        .context("set connector listener nonblocking")?;
    let bind_addr = listener
        .local_addr()
        .context("read connector listener address")?;
    eprintln!("codexw connector prototype listening on http://{bind_addr}");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_signal = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_for_signal.store(true, Ordering::Relaxed);
    })
    .context("install ctrl-c handler")?;

    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let cli = cli.clone();
                thread::spawn(move || {
                    let _ = super::handle_connection(stream, &cli);
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(err) => return Err(err).context("accept connector connection"),
        }
    }
    Ok(())
}
