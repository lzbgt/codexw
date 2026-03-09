use super::super::*;

#[test]
fn ps_command_can_invoke_parameterized_service_recipe_for_aliased_job() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "recipes": [
                    {
                        "name": "say",
                        "description": "Write one message to the service shell",
                        "parameters": [
                            {
                                "name": "message",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "stdin",
                            "text": "{{message}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    handle_ps_command(
        "alias 1 dev.api",
        &["alias", "1", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell");

    handle_ps_command(
        r#"run dev.api say {"message":"hello from parameterized recipe"}"#,
        &[
            "run",
            "dev.api",
            "say",
            r#"{"message":"hello"#,
            "from",
            "parameterized",
            r#"recipe"}"#,
        ],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("invoke parameterized recipe");

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = state
            .background_shells
            .poll_job("bg-1", 0, 200)
            .expect("poll shell directly");
        if rendered.contains("hello from parameterized recipe") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("hello from parameterized recipe"));
    let _ = state.background_shells.terminate_all_running();
}
