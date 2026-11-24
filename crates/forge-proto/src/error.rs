#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    // Forge OS application error codes (Blueprint 6.5.2)
    NotFound = 1001,
    Unsigned = 1002,
    InsufficientDisk = 1003,
    TierWarningUnacknowledged = 1004,
    BackendFailed = 1005,
    RollbackUnavailable = 1006,
    PermissionDenied = 1007,
    Conflict = 1008,
}

impl RpcErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    pub fn new(code: RpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_i32(),
            message: message.into(),
        }
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}
