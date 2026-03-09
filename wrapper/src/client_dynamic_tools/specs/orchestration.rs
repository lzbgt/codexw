use serde_json::Value;
use serde_json::json;

pub(crate) fn orchestration_tool_specs() -> Vec<Value> {
    vec![
        json!({
            "name": "orchestration_status",
            "description": "Summarize the current orchestration state, including worker counts, dependency health, and the first concrete tool-native next action when one exists.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "orchestration_list_workers",
            "description": "Render the current orchestration worker graph, optionally filtered to all, blockers, dependencies, agents, shells, services, capabilities, terminals, guidance, or actions. Blockers, guidance, and actions may also be narrowed to one @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "blockers", "dependencies", "agents", "shells", "services", "capabilities", "terminals", "guidance", "actions"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "orchestration_suggest_actions",
            "description": "Render concrete next-step dynamic tool suggestions for the current orchestration state, such as capability inspection, readiness waits, service attach, or scoped cleanup actions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "orchestration_list_dependencies",
            "description": "Render the current orchestration dependency graph, optionally filtered to all, blocking, sidecars, missing, booting, ambiguous, or satisfied dependency states and optionally narrowed to one @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "blocking", "sidecars", "missing", "booting", "ambiguous", "satisfied"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
    ]
}
