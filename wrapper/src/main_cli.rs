use clap::ArgAction;
use clap::Parser;
use std::ffi::OsString;

use crate::runtime_process::normalize_cli;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Codex app-server inline terminal client with auto-continue"
)]
pub(crate) struct Cli {
    #[arg(long, default_value = "codex")]
    pub(crate) codex_bin: String,

    #[arg(short = 'c', long = "config", value_name = "key=value", action = ArgAction::Append)]
    pub(crate) config_overrides: Vec<String>,

    #[arg(long = "enable", value_name = "FEATURE", action = ArgAction::Append)]
    pub(crate) enable_features: Vec<String>,

    #[arg(long = "disable", value_name = "FEATURE", action = ArgAction::Append)]
    pub(crate) disable_features: Vec<String>,

    #[arg(long)]
    pub(crate) resume: Option<String>,

    #[arg(skip = false)]
    pub(crate) resume_picker: bool,

    #[arg(long)]
    pub(crate) cwd: Option<String>,

    #[arg(long)]
    pub(crate) model: Option<String>,

    #[arg(long)]
    pub(crate) model_provider: Option<String>,

    #[arg(long, default_value_t = true)]
    pub(crate) auto_continue: bool,

    #[arg(long, default_value_t = false)]
    pub(crate) verbose_events: bool,

    #[arg(long, default_value_t = true)]
    pub(crate) verbose_thinking: bool,

    #[arg(long, default_value_t = false)]
    pub(crate) raw_json: bool,

    #[arg(long, default_value_t = false)]
    pub(crate) no_experimental_api: bool,

    #[arg(long, default_value_t = false)]
    pub(crate) yolo: bool,

    #[arg(long, default_value_t = false)]
    pub(crate) local_api: bool,

    #[arg(long, default_value = "127.0.0.1:0")]
    pub(crate) local_api_bind: String,

    #[arg(long)]
    pub(crate) local_api_token: Option<String>,

    #[arg(trailing_var_arg = true)]
    pub(crate) prompt: Vec<String>,
}

pub(crate) fn parse_cli_from<I, T>(args: I) -> std::result::Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args = rewrite_resume_startup_args(args.into_iter().map(Into::into).collect());
    Cli::try_parse_from(args).map(normalize_cli)
}

pub(crate) fn parse_cli() -> Cli {
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
            | "--local-api"
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
            | "--local-api-bind"
            | "--local-api-token"
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
