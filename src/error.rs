//! Plugin-local error type. Keep deps light — no `anyhow`/`thiserror` by default.

use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
}

#[derive(Debug)]
pub struct PluginError {
    pub code: ErrorCode,
    pub message: String,
}

impl PluginError {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: msg.into(),
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidParams,
            message: msg.into(),
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for PluginError {}

impl ErrorCode {
    pub fn value(&self) -> i64 {
        self.clone() as i64
    }
}

impl From<elasticsearch::Error> for PluginError {
    fn from(err: elasticsearch::Error) -> Self {
        Self::internal(format!("elasticsearch error: {:?}", err))
    }
}
