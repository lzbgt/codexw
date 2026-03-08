mod app;
mod app_input_controls;
mod app_input_editing;
mod app_input_editor;
mod app_input_interrupt;
mod catalog;
mod catalog_backend_views;
mod catalog_connector_views;
mod catalog_feature_views;
mod catalog_file_search;
mod catalog_thread_list;
mod client_dynamic_tools;
mod collaboration_apply;
mod collaboration_preset;
mod collaboration_view;
mod commands_catalog;
mod commands_completion_apply;
mod commands_completion_render;
mod commands_entry_session_catalog;
mod commands_entry_session_modes;
mod commands_match;
mod config_persistence;
mod dispatch_command_session_catalog_lists;
mod dispatch_command_session_catalog_models;
mod dispatch_command_session_collab;
mod dispatch_command_session_meta;
mod dispatch_command_session_ps;
mod dispatch_command_session_realtime;
mod dispatch_command_session_status;
mod dispatch_command_thread_common;
mod dispatch_command_thread_control;
mod dispatch_command_thread_draft;
mod dispatch_command_thread_navigation_identity;
mod dispatch_command_thread_navigation_session;
mod dispatch_command_thread_review;
mod dispatch_command_thread_view;
mod dispatch_command_utils;
mod dispatch_commands;
mod dispatch_submit_commands;
mod dispatch_submit_turns;
mod editor;
mod editor_graphemes;
mod event_request_approvals;
mod event_request_tools;
mod events;
mod history_render;
mod history_state;
mod history_text;
mod input;
mod model_catalog;
mod model_personality_actions;
mod model_personality_view;
mod notification_item_buffers;
mod notification_item_completion;
mod notification_item_status;
mod notification_turn_completed;
mod notification_turn_started;
mod output;
mod policy;
mod prompt;
mod prompt_file_completions_search;
mod prompt_file_completions_token;
mod prompt_state;
mod render_ansi;
mod render_block_common;
mod render_block_markdown;
mod render_block_structured;
mod render_markdown_block_structures;
mod render_markdown_code;
mod render_markdown_inline;
mod render_markdown_links;
mod render_markdown_styles;
mod render_prompt;
mod requests;
mod response_bootstrap_catalog_state;
mod response_bootstrap_catalog_views;
mod response_bootstrap_init;
mod response_error_runtime;
mod response_error_session;
mod response_local_command;
mod response_realtime_activity;
mod response_thread_loaded;
mod response_thread_maintenance;
mod response_thread_runtime;
mod response_turn_activity;
mod rpc;
mod runtime_event_sources;
mod runtime_keys;
mod runtime_process;
mod selection_flow;
mod session_prompt_status_active;
mod session_prompt_status_ready;
mod session_realtime_item;
mod session_realtime_status;
mod session_snapshot_overview;
mod session_snapshot_runtime;
mod state;
mod state_helpers;
mod status_account;
mod status_config;
mod status_rate_credits;
mod status_rate_windows;
mod status_token_usage;
mod status_value;
mod transcript_approval_summary;
mod transcript_completion_render;
mod transcript_item_summary;
mod transcript_plan_render;
mod transcript_status_summary;

#[cfg(test)]
mod editor_tests;
#[cfg(test)]
mod input_test_build_items;
#[cfg(test)]
mod input_test_build_mentions;
#[cfg(test)]
mod input_test_mentions;
#[cfg(test)]
mod main_test_approvals;
#[cfg(test)]
mod main_test_catalog;
#[cfg(test)]
mod main_test_catalog_render;
#[cfg(test)]
mod main_test_catalog_threads;
#[cfg(test)]
mod main_test_commands;
#[cfg(test)]
mod main_test_runtime_cli;
#[cfg(test)]
mod main_test_runtime_commands;
#[cfg(test)]
mod main_test_runtime_prompt;
#[cfg(test)]
mod main_test_session_collaboration;
#[cfg(test)]
mod main_test_session_model_catalog;
#[cfg(test)]
mod main_test_session_personality_status;
#[cfg(test)]
mod main_test_session_realtime;
#[cfg(test)]
mod main_test_session_render;
#[cfg(test)]
mod main_test_session_selections;
#[cfg(test)]
mod main_test_session_status;
#[cfg(test)]
mod render_tests;

