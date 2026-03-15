#[path = "shells/mutate.rs"]
mod mutate;
#[path = "shells/read.rs"]
mod read;

use crate::local_api::SharedCommandQueue;
use crate::local_api::server::HttpRequest;
use crate::local_api::snapshot::LocalApiSnapshot;

pub(super) fn handle_shells_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    read::handle_shells_route(snapshot)
}

pub(super) fn handle_shell_detail_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    read::handle_shell_detail_route(snapshot, reference)
}

pub(super) fn handle_shell_start_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    mutate::handle_shell_start_route(request, snapshot, command_queue)
}

pub(super) fn handle_shell_poll_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    read::handle_shell_poll_route(snapshot, reference)
}

pub(super) fn handle_shell_send_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    mutate::handle_shell_send_route(request, snapshot, command_queue, reference)
}

pub(super) fn handle_shell_terminate_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    mutate::handle_shell_terminate_route(request, snapshot, command_queue, reference)
}
