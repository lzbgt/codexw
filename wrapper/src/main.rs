mod app;
mod catalog;
mod catalog_views;
mod commands;
mod dispatch;
mod editor;
mod events;
mod history;
mod input;
mod interaction;
mod notifications;
mod output;
mod policy;
mod prompt;
mod prompting;
mod render;
mod requests;
mod responses;
mod rpc;
mod runtime;
mod session;
mod state;
mod status_views;
mod transcript_views;
mod views;

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

#[cfg(test)]
mod tests {
    use super::Cli;
    use super::normalize_cli;
    use crate::commands::builtin_command_names;
    use crate::commands::builtin_help_lines;
    use crate::commands::quote_if_needed;
    use crate::commands::render_slash_completion_candidates;
    use crate::commands::try_complete_slash_command;
    use crate::editor::LineEditor;
    use crate::events::params_auto_approval_result;
    use crate::history::latest_conversation_history_items;
    use crate::history::seed_resumed_state_from_turns;
    use crate::input::AppCatalogEntry;
    use crate::interaction::is_builtin_command;
    use crate::interaction::parse_feedback_args;
    use crate::interaction::prompt_accepts_input;
    use crate::interaction::prompt_is_visible;
    use crate::interaction::try_complete_file_token;
    use crate::policy::choose_command_approval_decision;
    use crate::session::CollaborationModePreset;
    use crate::session::extract_collaboration_mode_presets;
    use crate::session::extract_models;
    use crate::session::render_collaboration_modes;
    use crate::session::render_personality_options;
    use crate::session::render_prompt_status;
    use crate::session::render_realtime_item;
    use crate::session::render_status_snapshot;
    use crate::session::summarize_active_collaboration_mode;
    use crate::session::summarize_active_personality;
    use crate::state::AppState;
    use crate::views::build_tool_user_input_response;
    use crate::views::extract_file_search_paths;
    use crate::views::extract_thread_ids;
    use crate::views::render_apps_list;
    use crate::views::render_experimental_features_list;
    use crate::views::render_fuzzy_file_search_results;
    use crate::views::render_models_list;
    use crate::views::render_rate_limit_lines;
    use crate::views::render_reasoning_item;
    use crate::views::render_thread_list;
    use crate::views::summarize_terminal_interaction;
    use crate::views::summarize_thread_status_for_display;
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn yolo_prefers_first_available_command_approval_decision() {
        let params = json!({
            "availableDecisions": [
                "acceptForSession",
                "accept"
            ]
        });
        assert_eq!(
            choose_command_approval_decision(&params, true),
            json!("acceptForSession")
        );
    }

    #[test]
    fn command_approval_defaults_to_accept() {
        assert_eq!(
            choose_command_approval_decision(&json!({}), false),
            json!("accept")
        );
    }

    #[test]
    fn approval_prefers_allow_decisions_over_first_entry() {
        let params = json!({
            "availableDecisions": [
                "decline",
                "accept",
                "cancel"
            ]
        });
        assert_eq!(
            choose_command_approval_decision(&params, false),
            json!("accept")
        );
    }

    #[test]
    fn generic_approval_prefers_session_accept_when_available() {
        let params = json!({
            "availableDecisions": [
                "decline",
                "acceptForSession",
                "accept"
            ]
        });
        assert_eq!(
            params_auto_approval_result(&params),
            json!({"decision": "acceptForSession"})
        );
    }

    #[test]
    fn slash_aliases_are_treated_as_builtin_commands() {
        assert!(is_builtin_command("status"));
        assert!(is_builtin_command("statusline"));
        assert!(is_builtin_command("resume thread-1"));
        assert!(is_builtin_command("apps"));
        assert!(is_builtin_command("skills"));
        assert!(is_builtin_command("models"));
        assert!(is_builtin_command("settings"));
        assert!(is_builtin_command("compact"));
        assert!(is_builtin_command("review current changes"));
        assert!(is_builtin_command("permissions"));
        assert!(is_builtin_command("feedback bug something broke"));
        assert!(is_builtin_command("logout"));
        assert!(is_builtin_command("mcp"));
        assert!(is_builtin_command("threads"));
        assert!(is_builtin_command("mention foo"));
        assert!(is_builtin_command("diff"));
        assert!(!is_builtin_command("unknown-command"));
    }

