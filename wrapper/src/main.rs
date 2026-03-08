mod app;
mod app_input;
mod catalog;
mod catalog_lists;
mod catalog_threads;
mod catalog_views;
mod collaboration;
mod commands;
mod commands_catalog;
mod commands_completion;
mod commands_entries;
mod commands_match;
mod commands_metadata;
mod dispatch;
mod dispatch_command_session;
mod dispatch_command_thread;
mod dispatch_command_utils;
mod dispatch_commands;
mod dispatch_submit;
mod editor;
mod editor_graphemes;
mod events;
mod history;
mod input;
mod interaction;
mod model_session;
mod notifications;
mod output;
mod policy;
mod prompt;
mod prompting;
mod render;
mod render_ansi;
mod render_block_common;
mod render_block_markdown;
mod render_block_structured;
mod render_blocks;
mod render_markdown_code;
mod render_markdown_inline;
mod render_prompt;
mod requests;
mod responses;
mod rpc;
mod runtime;
mod session_prompt_status;
mod session_realtime;
mod session_snapshot;
mod session_status;
mod state;
mod state_core;
mod state_helpers;
mod status_account;
mod status_config;
mod status_limits;
mod status_views;
mod transcript_render;
mod transcript_summary;
mod transcript_views;

#[cfg(test)]
mod editor_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod main_tests;

use anyhow::Result;
use clap::ArgAction;
use clap::Parser;
use runtime::normalize_cli;

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
