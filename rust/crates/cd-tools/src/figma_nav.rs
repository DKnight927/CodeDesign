//! Figma navigation tool schemas. Runtime execution goes through
//! cd-figma-mcp; this module owns the schemas and a pure deeplink
//! formatter for `open_figma_node`.

use serde_json::{json, Value};

/// JSON schema for the `open_figma_node` tool.
#[must_use]
pub fn open_figma_node_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "open_figma_node",
            "description": "Emit a deeplink that opens the given node in the current Figma file. Use at the end of a report so the user can jump there in Desktop.",
            "parameters": {
                "type": "object",
                "required": ["fileKey", "nodeId"],
                "properties": {
                    "fileKey": {"type": "string"},
                    "nodeId":  {"type": "string", "description": "Figma node id (e.g. `1:234` or `I12:34`)"},
                    "label":   {"type": "string", "description": "optional short label shown next to the link"}
                },
                "additionalProperties": false
            }
        }
    })
}

/// JSON schema for the `figma.selection` tool (read-only).
#[must_use]
pub fn figma_selection_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "figma.selection",
            "description": "Read the user's current Figma selection. Returns an array of nodeIds. No write. Use to scope `refine` or `critique` to the selection.",
            "parameters": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }
    })
}

/// Format a Figma node deeplink. Figma URL shape:
///   https://www.figma.com/design/<fileKey>/?node-id=<urlNodeId>
/// where nodeId `1:234` becomes `1-234` in the URL (colon encoded as hyphen).
#[must_use]
pub fn format_figma_deeplink(file_key: &str, node_id: &str) -> String {
    let url_node = node_id.replace(':', "-");
    format!(
        "https://www.figma.com/design/{}/?node-id={}",
        file_key, url_node
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeplink_encodes_node_id() {
        let u = format_figma_deeplink("ABC123", "1:234");
        assert!(u.contains("node-id=1-234"));
        assert!(u.starts_with("https://www.figma.com/design/ABC123/"));
    }

    #[test]
    fn open_schema_requires_file_and_node() {
        let s = open_figma_node_schema();
        assert_eq!(s["function"]["parameters"]["required"], json!(["fileKey","nodeId"]));
    }

    #[test]
    fn selection_schema_has_no_required() {
        let s = figma_selection_schema();
        assert_eq!(s["function"]["parameters"]["properties"], json!({}));
    }
}
