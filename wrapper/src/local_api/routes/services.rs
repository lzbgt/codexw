#[path = "services/interact.rs"]
mod interact;
#[path = "services/mutate.rs"]
mod mutate;
#[path = "services/read.rs"]
mod read;

use crate::background_shells::BackgroundShellManager;
use crate::local_api::SharedCommandQueue;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::LocalApiSnapshot;

pub(super) fn handle_services_route(snapshot: &LocalApiSnapshot) -> HttpResponse {
    read::handle_services_route(snapshot)
}

pub(super) fn handle_service_detail_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> HttpResponse {
    read::handle_service_detail_route(snapshot, reference)
}

pub(super) fn handle_capabilities_route(snapshot: &LocalApiSnapshot) -> HttpResponse {
    read::handle_capabilities_route(snapshot)
}

pub(super) fn handle_capability_detail_route(
    snapshot: &LocalApiSnapshot,
    capability: &str,
) -> HttpResponse {
    read::handle_capability_detail_route(snapshot, capability)
}

pub(super) fn handle_service_attach_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    interact::handle_service_attach_route(
        request,
        snapshot,
        background_shells,
        reference,
        session_id,
    )
}

pub(super) fn handle_service_wait_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    interact::handle_service_wait_route(request, snapshot, background_shells, reference, session_id)
}

pub(super) fn handle_service_run_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    interact::handle_service_run_route(request, snapshot, background_shells, reference, session_id)
}

pub(super) fn handle_service_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    mutate::handle_service_update_route(request, snapshot, command_queue, session_id)
}

pub(super) fn handle_dependency_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    mutate::handle_dependency_update_route(request, snapshot, command_queue, session_id)
}

pub(super) fn handle_service_provide_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    mutate::handle_service_provide_route(request, snapshot, command_queue, reference, session_id)
}

pub(super) fn handle_service_depend_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    mutate::handle_service_depend_route(request, snapshot, command_queue, reference, session_id)
}

pub(super) fn handle_service_contract_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    mutate::handle_service_contract_route(request, snapshot, command_queue, reference, session_id)
}

pub(super) fn handle_service_relabel_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    mutate::handle_service_relabel_route(request, snapshot, command_queue, reference, session_id)
}
