use serde_json::Value;
use serde_json::json;

const MAX_RESULTS: usize = 100;

pub(crate) fn workspace_tool_specs() -> Vec<Value> {
    vec![
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
    ]
}
