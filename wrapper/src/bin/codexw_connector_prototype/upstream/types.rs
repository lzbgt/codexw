use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct UpstreamResponse {
    pub(crate) status: u16,
    pub(crate) reason: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug)]
pub(crate) enum ForwardRequestError {
    Validation {
        message: String,
        details: Option<Value>,
    },
    Transport(anyhow::Error),
}

impl ForwardRequestError {
    pub(crate) fn validation(message: impl Into<String>, details: Option<Value>) -> Self {
        Self::Validation {
            message: message.into(),
            details,
        }
    }
}
