use crate::history::latest_conversation_history_items;
use crate::history::seed_resumed_state_from_turns;
use crate::state::AppState;
use crate::status_views::render_rate_limit_lines;
use serde_json::Value;
use serde_json::json;

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
fn rate_limit_lines_show_remaining_capacity_and_reset() {
    let rendered = render_rate_limit_lines(Some(&json!({
        "primary": {
            "usedPercent": 25,
            "windowDurationMins": 300,
            "resetsAt": "2026-03-08T14:30:00Z"
        },
        "secondary": {
            "usedPercent": 40,
            "windowDurationMins": 10080,
            "resetsAt": "2026-03-10T09:00:00Z"
        }
    })));
    let rendered = rendered.join("\n");
    assert!(rendered.contains("5h limit 75% left"));
    assert!(rendered.contains("weekly limit 60% left"));
}
