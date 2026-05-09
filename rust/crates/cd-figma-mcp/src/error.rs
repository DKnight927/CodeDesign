//! Error types for cd-figma-mcp.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("toml decode: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml encode: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// Server returned a JSON-RPC error object.
    #[error("mcp rpc error {code}: {message}")]
    Rpc { code: i64, message: String },

    /// Response JSON was well-formed but not shaped the way we expected.
    #[error("mcp protocol: {0}")]
    Protocol(String),

    /// Child subprocess (npx figma-console-mcp) failed to start or exited.
    #[error("subprocess: {0}")]
    Subprocess(String),

    /// No Figma PAT found in env or config.
    #[error("auth: {0}")]
    Auth(String),

    /// Request timed out before the server responded.
    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Required MCP tool not exposed by the server.
    #[error("tool not available: {0}")]
    ToolUnavailable(String),

    /// `tools/call` returned `isError: true`.
    #[error("tool execution error: {0}")]
    ToolError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
