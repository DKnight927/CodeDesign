//! cd-tools — tool schemas + pure handlers for the P3.1 design-specific tool set.
//!
//! Each submodule owns ONE tool's JSON schema and its pure handler.
//! The CLI / agent loop wires schemas into the model's tool surface
//! and routes `tool_call` payloads to the matching handler.
//!
//! Scope of this crate:
//!   - pure, side-effect-free logic where possible
//!   - disk I/O only for `todo` (session state file)
//!   - NO LLM calls, NO network, NO Figma round-trips here; the
//!     Figma-backed tools (`figma.selection`, `open_figma_node`)
//!     expose their schema and delegate execution to cd-figma-mcp.

pub mod ds_query;
pub mod ds_validate;
pub mod figma_nav;
pub mod prd_parse;
pub mod todo;

pub use ds_query::{ds_query_schema, handle_ds_query, DsQueryResult};
pub use ds_validate::{ds_validate_schema, handle_ds_validate, DsValidateReport};
pub use figma_nav::{
    figma_selection_schema, format_figma_deeplink, open_figma_node_schema,
};
pub use prd_parse::{handle_prd_parse, prd_parse_schema, PrdParseResult};
pub use todo::{handle_todo, todo_schema, TodoItem, TodoList, TodoStatus, TodoStore};

pub const CRATE_NAME: &str = "cd-tools";

/// All tool schemas in one place. Handy for tests / prompt-layer wiring.
#[must_use]
pub fn all_schemas() -> Vec<serde_json::Value> {
    vec![
        todo_schema(),
        ds_query_schema(),
        ds_validate_schema(),
        prd_parse_schema(),
        open_figma_node_schema(),
        figma_selection_schema(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-tools");
    }

    #[test]
    fn all_schemas_are_functions() {
        let s = all_schemas();
        assert_eq!(s.len(), 6);
        for schema in s {
            assert_eq!(schema["type"], "function");
            assert!(schema["function"]["name"].is_string());
            assert!(schema["function"]["parameters"].is_object());
        }
    }
}
