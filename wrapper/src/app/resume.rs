use std::ffi::OsString;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::commands_completion_render::quote_if_needed;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn emit_resume_exit_hint(
    output: &mut Output,
    state: &AppState,
    resolved_cwd: &str,
) -> Result<()> {
    if state.resume_exit_hint_emitted {
        return Ok(());
    }
    let Some(line) = build_resume_hint_line(
        &current_program_name(),
        resolved_cwd,
        state.thread_id.as_deref(),
    ) else {
        return Ok(());
    };
    output.line_stderr(line)?;
    Ok(())
}

pub(crate) fn current_program_name() -> String {
    std::env::args_os()
        .next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "codexw".to_string())
}

pub(crate) fn build_resume_hint_line(
    program: &str,
    resolved_cwd: &str,
    thread_id: Option<&str>,
) -> Option<String> {
    thread_id.map(|thread_id| {
        format!(
            "[session] resume with: {}",
            build_resume_command(program, resolved_cwd, thread_id)
        )
    })
}

pub(crate) fn build_resume_command(program: &str, resolved_cwd: &str, thread_id: &str) -> String {
    format!(
        "{} --cwd {} resume {}",
        quote_if_needed(program),
        quote_if_needed(resolved_cwd),
        quote_if_needed(thread_id)
    )
}

pub(crate) fn build_resume_args(
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: &str,
    initial_prompt: Option<&str>,
) -> Vec<OsString> {
    let mut args = Vec::new();
    args.push(OsString::from("--codex-bin"));
    args.push(OsString::from(&cli.codex_bin));
    for override_kv in &cli.config_overrides {
        args.push(OsString::from("--config"));
        args.push(OsString::from(override_kv));
    }
    for feature in &cli.enable_features {
        args.push(OsString::from("--enable"));
        args.push(OsString::from(feature));
    }
    for feature in &cli.disable_features {
        args.push(OsString::from("--disable"));
        args.push(OsString::from(feature));
    }
    args.push(OsString::from("--cwd"));
    args.push(OsString::from(resolved_cwd));
    if let Some(model) = cli.model.as_deref() {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(provider) = cli.model_provider.as_deref() {
        args.push(OsString::from("--model-provider"));
        args.push(OsString::from(provider));
    }
    if cli.verbose_events {
        args.push(OsString::from("--verbose-events"));
    }
    if cli.raw_json {
        args.push(OsString::from("--raw-json"));
    }
    if cli.no_experimental_api {
        args.push(OsString::from("--no-experimental-api"));
    }
    if cli.yolo {
        args.push(OsString::from("--yolo"));
    }
    if cli.local_api {
        args.push(OsString::from("--local-api"));
        args.push(OsString::from("--local-api-bind"));
        args.push(OsString::from(&cli.local_api_bind));
        if let Some(token) = cli.local_api_token.as_deref() {
            args.push(OsString::from("--local-api-token"));
            args.push(OsString::from(token));
        }
    }
    args.push(OsString::from("resume"));
    args.push(OsString::from(thread_id));
    if let Some(prompt) = initial_prompt {
        args.push(OsString::from(prompt));
    }
    args
}

pub(crate) fn reexec_into_resume(
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: &str,
    initial_prompt: Option<&str>,
) -> Result<()> {
    let current_exe = std::env::current_exe().context("resolve current codexw executable")?;
    let args = build_resume_args(cli, resolved_cwd, thread_id, initial_prompt);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let err = Command::new(&current_exe).args(&args).exec();
        Err(err).with_context(|| {
            format!(
                "exec self-heal resume via `{}`",
                current_exe.to_string_lossy()
            )
        })
    }
    #[cfg(not(unix))]
    {
        Command::new(&current_exe)
            .args(&args)
            .spawn()
            .with_context(|| {
                format!(
                    "spawn self-heal resume via `{}`",
                    current_exe.to_string_lossy()
                )
            })?;
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::build_resume_args;
    use crate::Cli;

    #[test]
    fn build_resume_args_preserves_runtime_flags_and_prompt() {
        let cli = Cli {
            codex_bin: "/usr/local/bin/codex".to_string(),
            config_overrides: vec!["foo=bar".to_string()],
            enable_features: vec!["feature_a".to_string()],
            disable_features: vec!["feature_b".to_string()],
            resume: None,
            resume_picker: false,
            cwd: None,
            model: Some("gpt-5".to_string()),
            model_provider: Some("openai".to_string()),
            auto_continue: true,
            verbose_events: true,
            verbose_thinking: true,
            raw_json: true,
            no_experimental_api: true,
            yolo: true,
            local_api: true,
            local_api_bind: "127.0.0.1:4000".to_string(),
            local_api_token: Some("secret".to_string()),
            prompt: Vec::new(),
        };

        let args = build_resume_args(
            &cli,
            "/tmp/repo",
            "thread_123",
            Some("continue from self-heal"),
        );
        let args = args
            .into_iter()
            .map(|value| value.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(args.windows(2).any(|pair| pair == ["--codex-bin", "/usr/local/bin/codex"]));
        assert!(args.windows(2).any(|pair| pair == ["--config", "foo=bar"]));
        assert!(args.windows(2).any(|pair| pair == ["--enable", "feature_a"]));
        assert!(args.windows(2).any(|pair| pair == ["--disable", "feature_b"]));
        assert!(args.windows(2).any(|pair| pair == ["--cwd", "/tmp/repo"]));
        assert!(args.windows(2).any(|pair| pair == ["--model", "gpt-5"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--model-provider", "openai"]));
        assert!(args.contains(&"--verbose-events".to_string()));
        assert!(args.contains(&"--raw-json".to_string()));
        assert!(args.contains(&"--no-experimental-api".to_string()));
        assert!(args.contains(&"--yolo".to_string()));
        assert!(args.contains(&"--local-api".to_string()));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--local-api-bind", "127.0.0.1:4000"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--local-api-token", "secret"]));
        assert!(args.ends_with(&[
            "resume".to_string(),
            "thread_123".to_string(),
            "continue from self-heal".to_string(),
        ]));
    }
}
