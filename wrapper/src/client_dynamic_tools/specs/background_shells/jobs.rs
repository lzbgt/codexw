use serde_json::Value;
use serde_json::json;

pub(super) fn interactive_job_tool_specs() -> Vec<Value> {
    vec![
        json!({
            "name": "background_shell_start",
            "description": "Start a long-running shell command in the background so you can continue other work in the same turn. Use `intent=prerequisite` for critical-path work you will need before finishing, `intent=observation` for non-blocking sidecar work such as tests or searches, and `intent=service` for reusable long-lived helpers such as dev servers. Jobs may also declare `dependsOnCapabilities` so the orchestration graph can model durable dependencies on reusable services, and service jobs may additionally declare `capabilities`, `readyPattern`, `protocol`, `endpoint`, `attachHint`, and structured `recipes` so the wrapper can distinguish booting versus ready services, expose a reusable attach surface, and invoke typed service recipes later.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string"},
                    "cwd": {"type": "string"},
                    "intent": {
                        "type": "string",
                        "enum": ["prerequisite", "observation", "service"]
                    },
                    "label": {"type": "string"},
                    "capabilities": {
                        "type": ["array", "null"],
                        "items": {"type": "string"}
                    },
                    "dependsOnCapabilities": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "readyPattern": {"type": "string"},
                    "protocol": {"type": "string"},
                    "endpoint": {"type": "string"},
                    "attachHint": {"type": "string"},
                    "recipes": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "description": {"type": "string"},
                                "example": {"type": "string"},
                                "parameters": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"},
                                            "description": {"type": "string"},
                                            "default": {"type": "string"},
                                            "required": {"type": "boolean"}
                                        },
                                        "required": ["name"]
                                    }
                                },
                                "action": {
                                    "type": "object",
                                    "properties": {
                                        "type": {
                                            "type": "string",
                                            "enum": ["informational", "stdin", "http", "tcp", "redis"]
                                        },
                                        "text": {"type": "string"},
                                        "appendNewline": {"type": "boolean"},
                                        "method": {"type": "string"},
                                        "path": {"type": "string"},
                                        "body": {"type": "string"},
                                        "payload": {"type": "string"},
                                        "command": {
                                            "type": "array",
                                            "items": {"type": "string"}
                                        },
                                        "expectSubstring": {"type": "string"},
                                        "readTimeoutMs": {"type": "integer", "minimum": 1},
                                        "headers": {
                                            "type": "object",
                                            "additionalProperties": {"type": "string"}
                                        },
                                        "expectedStatus": {
                                            "type": "integer",
                                            "minimum": 100,
                                            "maximum": 599
                                        }
                                    }
                                }
                            },
                            "required": ["name"]
                        }
                    }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "background_shell_poll",
            "description": "Inspect a background shell job by jobId, alias, or @capability and fetch new output lines since an optional afterLine cursor. Use one final poll to collect terminal output, but do not keep polling jobs that are already completed, failed, or terminated; cursor-based exhausted terminal polls fail explicitly so callers can stop retrying.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "afterLine": {"type": "integer", "minimum": 0},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 200}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_send",
            "description": "Send stdin text to a running background shell job by jobId, alias, or @capability. Defaults to appending a trailing newline.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "text": {"type": "string"},
                    "appendNewline": {"type": "boolean"}
                },
                "required": ["jobId", "text"]
            }
        }),
        json!({
            "name": "background_shell_set_alias",
            "description": "Assign or clear a stable in-session alias for a background shell job by jobId, alias, or @capability. Pass `alias=null` to clear the current alias.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "alias": {"type": ["string", "null"]}
                },
                "required": ["jobId", "alias"]
            }
        }),
    ]
}

pub(super) fn cleanup_job_tool_specs() -> Vec<Value> {
    vec![
        json!({
            "name": "background_shell_list",
            "description": "List wrapper-owned background shell jobs with their current status.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "background_shell_terminate",
            "description": "Terminate a running background shell job by jobId, alias, or @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_clean",
            "description": "Terminate local background shell jobs by scope. Supports all, blockers, shells, or services. Blocker cleanup can optionally target one @capability to clear only prerequisite shells gated on that reusable role, and service cleanup can optionally target one @capability to resolve ambiguous reusable roles.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": {
                        "type": "string",
                        "enum": ["all", "blockers", "shells", "services"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
    ]
}
