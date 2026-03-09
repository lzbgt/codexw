use super::super::super::*;

#[test]
fn collab_wait_item_sets_waiting_on_agent_status() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();

    handle_status_update(
        "item/started",
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "wait-1",
                "tool": "wait",
                "status": "inProgress",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "running"
                    }
                }
            }
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle wait start");

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on agent thread-agent-1")
    );
}

#[test]
fn completing_one_wait_task_keeps_status_for_remaining_waits() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    for (call_id, agent_id) in [("wait-1", "thread-agent-1"), ("wait-2", "thread-agent-2")] {
        handle_status_update(
            "item/started",
            &json!({
                "item": {
                    "type": "collabAgentToolCall",
                    "id": call_id,
                    "tool": "wait",
                    "status": "inProgress",
                    "senderThreadId": "thread-main",
                    "receiverThreadIds": [agent_id],
                    "agentsStates": {
                        agent_id: {
                            "status": "running"
                        }
                    }
                }
            }),
            &cli,
            &mut state,
            &mut output,
        )
        .expect("start wait task");
    }

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on agents thread-agent-1, thread-agent-2")
    );

    render_item_completed(
        &cli,
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "wait-1",
                "tool": "wait",
                "status": "completed",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "completed",
                        "message": "done"
                    }
                }
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("complete first wait");

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on agent thread-agent-2")
    );
}

#[test]
fn collab_agent_items_register_live_agent_tasks_and_cache_threads() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();

    handle_status_update(
        "item/started",
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "call-1",
                "tool": "spawnAgent",
                "status": "inProgress",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "prompt": "Inspect auth flow and report risks",
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "running",
                        "message": "reviewing auth flow"
                    }
                }
            }
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle collab start");

    assert_eq!(state.live_agent_tasks.len(), 1);
    assert_eq!(state.cached_agent_threads.len(), 1);
    assert_eq!(state.cached_agent_threads[0].id, "thread-agent-1");
    assert_eq!(state.cached_agent_threads[0].status, "running");

    render_item_completed(
        &cli,
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "call-1",
                "tool": "spawnAgent",
                "status": "completed",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "completed",
                        "message": "done"
                    }
                }
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("complete collab call");

    assert!(state.live_agent_tasks.is_empty());
    assert_eq!(state.cached_agent_threads[0].status, "completed");
    assert_eq!(state.cached_agent_threads[0].preview, "done");
}
