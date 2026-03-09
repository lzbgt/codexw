use serde_json::json;

use super::super::tracking::track_collab_agent_task_completed;
use super::super::tracking::track_collab_agent_task_started;

#[test]
fn collab_agent_task_tracking_updates_live_registry_and_cached_threads() {
    let mut state = crate::state::AppState::new(true, false);
    let item = json!({
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
    });

    track_collab_agent_task_started(&mut state, &item);

    assert_eq!(state.live_agent_tasks.len(), 1);
    assert_eq!(state.cached_agent_threads.len(), 1);
    assert_eq!(state.cached_agent_threads[0].id, "thread-agent-1");
    assert_eq!(state.cached_agent_threads[0].status, "running");
    assert_eq!(state.cached_agent_threads[0].preview, "reviewing auth flow");

    let completed = json!({
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
    });
    track_collab_agent_task_completed(&mut state, &completed);

    assert!(state.live_agent_tasks.is_empty());
    assert_eq!(state.cached_agent_threads[0].status, "completed");
    assert_eq!(state.cached_agent_threads[0].preview, "done");
}