use anyhow::Result;
use clap::ArgAction;
use clap::Parser;
use runtime_process::normalize_cli;
use std::ffi::OsString;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Codex app-server inline terminal client with auto-continue"
)]
struct Cli {
    #[arg(long, default_value = "codex")]
    codex_bin: String,

    #[arg(short = 'c', long = "config", value_name = "key=value", action = ArgAction::Append)]
    config_overrides: Vec<String>,

    #[arg(long = "enable", value_name = "FEATURE", action = ArgAction::Append)]
    enable_features: Vec<String>,

    #[arg(long = "disable", value_name = "FEATURE", action = ArgAction::Append)]
    disable_features: Vec<String>,

    #[arg(long)]
    resume: Option<String>,

    #[arg(skip = false)]
    resume_picker: bool,

    #[arg(long)]
    cwd: Option<String>,

    #[arg(long)]
    model: Option<String>,

    #[arg(long)]
    model_provider: Option<String>,

    #[arg(long, default_value_t = true)]
    auto_continue: bool,

    #[arg(long, default_value_t = false)]
    verbose_events: bool,

    #[arg(long, default_value_t = true)]
    verbose_thinking: bool,

    #[arg(long, default_value_t = false)]
    raw_json: bool,

    #[arg(long, default_value_t = false)]
    no_experimental_api: bool,

    #[arg(long, default_value_t = false)]
    yolo: bool,

    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

fn parse_cli_from<I, T>(args: I) -> std::result::Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args = rewrite_resume_startup_args(args.into_iter().map(Into::into).collect());
    Cli::try_parse_from(args).map(normalize_cli)
}

fn parse_cli() -> Cli {
    parse_cli_from(std::env::args_os()).unwrap_or_else(|err| err.exit())
}

fn rewrite_resume_startup_args(args: Vec<OsString>) -> Vec<OsString> {
    if args.len() < 3 {
        return args;
    }

    let mut pre_resume = vec![args[0].clone()];
    let mut index = 1;
    while index < args.len() {
        let token = args[index].to_string_lossy();
        if token == "--" {
            return args;
        }
        if token == "resume" {
            break;
        }
        if let Some(option_arity) = option_arity(token.as_ref()) {
            pre_resume.push(args[index].clone());
            index += 1;
            if option_arity == OptionArity::TakesValue {
                if index >= args.len() {
                    return args;
                }
                pre_resume.push(args[index].clone());
                index += 1;
            }
            continue;
        }
        return args;
    }

    if index >= args.len() || args[index].to_string_lossy() != "resume" {
        return args;
    }

    let resume_token = args[index].clone();
    index += 1;

    let mut migrated_options = Vec::new();
    let mut resume_prompt = Vec::new();
    while index < args.len() {
        let token = args[index].to_string_lossy();
        if token == "--" {
            resume_prompt.extend(args[index..].iter().cloned());
            break;
        }
        if let Some(option_arity) = option_arity(token.as_ref()) {
            migrated_options.push(args[index].clone());
            index += 1;
            if option_arity == OptionArity::TakesValue {
                if index >= args.len() {
                    return args;
                }
                migrated_options.push(args[index].clone());
                index += 1;
            }
            continue;
        }
        resume_prompt.push(args[index].clone());
        index += 1;
    }

    let mut rewritten = pre_resume;
    rewritten.extend(migrated_options);
    rewritten.push(resume_token);
    rewritten.extend(resume_prompt);
    rewritten
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum OptionArity {
    FlagOnly,
    TakesValue,
}

fn option_arity(token: &str) -> Option<OptionArity> {
    if matches!(
        token,
        "--auto-continue"
            | "--verbose-events"
            | "--verbose-thinking"
            | "--raw-json"
            | "--no-experimental-api"
            | "--yolo"
    ) {
        return Some(OptionArity::FlagOnly);
    }
    if matches!(
        token,
        "--codex-bin"
            | "--config"
            | "-c"
            | "--enable"
            | "--disable"
            | "--resume"
            | "--cwd"
            | "--model"
            | "--model-provider"
    ) {
        return Some(OptionArity::TakesValue);
    }
    if let Some((name, _)) = token.split_once('=')
        && option_arity(name) == Some(OptionArity::TakesValue)
    {
        return Some(OptionArity::FlagOnly);
    }
    None
}

fn main() -> Result<()> {
    app::run(parse_cli())
}
