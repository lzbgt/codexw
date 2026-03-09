use super::*;

#[test]
fn rollout_message_uses_current_path_when_available() {
    let mut state = AppState::new(true, false);
    state.current_rollout_path = Some(std::path::PathBuf::from("/tmp/codex-rollout.jsonl"));
    assert_eq!(
        current_rollout_message(&state),
        "Current rollout path: /tmp/codex-rollout.jsonl"
    );
}

#[test]
fn rollout_message_explains_missing_path() {
    let state = AppState::new(true, false);
    assert_eq!(
        current_rollout_message(&state),
        "Rollout path is not available yet."
    );
}
