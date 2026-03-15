#[path = "turn/interrupt.rs"]
mod interrupt;
#[path = "turn/start.rs"]
mod start;

use crate::local_api::SharedCommandQueue;
use crate::local_api::server::HttpRequest;
use crate::local_api::snapshot::LocalApiSnapshot;

pub(super) fn handle_turn_start_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    start::handle_turn_start_route(request, snapshot, command_queue)
}

pub(super) fn handle_turn_start_route_for_session(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    start::handle_turn_start_route_for_session(request, snapshot, command_queue)
}

pub(super) fn handle_turn_interrupt_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    interrupt::handle_turn_interrupt_route(request, snapshot, command_queue)
}

pub(super) fn handle_turn_interrupt_route_for_session(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    interrupt::handle_turn_interrupt_route_for_session(request, snapshot, command_queue)
}
