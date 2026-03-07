use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Integer(i64),
    String(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum IncomingMessage {
    Response(RpcResponse),
    Request(RpcRequest),
    Notification(RpcNotification),
}

#[derive(Debug, Deserialize)]
pub struct RpcResponse {
    pub id: RequestId,
    #[serde(default)]
    pub result: Value,
    #[serde(default)]
    pub error: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub id: RequestId,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Deserialize)]
pub struct RpcNotification {
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct OutgoingRequest<'a> {
    pub id: RequestId,
    pub method: &'a str,
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct OutgoingNotification<'a> {
    pub method: &'a str,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct OutgoingResponse {
    pub id: RequestId,
    pub result: Value,
}

#[derive(Debug, Serialize)]
pub struct OutgoingErrorResponse {
    pub id: RequestId,
    pub error: OutgoingErrorObject,
}

#[derive(Debug, Serialize)]
pub struct OutgoingErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub fn parse_line(line: &str) -> Result<IncomingMessage> {
    serde_json::from_str(line).with_context(|| format!("parse JSON-RPC line: {line}"))
}