    #[test]
    fn tool_user_input_defaults_to_first_option() {
        let response = build_tool_user_input_response(&json!({
            "questions": [
                {
                    "id": "confirm_path",
                    "options": [
                        {"label": "yes", "description": "Proceed"},
                        {"label": "no", "description": "Stop"}
                    ]
                }
            ]
        }));
        assert_eq!(
            response,
            json!({
                "answers": {
                    "confirm_path": { "answers": ["yes"] }
                }
            })
        );
    }

    #[test]
    fn reasoning_prefers_summary_blocks() {
        let rendered = render_reasoning_item(&json!({
            "summary": ["First block", "Second block"],
            "content": ["raw detail"]
        }));
        assert_eq!(rendered, "First block\n\nSecond block");
    }

    #[test]
    fn empty_terminal_interaction_is_suppressed() {
        assert_eq!(
            summarize_terminal_interaction(&json!({
                "processId": "123",
                "stdin": ""
            })),
            None
        );
    }

    #[test]
    fn terminal_interaction_only_surfaces_meaningful_stdin() {
        assert_eq!(
            summarize_terminal_interaction(&json!({
                "processId": "123",
                "stdin": "yes\n"
            })),
            Some("process=123 stdin=yes".to_string())
        );
    }

    #[test]
    fn tab_completes_unique_slash_command() {
        let mut editor = LineEditor::default();
        for ch in "/di".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();
        assert!(try_complete_slash_command(&mut editor, &buffer, cursor).is_some());
        assert_eq!(editor.buffer(), "/diff ");
    }

    #[test]
    fn ambiguous_slash_completion_lists_candidates() {
        let mut editor = LineEditor::default();
        for ch in "/re".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();
        let result = try_complete_slash_command(&mut editor, &buffer, cursor)
            .expect("expected slash completion result");
        let rendered = result.rendered_candidates.expect("expected candidate list");
        assert_eq!(editor.buffer(), "/re");
        assert!(rendered.contains("/resume"));
        assert!(rendered.contains("/review"));
    }

    #[test]
    fn fuzzy_slash_completion_lists_candidates() {
        let mut editor = LineEditor::default();
        for ch in "/ac".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();
        let result = try_complete_slash_command(&mut editor, &buffer, cursor)
            .expect("expected slash completion result");
        let rendered = result.rendered_candidates.expect("expected candidate list");
        assert_eq!(editor.buffer(), "/ac");
        assert!(rendered.contains("/feedback"));
        assert!(rendered.contains("Fuzzy matches for /ac:"));
    }

