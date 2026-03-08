mod bootstrap_account;
mod bootstrap_catalog_core;
mod bootstrap_catalog_lists;
mod bootstrap_init;
mod bootstrap_search;
mod command_requests;
mod request_types;
mod thread_maintenance;
mod thread_realtime;
mod thread_review;
mod thread_switch_common;
mod turn_control;
mod turn_start;

use std::io::Write;
use std::process::ChildStdin;

use anyhow::Context;
use anyhow::Result;

pub(crate) use bootstrap_account::send_feedback_upload;
pub(crate) use bootstrap_account::send_load_account;
pub(crate) use bootstrap_account::send_load_rate_limits;
pub(crate) use bootstrap_account::send_logout_account;
pub(crate) use bootstrap_catalog_core::send_load_collaboration_modes;
pub(crate) use bootstrap_catalog_core::send_load_models;
pub(crate) use bootstrap_catalog_lists::send_load_apps;
pub(crate) use bootstrap_catalog_lists::send_load_config;
pub(crate) use bootstrap_catalog_lists::send_load_experimental_features;
pub(crate) use bootstrap_catalog_lists::send_load_mcp_servers;
pub(crate) use bootstrap_catalog_lists::send_load_skills;
pub(crate) use bootstrap_catalog_lists::send_windows_sandbox_setup_start;
pub(crate) use bootstrap_init::send_initialize;
pub(crate) use bootstrap_init::send_initialized;
pub(crate) use bootstrap_search::send_fuzzy_file_search;
pub(crate) use bootstrap_search::send_list_agent_threads;
pub(crate) use bootstrap_search::send_list_threads;
pub(crate) use bootstrap_search::send_list_threads_with_view;
#[cfg(test)]
pub(crate) use bootstrap_search::thread_list_params;
pub(crate) use command_requests::send_command_exec;
pub(crate) use command_requests::send_command_exec_terminate;
pub(crate) use request_types::PendingRequest;
pub(crate) use request_types::ThreadListView;
pub(crate) use thread_maintenance::send_clean_background_terminals;
pub(crate) use thread_maintenance::send_thread_compact;
pub(crate) use thread_maintenance::send_thread_rename;
pub(crate) use thread_realtime::send_thread_realtime_append_text;
pub(crate) use thread_realtime::send_thread_realtime_start;
pub(crate) use thread_realtime::send_thread_realtime_stop;
pub(crate) use thread_review::send_start_review;
pub(crate) use thread_switch_common::send_thread_fork;
pub(crate) use thread_switch_common::send_thread_resume;
pub(crate) use thread_switch_common::send_thread_start;
pub(crate) use turn_control::send_turn_interrupt;
pub(crate) use turn_start::send_turn_start;
pub(crate) use turn_start::send_turn_steer;

pub(crate) fn send_json<T: serde::Serialize>(writer: &mut ChildStdin, value: &T) -> Result<()> {
    let mut encoded = serde_json::to_string(value).context("serialize JSON-RPC message")?;
    encoded.push('\n');
    writer
        .write_all(encoded.as_bytes())
        .context("write JSON-RPC message")?;
    writer.flush().context("flush JSON-RPC message")
}
