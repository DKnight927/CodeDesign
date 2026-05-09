//! MCP client — high-level operations layered on the stdio transport.
//!
//! We implement only the subset CodeDesign needs:
//!   * `initialize`        (handshake + capability exchange)
//!   * `notifications/initialized`
//!   * `tools/list`
//!   * `tools/call`
//!
//! Resources, prompts, sampling, roots, and logging are out of scope
//! for v0.0.3 — CodeDesign's contract with Figma is: "hand me a tool
//! named `figma_execute`". Anything else is a courtesy.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth;
use crate::error::{Error, Result};
use crate::transport::StdioTransport;

/// MCP protocol version we declare on `initialize`. This is the
/// 2025-03-26 Streamable-HTTP spec revision, which is also what
/// contemporary Node MCP servers accept on stdio.
pub const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// Advertised client identity. Surfaces in server logs and lets us
/// tell ourselves apart from other clients during support triage.
pub const CLIENT_NAME: &str = "codedesign";

/// Minimal info about an exposed tool returned by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Raw inputSchema JSON, if provided. We don't try to interpret
    /// it — callers that need schema enforcement should validate
    /// against it themselves.
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Option<Value>,
}

/// High-level MCP operations CodeDesign needs from any Figma backend.
/// Implemented today by [`ConsoleBridgeClient`]; future
/// `RemoteMcpClient` / `DesktopMcpClient` will implement the same
/// trait so the rest of the codebase does not change when we gain
/// Catalog access.
pub trait McpClient {
    fn list_tools(&mut self) -> Result<Vec<ToolInfo>>;

    /// Call a tool and return the flattened text content from its
    /// `content` array, or the raw JSON if the server returned
    /// structured output.
    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value>;

    /// Convenience: assert a tool exists and return its schema.
    fn require_tool(&mut self, name: &str) -> Result<ToolInfo> {
        let tools = self.list_tools()?;
        tools
            .into_iter()
            .find(|t| t.name == name)
            .ok_or_else(|| Error::ToolUnavailable(name.to_string()))
    }

    /// Graceful shutdown. After calling this, the client is dead.
    fn shutdown(self: Box<Self>);
}

/// Backend: spawns `npx figma-console-mcp@latest` locally and speaks
/// MCP over stdio. Requires a Figma PAT either in the env or in
/// `~/.codedesign/auth.toml`.
pub struct ConsoleBridgeClient {
    transport: StdioTransport,
    server_info: Option<Value>,
}

/// Config for spawning the figma-console-mcp subprocess.
#[derive(Debug, Clone)]
pub struct ConsoleBridgeConfig {
    /// Binary used to run the server. `"npx"` by default; override
    /// for offline testing or when running from a local clone.
    pub program: String,
    /// Arguments passed after the program. Defaults to running
    /// `figma-console-mcp@latest` via npx with `--yes` so the first
    /// invocation does not prompt.
    pub args: Vec<String>,
    /// Figma PAT to inject as `FIGMA_ACCESS_TOKEN`. If `None`, we
    /// resolve it via [`auth::load_figma_token`].
    pub figma_token: Option<String>,
}

impl Default for ConsoleBridgeConfig {
    fn default() -> Self {
        Self {
            program: "npx".into(),
            args: vec![
                "--yes".into(),
                "figma-console-mcp@latest".into(),
            ],
            figma_token: None,
        }
    }
}

impl ConsoleBridgeClient {
    /// Spawn the server and complete the `initialize` handshake.
    pub fn connect(cfg: ConsoleBridgeConfig) -> Result<Self> {
        let token = match cfg.figma_token {
            Some(t) => t,
            None => auth::load_figma_token()?,
        };

        let args_ref: Vec<&str> = cfg.args.iter().map(String::as_str).collect();
        let env: [(&str, &str); 1] = [("FIGMA_ACCESS_TOKEN", token.as_str())];

        let mut transport = StdioTransport::spawn(&cfg.program, &args_ref, &env)?;

        // initialize
        let init_params = json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "clientInfo": {
                "name": CLIENT_NAME,
                "version": env!("CARGO_PKG_VERSION"),
            },
        });
        let init_result = transport.request("initialize", init_params)?;
        let server_info = init_result.get("serverInfo").cloned();

        // Per MCP spec, client MUST send this after init result.
        transport.notify("notifications/initialized", json!({}))?;

        Ok(Self {
            transport,
            server_info,
        })
    }

    /// Info the server reported on `initialize`. Handy for `doctor`.
    pub fn server_info(&self) -> Option<&Value> {
        self.server_info.as_ref()
    }
}

impl McpClient for ConsoleBridgeClient {
    fn list_tools(&mut self) -> Result<Vec<ToolInfo>> {
        let result = self.transport.request("tools/list", json!({}))?;
        let arr = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Protocol("tools/list missing `tools` array".into()))?;
        let mut out = Vec::with_capacity(arr.len());
        for t in arr {
            let info: ToolInfo = serde_json::from_value(t.clone())?;
            out.push(info);
        }
        Ok(out)
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let result = self
            .transport
            .request("tools/call", json!({ "name": name, "arguments": arguments }))?;

        // MCP spec: `isError: true` means the tool itself failed,
        // even though the RPC succeeded. We surface this as an error
        // so callers don't silently ignore failed Plugin-JS runs.
        if result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let msg = extract_text_content(&result).unwrap_or_else(|| result.to_string());
            return Err(Error::ToolError(msg));
        }
        Ok(result)
    }

    fn shutdown(self: Box<Self>) {
        self.transport.shutdown();
    }
}

/// Pull the concatenated text from an MCP tool result's `content`
/// array. Non-text content blocks are ignored.
pub fn extract_text_content(tool_result: &Value) -> Option<String> {
    let content = tool_result.get("content")?.as_array()?;
    let mut buf = String::new();
    for c in content {
        if c.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(t) = c.get("text").and_then(Value::as_str) {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(t);
            }
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_content_concatenates_blocks() {
        let v = json!({
            "content": [
                {"type": "text", "text": "one"},
                {"type": "image", "data": "..."},
                {"type": "text", "text": "two"},
            ]
        });
        assert_eq!(extract_text_content(&v).as_deref(), Some("one\ntwo"));
    }

    #[test]
    fn extract_text_content_none_when_empty() {
        assert!(extract_text_content(&json!({"content": []})).is_none());
        assert!(extract_text_content(&json!({})).is_none());
    }

    #[test]
    fn tool_info_parses_from_server_shape() {
        let v = json!({
            "name": "figma_execute",
            "description": "Run Plugin API JS.",
            "inputSchema": {"type": "object"}
        });
        let t: ToolInfo = serde_json::from_value(v).unwrap();
        assert_eq!(t.name, "figma_execute");
        assert_eq!(t.description.as_deref(), Some("Run Plugin API JS."));
        assert!(t.input_schema.is_some());
    }
}