    #[test]
    fn tab_completes_unique_file_token() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file_path = temp.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&file_path, "fn main() {}\n").expect("write");

        let mut editor = LineEditor::default();
        for ch in "@src/ma".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();

        let result = try_complete_file_token(
            &mut editor,
            &buffer,
            cursor,
            temp.path().to_str().expect("utf8"),
        )
        .expect("complete")
        .expect("some result");

        assert!(result.rendered_candidates.is_none());
        assert_eq!(editor.buffer(), "src/main.rs ");
    }

    #[test]
    fn tab_lists_ambiguous_file_completions() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("alpha.txt"), "a").expect("write alpha");
        std::fs::write(temp.path().join("alpine.txt"), "b").expect("write alpine");

        let mut editor = LineEditor::default();
        for ch in "@al".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();

        let result = try_complete_file_token(
            &mut editor,
            &buffer,
            cursor,
            temp.path().to_str().expect("utf8"),
        )
        .expect("complete")
        .expect("some result");

        let rendered = result.rendered_candidates.expect("candidate list");
        assert!(rendered.contains("alpha.txt"));
        assert!(rendered.contains("alpine.txt"));
        assert_eq!(editor.buffer(), "@alp");
    }

    #[test]
    fn thread_list_is_numbered_and_extractable() {
        let result = json!({
            "data": [
                {
                    "id": "thr_1",
                    "preview": "first thread",
                    "status": {"type": "idle"},
                    "updatedAt": 1
                },
                {
                    "id": "thr_2",
                    "preview": "second thread",
                    "status": {"type": "active"},
                    "updatedAt": 2
                }
            ]
        });
        let rendered = render_thread_list(&result, None);
        assert!(rendered.contains(" 1. thr_1"));
        assert!(rendered.contains("Use /resume <n>"));
        assert_eq!(extract_thread_ids(&result), vec!["thr_1", "thr_2"]);
    }

    #[test]
    fn resume_helpers_only_keep_recent_conversation_context() {
        let turns = vec![
            json!({
                "items": [
                    {"type": "userMessage", "content": [{"type": "text", "text": "old objective"}]},
                    {"type": "agentMessage", "text": "old reply"},
                    {"type": "reasoning", "text": "ignore"}
                ]
            }),
            json!({
                "items": [
                    {"type": "userMessage", "content": [{"type": "text", "text": "latest request"}]},
                    {"type": "agentMessage", "text": "latest reply"}
                ]
            }),
        ];

        let mut state = AppState::new(true, false);
        seed_resumed_state_from_turns(&turns, &mut state);
        assert_eq!(state.objective.as_deref(), Some("latest request"));
        assert_eq!(state.last_agent_message.as_deref(), Some("latest reply"));

        let recent_items = latest_conversation_history_items(&turns, 2);
        assert_eq!(recent_items.len(), 2);
        assert_eq!(
            recent_items[0].get("type").and_then(Value::as_str),
            Some("userMessage")
        );
        assert_eq!(
            recent_items[1].get("type").and_then(Value::as_str),
            Some("agentMessage")
        );
    }

    #[test]
    fn file_search_paths_are_extractable_for_numeric_insert() {
        let files = vec![
            json!({"path": "src/main.rs", "score": 1}),
            json!({"path": "src/lib.rs", "score": 2}),
        ];
        assert_eq!(
            extract_file_search_paths(&files),
            vec!["src/main.rs", "src/lib.rs"]
        );
        assert_eq!(quote_if_needed("src/main.rs"), "src/main.rs");
        assert_eq!(
            quote_if_needed("path with spaces.rs"),
            "\"path with spaces.rs\""
        );
    }

    #[test]
    fn fuzzy_file_search_rendering_shows_ranked_paths() {
        let rendered = render_fuzzy_file_search_results(
            "agent",
            &[
                json!({"path": "src/agent.rs", "score": 99}),
                json!({"path": "tests/agent_test.rs", "score": 78}),
            ],
        );
        assert!(rendered.contains("Query: agent"));
        assert!(rendered.contains("1. src/agent.rs  [score 99]"));
        assert!(rendered.contains("2. tests/agent_test.rs  [score 78]"));
    }

    #[test]
    fn slash_completion_rendering_includes_descriptions() {
        let rendered = render_slash_completion_candidates("re", &["resume", "review"], false);
        assert!(rendered.contains("/resume"));
        assert!(rendered.contains("resume a saved thread"));
        assert!(rendered.contains("/review"));
    }

    #[test]
    fn bare_slash_completion_uses_native_like_order() {
        let rendered = render_slash_completion_candidates("", builtin_command_names(), false);
        let model_pos = rendered.find("/model").expect("model should be listed");
        let review_pos = rendered.find("/review").expect("review should be listed");
        let new_pos = rendered.find("/new").expect("new should be listed");
        assert!(model_pos < review_pos);
        assert!(review_pos < new_pos);
    }

    #[test]
    fn help_lines_are_derived_from_command_metadata() {
        let rendered = builtin_help_lines().join("\n");
        assert!(rendered.contains(":resume [thread-id|n]"));
        assert!(rendered.contains("resume a saved thread"));
        assert!(rendered.contains(":plan"));
        assert!(rendered.contains("toggle plan collaboration mode"));
        assert!(rendered.contains(":approvals or /permissions"));
        assert!(rendered.contains(":ps [clean]"));
        assert!(rendered.contains("stop all background terminals"));
        assert!(rendered.contains(":realtime [start [prompt...]|send <text>|stop|status]"));
    }

    #[test]
    fn app_list_rendering_includes_slug_and_status() {
        let rendered = render_apps_list(&[AppCatalogEntry {
            id: "connector-1".to_string(),
            name: "Demo App".to_string(),
            slug: "demo-app".to_string(),
            enabled: true,
        }]);
        assert!(rendered.contains("Demo App"));
        assert!(rendered.contains("$demo-app"));
        assert!(rendered.contains("[enabled]"));
    }

    #[test]
    fn rate_limit_lines_show_remaining_capacity_and_reset() {
        let lines = render_rate_limit_lines(Some(&json!({
            "primary": {
                "usedPercent": 25,
                "windowDurationMins": 300,
                "resetsAt": 2200000000i64
            },
            "secondary": null
        })));
        assert!(lines[0].contains("5h limit 75% left"));
        assert!(lines[0].contains("resets"));
    }

    #[test]
    fn collaboration_modes_are_extractable_from_response() {
        let presets = extract_collaboration_mode_presets(&json!({
            "data": [
                {
                    "name": "Plan",
                    "mode": "plan",
                    "model": "gpt-5-codex",
                    "reasoning_effort": "high"
                },
                {
                    "name": "Default",
                    "mode": "default",
                    "model": "gpt-5-codex",
                    "reasoning_effort": null
                }
            ]
        }));
        assert_eq!(presets.len(), 2);
        assert!(presets[0].is_plan());
        assert_eq!(presets[1].mode_kind.as_deref(), Some("default"));
    }

    #[test]
    fn collaboration_mode_rendering_shows_current_and_available_presets() {
        let mut state = AppState::new(true, false);
        let presets = extract_collaboration_mode_presets(&json!({
            "data": [
                {
                    "name": "Plan",
                    "mode": "plan",
                    "model": "gpt-5-codex",
                    "reasoning_effort": "high"
                }
            ]
        }));
        state.collaboration_modes = presets.clone();
        state.active_collaboration_mode = Some(presets[0].clone());
        let rendered = render_collaboration_modes(&state);
        assert!(rendered.contains("current         Plan"));
        assert!(rendered.contains("mode=plan"));
        assert!(rendered.contains("Use /collab <name|mode> or /plan to switch."));
    }

    #[test]
    fn experimental_feature_rendering_shows_stage_status_and_key() {
        let rendered = render_experimental_features_list(&json!({
            "data": [
                {
                    "name": "realtime_conversation",
                    "stage": "beta",
                    "displayName": "Realtime conversation",
                    "description": "Enable the experimental realtime voice workflow.",
                    "announcement": "Try voice mode in supported clients.",
                    "enabled": true,
                    "defaultEnabled": false
                }
            ],
            "nextCursor": null
        }));
        assert!(rendered.contains("Realtime conversation  [beta] [enabled]"));
        assert!(rendered.contains("key: realtime_conversation"));
        assert!(rendered.contains("Enable the experimental realtime voice workflow."));
    }

    #[test]
    fn models_are_extractable_with_personality_support() {
        let models = extract_models(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                },
                {
                    "id": "legacy-model",
                    "displayName": "Legacy",
                    "supportsPersonality": false,
                    "isDefault": false
                }
            ]
        }));
        assert_eq!(models.len(), 2);
        assert!(models[0].supports_personality);
        assert!(models[0].is_default);
        assert!(!models[1].supports_personality);
    }

    #[test]
    fn personality_rendering_shows_current_and_model_support() {
        let mut state = AppState::new(true, false);
        state.models = extract_models(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                }
            ]
        }));
        state.active_personality = Some("pragmatic".to_string());
        let cli = Cli {
            codex_bin: "codex".to_string(),
            config_overrides: Vec::new(),
            enable_features: Vec::new(),
            disable_features: Vec::new(),
            resume: None,
            cwd: None,
            model: None,
            model_provider: None,
            auto_continue: true,
            verbose_events: false,
            verbose_thinking: true,
            raw_json: false,
            no_experimental_api: false,
            yolo: false,
            prompt: Vec::new(),
        };
        let rendered = render_personality_options(&cli, &state);
        assert_eq!(summarize_active_personality(&state), "Pragmatic");
        assert!(rendered.contains("current          Pragmatic"));
        assert!(rendered.contains("current model     GPT-5 Codex [supports personality]"));
    }

    #[test]
    fn models_render_default_and_personality_support_markers() {
        let rendered = render_models_list(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                },
                {
                    "id": "legacy-model",
                    "displayName": "Legacy",
                    "supportsPersonality": false,
                    "isDefault": false
                }
            ]
        }));
        assert!(rendered.contains("GPT-5 Codex (gpt-5-codex) [default] [supports personality]"));
        assert!(rendered.contains("Legacy (legacy-model) [personality unsupported]"));
    }

    #[test]
    fn status_snapshot_surfaces_effective_model_personality_support() {
        let mut state = AppState::new(true, false);
        state.models = extract_models(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                }
            ]
        }));
        let cli = Cli {
            codex_bin: "codex".to_string(),
            config_overrides: Vec::new(),
            enable_features: Vec::new(),
            disable_features: Vec::new(),
            resume: None,
            cwd: None,
            model: None,
            model_provider: None,
            auto_continue: true,
            verbose_events: false,
            verbose_thinking: true,
            raw_json: false,
            no_experimental_api: false,
            yolo: false,
            prompt: Vec::new(),
        };
        let rendered = render_status_snapshot(&cli, "/tmp/project", &state);
        assert!(rendered.contains("model           GPT-5 Codex [supports personality]"));
        assert!(rendered.contains("models cached   1"));
    }

    #[test]
    fn prompt_visibility_and_input_follow_runtime_state() {
        let mut state = AppState::new(true, false);
        assert!(!prompt_is_visible(&state));
        assert!(!prompt_accepts_input(&state));

        state.thread_id = Some("thread-1".to_string());
        assert!(prompt_is_visible(&state));
        assert!(prompt_accepts_input(&state));

        state.pending_thread_switch = true;
        assert!(!prompt_is_visible(&state));
        assert!(!prompt_accepts_input(&state));

        state.pending_thread_switch = false;
        state.active_exec_process_id = Some("proc-1".to_string());
        assert!(prompt_is_visible(&state));
        assert!(!prompt_accepts_input(&state));
    }

    #[test]
    fn thread_status_summary_prefers_human_flags() {
        assert_eq!(
            summarize_thread_status_for_display(&json!({
                "status": {"type": "active", "activeFlags": ["waitingOnApproval"]}
            })),
            Some("waiting on approval".to_string())
        );
        assert_eq!(
            summarize_thread_status_for_display(&json!({
                "status": {"type": "idle", "activeFlags": []}
            })),
            Some("ready".to_string())
        );
    }

    #[test]
    fn prompt_status_uses_active_detail_when_present() {
        let mut state = AppState::new(true, false);
        state.turn_running = true;
        state.started_turn_count = 2;
        state.last_status_line = Some("waiting on approval".to_string());
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("waiting on approval"));
    }

    #[test]
    fn prompt_status_mentions_plan_mode_when_selected() {
        let mut state = AppState::new(true, false);
        state.active_collaboration_mode = Some(CollaborationModePreset {
            name: "Plan".to_string(),
            mode_kind: Some("plan".to_string()),
            model: Some("gpt-5-codex".to_string()),
            reasoning_effort: Some(Some("high".to_string())),
        });
        assert_eq!(
            summarize_active_collaboration_mode(&state),
            "Plan (mode=plan, model=gpt-5-codex, effort=high)"
        );
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("plan mode"));
    }

    #[test]
    fn prompt_status_mentions_personality_when_selected() {
        let mut state = AppState::new(true, false);
        state.active_personality = Some("friendly".to_string());
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("Friendly"));
    }

    #[test]
    fn prompt_status_mentions_realtime_when_active() {
        let mut state = AppState::new(true, false);
        state.realtime_active = true;
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("realtime"));
    }

    #[test]
    fn status_snapshot_includes_realtime_fields() {
        let mut state = AppState::new(true, false);
        state.thread_id = Some("thread-1".to_string());
        state.realtime_active = true;
        state.realtime_session_id = Some("rt-1".to_string());
        state.realtime_prompt = Some("hello world".to_string());
        state.realtime_last_error = Some("bad gateway".to_string());
        let cli = normalize_cli(Cli {
            codex_bin: "codex".to_string(),
            config_overrides: Vec::new(),
            enable_features: Vec::new(),
            disable_features: Vec::new(),
            resume: None,
            cwd: None,
            model: None,
            model_provider: None,
            auto_continue: true,
            verbose_events: false,
            verbose_thinking: true,
            raw_json: false,
            no_experimental_api: false,
            yolo: false,
            prompt: Vec::new(),
        });
        let rendered = render_status_snapshot(&cli, "/tmp/project", &state);
        assert!(rendered.contains("realtime        true"));
        assert!(rendered.contains("realtime id     rt-1"));
        assert!(rendered.contains("realtime prompt hello world"));
        assert!(rendered.contains("realtime error  bad gateway"));
    }

    #[test]
    fn realtime_item_prefers_text_content() {
        let rendered = render_realtime_item(&json!({
            "type": "message",
            "id": "msg-1",
            "role": "assistant",
            "content": [
                {"text": "first line"},
                {"transcript": "second line"}
            ]
        }));
        assert!(rendered.contains("type            message"));
        assert!(rendered.contains("id              msg-1"));
        assert!(rendered.contains("role            assistant"));
        assert!(rendered.contains("first line"));
        assert!(rendered.contains("second line"));
    }

    #[test]
    fn normalize_cli_supports_codex_style_resume_startup() {
        let cli = normalize_cli(Cli {
            codex_bin: "codex".to_string(),
            config_overrides: Vec::new(),
            enable_features: Vec::new(),
            disable_features: Vec::new(),
            resume: None,
            cwd: None,
            model: None,
            model_provider: None,
            auto_continue: true,
            verbose_events: false,
            verbose_thinking: true,
            raw_json: false,
            no_experimental_api: false,
            yolo: false,
            prompt: vec![
                "resume".to_string(),
                "thread-123".to_string(),
                "continue".to_string(),
                "work".to_string(),
            ],
        });
        assert_eq!(cli.resume.as_deref(), Some("thread-123"));
        assert_eq!(cli.prompt, vec!["continue".to_string(), "work".to_string()]);
    }

    #[test]
    fn feedback_args_parse_category_reason_and_logs() {
        let parsed = parse_feedback_args(&[
            "bug".to_string(),
            "command".to_string(),
            "output".to_string(),
            "was".to_string(),
            "wrong".to_string(),
            "--logs".to_string(),
        ])
        .expect("expected feedback args to parse");
        assert_eq!(parsed.classification, "bug");
        assert_eq!(parsed.reason.as_deref(), Some("command output was wrong"));
        assert!(parsed.include_logs);
    }

    #[test]
    fn feedback_args_accept_aliases() {
        let parsed =
            parse_feedback_args(&["good".to_string()]).expect("expected feedback args to parse");
        assert_eq!(parsed.classification, "good_result");
        assert_eq!(parsed.reason, None);
        assert!(!parsed.include_logs);
    }
}
