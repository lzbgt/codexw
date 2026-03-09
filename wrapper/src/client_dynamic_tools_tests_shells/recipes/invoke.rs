use super::super::BackgroundShellManager;
use super::super::execute_dynamic_tool_call;
use super::super::json;

#[path = "invoke/http.rs"]
mod http;
#[path = "invoke/socket.rs"]
mod socket;
#[path = "invoke/stdin.rs"]
mod stdin;
