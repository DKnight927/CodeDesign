//! cd-figma-mcp — real MCP client for Figma.
//!
//! CodeDesign's sole write path into Figma. Per the
//! `project_figma_mcp_entry` memory, the primary backend is
//! [`figma-console-mcp`](https://github.com/southleft/figma-console-mcp)
//! (MIT, community) because the official Figma Remote MCP is gated by
//! a Figma-maintained Catalog allowlist that does not include us, and
//! Figma Desktop MCP has no write tools.
//!
//! This crate is the *client*: it spawns `npx figma-console-mcp@latest`
//! as a child process, speaks MCP over stdio JSON-RPC, and exposes a
//! narrow high-level API used by `cd-cli`:
//!
//!   * [`ConsoleBridgeClient::connect`] — spawn + `initialize` handshake
//!   * [`McpClient::list_tools`]        — discover tools
//!   * [`McpClient::call_tool`]         — invoke one, e.g. `figma_execute`
//!
//! The IR → Plugin-JS compiler stays in `cd-canvas`; this crate never
//! parses the Plan. It is a pure transport layer so the backend can be
//! swapped later (Remote MCP, Desktop MCP) without touching the seam.

pub mod auth;
pub mod client;
pub mod error;
pub mod transport;

pub use client::{ConsoleBridgeClient, ConsoleBridgeConfig, McpClient, ToolInfo};
pub use error::{Error, Result};

/// Runtime shim that MUST be prepended to any Plugin-JS snippet
/// executed via `figma_execute`. It resolves default-DS refs into
/// concrete Figma primitives. Owned by this crate — not by the
/// compiler — because the shim is a property of *executing* IR in
/// Figma, not of *emitting* it.
pub const RUNTIME_SHIM_JS: &str = include_str!("runtime_shim.js");

/// Wrap a compiled Product JS snippet with the runtime shim. Calling
/// this is the only supported way to produce the string handed to
/// `figma_execute`.
pub fn bundle_plugin_js(product_js: &str) -> String {
    format!("{RUNTIME_SHIM_JS}\n{product_js}")
}

/// Name of the figma-console-mcp tool that runs arbitrary Plugin API JS.
pub const TOOL_FIGMA_EXECUTE: &str = "figma_execute";

/// Crate name constant; used in smoke tests to prove wiring across the workspace.
pub const CRATE_NAME: &str = "cd-figma-mcp";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-figma-mcp");
    }
}
