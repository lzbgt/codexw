use serde_json::Value;
use serde_json::json;

pub(super) fn service_tool_specs() -> Vec<Value> {
    vec![
        json!({
            "name": "background_shell_list_capabilities",
            "description": "List the reusable service capability registry, optionally filtered to healthy, missing, booting, untracked, or ambiguous capability states.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["all", "healthy", "missing", "booting", "untracked", "ambiguous"]
                    }
                }
            }
        }),
        json!({
            "name": "background_shell_list_services",
            "description": "List reusable service shell jobs, optionally filtered to ready, booting, untracked, or conflicting services and optionally narrowed to one @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["all", "ready", "booting", "untracked", "conflicts"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "background_shell_update_service",
            "description": "Update mutable metadata for a running service background shell job by jobId, alias, or @capability. Supports replacing or clearing declared reusable capabilities and updating or clearing the live service label, protocol, endpoint, attach hint, readyPattern, and structured recipes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "label": {
                        "type": ["string", "null"]
                    },
                    "protocol": {
                        "type": ["string", "null"]
                    },
                    "endpoint": {
                        "type": ["string", "null"]
                    },
                    "attachHint": {
                        "type": ["string", "null"]
                    },
                    "readyPattern": {
                        "type": ["string", "null"]
                    },
                    "capabilities": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "recipes": {
                        "type": ["array", "null"],
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
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_update_dependencies",
            "description": "Update or clear the declared dependsOnCapabilities set for a running background shell job by jobId, alias, or @capability. This retargets orchestration dependency edges without restarting the underlying job.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "dependsOnCapabilities": {
                        "type": ["array", "null"],
                        "items": {"type": "string"}
                    }
                },
                "required": ["jobId", "dependsOnCapabilities"]
            }
        }),
        json!({
            "name": "background_shell_inspect_capability",
            "description": "Inspect one reusable service capability and show its current providers, provider metadata, and consumers. Accepts `capability` with or without the leading @.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "capability": {"type": "string"}
                },
                "required": ["capability"]
            }
        }),
        json!({
            "name": "background_shell_attach",
            "description": "Show structured attachment metadata for a service background shell job by jobId, alias, or @capability, including endpoint, capabilities, and attach hints when declared.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_wait_ready",
            "description": "Wait for a service background shell job with a declared readyPattern to become ready. Supports jobId, alias, or @capability references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "timeoutMs": {"type": "integer", "minimum": 0}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_invoke_recipe",
            "description": "Invoke a structured recipe declared by a service background shell job. Supports jobId, alias, or @capability references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "recipe": {"type": "string"},
                    "waitForReadyMs": {"type": "integer", "minimum": 0},
                    "args": {
                        "type": "object",
                        "additionalProperties": {
                            "type": ["string", "number", "boolean"]
                        }
                    }
                },
                "required": ["jobId", "recipe"]
            }
        }),
    ]
}
