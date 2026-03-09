use serde_json::Value;
use serde_json::json;

const MAX_RESULTS: usize = 100;

pub(crate) fn dynamic_tool_specs() -> Value {
    Value::Array(vec![
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
        json!({
            "name": "workspace_list_dir",
            "description": "List files and directories under a workspace directory. Defaults to the workspace root.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                }
            }
        }),
        json!({
            "name": "workspace_stat_path",
            "description": "Inspect a workspace path and report whether it is a file or directory, plus basic metadata.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "workspace_read_file",
            "description": "Read a UTF-8 text file from the current workspace. Supports optional 1-based startLine and endLine filters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "startLine": {"type": "integer", "minimum": 1},
                    "endLine": {"type": "integer", "minimum": 1}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "workspace_find_files",
            "description": "Find workspace file paths whose relative path contains the given query substring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "workspace_search_text",
            "description": "Search UTF-8 text files in the current workspace for lines containing the given query substring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                },
                "required": ["query"]
            }
        }),
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
            "description": "Inspect a background shell job by jobId, alias, or @capability and fetch new output lines since an optional afterLine cursor.",
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
    ])
}
