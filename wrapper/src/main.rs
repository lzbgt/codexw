mod app;
mod app_input;
mod catalog;
mod catalog_app_views;
mod catalog_backend_views;
mod catalog_connector_views;
mod catalog_feature_views;
mod catalog_lists;
mod catalog_threads;
mod collaboration;
mod collaboration_actions;
mod collaboration_preset;
mod commands;
mod commands_catalog;
mod commands_completion;
mod commands_entry_session_catalog;
mod commands_entry_session_modes;
mod commands_match;
mod commands_metadata;
mod dispatch_command_session_catalog;
mod dispatch_command_session_catalog_lists;
mod dispatch_command_session_catalog_models;
mod dispatch_command_session_meta;
mod dispatch_command_session_modes;
mod dispatch_command_session_status;
mod dispatch_command_thread_actions;
mod dispatch_command_thread_common;
mod dispatch_command_thread_draft;
mod dispatch_command_thread_navigation;
mod dispatch_command_thread_view;
mod dispatch_command_thread_workspace;
mod dispatch_command_utils;
mod dispatch_commands;
mod dispatch_submit;
mod dispatch_submit_commands;
mod dispatch_submit_turns;
mod editor;
mod editor_graphemes;
mod events;
mod history;
mod history_render;
mod history_state;
mod history_text;
mod input;
mod model_catalog;
mod model_personality;
mod model_session;
mod notifications;
mod output;
mod policy;
mod prompt;
mod prompt_completion;
mod prompt_file_completions_search;
mod prompt_file_completions_token;
mod prompt_state;
mod render;
mod render_ansi;
mod render_block_common;
mod render_block_markdown;
mod render_block_structured;
mod render_blocks;
mod render_markdown_block_structures;
mod render_markdown_code;
mod render_markdown_inline;
mod render_markdown_links;
mod render_markdown_styles;
mod render_prompt;
mod render_prompt_commit;
mod render_prompt_layout;
mod requests;
mod response_local_command;
mod response_realtime_activity;
mod response_thread_loaded;
mod response_thread_switch;
mod response_turn_activity;
mod responses;
mod rpc;
mod runtime_input;
mod runtime_process;
mod session_prompt_status;
mod session_realtime;
mod session_snapshot;
mod state;
mod state_core;
mod state_helpers;
mod status_account;
mod status_config;
mod status_limits;
mod status_rate_limits;
mod status_token_usage;
mod status_views;
mod transcript_completion_render;
mod transcript_plan_render;
mod transcript_render;
mod transcript_summary;

#[cfg(test)]
mod editor_tests;
#[cfg(test)]
mod input_test_build;
#[cfg(test)]
mod input_test_mentions;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod main_tests;

use anyhow::Result;
use clap::ArgAction;
use clap::Parser;
use runtime_process::normalize_cli;

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

fn main() -> Result<()> {
    app::run(normalize_cli(Cli::parse()))
}
