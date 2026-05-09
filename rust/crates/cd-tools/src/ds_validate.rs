//! `ds.validate` — pre-submit compliance check on a DesignPlan fragment.
//!
//! Given any JSON value containing `fillRef` / `styleRef` /
//! `componentRef` / `padding` / `gap` strings, verify every ref
//! resolves in the DS and follows the section prefix contract.

use serde::Serialize;
use serde_json::{json, Value};

use cd_ds::DesignSystem;

#[derive(Debug, Clone, Serialize)]
pub struct DsValidateFinding {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DsValidateReport {
    pub ok: bool,
    pub findings: Vec<DsValidateFinding>,
}

#[must_use]
pub fn ds_validate_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "ds.validate",
            "description": "Validate refs in a DesignPlan (or fragment) against the active DesignSystem. Returns every unresolved ref with a JSON path. Call this before plan_emit's final submission.",
            "parameters": {
                "type": "object",
                "required": ["fragment"],
                "properties": {
                    "fragment": {"type": "object", "description": "any JSON containing fillRef/styleRef/componentRef/padding/gap strings"}
                },
                "additionalProperties": false
            }
        }
    })
}

#[must_use]
pub fn handle_ds_validate(ds: &DesignSystem, fragment: &Value) -> DsValidateReport {
    let mut out = Vec::new();
    walk(ds, fragment, String::new(), &mut out);
    DsValidateReport { ok: out.is_empty(), findings: out }
}

fn walk(ds: &DesignSystem, v: &Value, path: String, out: &mut Vec<DsValidateFinding>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                if let Value::String(s) = child {
                    check_ref(ds, k, s, &p, out);
                } else {
                    walk(ds, child, p, out);
                }
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                walk(ds, item, format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

fn check_ref(ds: &DesignSystem, key: &str, val: &str, path: &str, out: &mut Vec<DsValidateFinding>) {
    match key {
        "fillRef" => {
            if !val.starts_with("color.") {
                out.push(finding("DSV-PREFIX-001", path, format!("fillRef `{val}` must start with `color.`")));
            } else if !ds.knows_ref(val) {
                out.push(finding("DSV-UNRESOLVED", path, format!("fillRef `{val}` does not resolve in DS")));
            }
        }
        "styleRef" => {
            if !val.starts_with("typo.") {
                out.push(finding("DSV-PREFIX-002", path, format!("styleRef `{val}` must start with `typo.`")));
            } else if !ds.knows_ref(val) {
                out.push(finding("DSV-UNRESOLVED", path, format!("styleRef `{val}` does not resolve in DS")));
            }
        }
        "padding" | "gap" => {
            if !val.starts_with("space.") {
                out.push(finding("DSV-PREFIX-003", path, format!("{key} `{val}` must start with `space.`")));
            } else if !ds.knows_ref(val) {
                out.push(finding("DSV-UNRESOLVED", path, format!("{key} `{val}` does not resolve in DS")));
            }
        }
        "componentRef" => {
            if !ds.component_names().iter().any(|n| n == val) {
                out.push(finding("DSV-UNKNOWN-COMPONENT", path, format!("componentRef `{val}` is not a DS component")));
            }
        }
        _ => {}
    }
}

fn finding(code: &'static str, path: &str, message: String) -> DsValidateFinding {
    DsValidateFinding { code, path: path.to_owned(), message }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds() -> DesignSystem {
        DesignSystem::bundled_default().unwrap()
    }

    #[test]
    fn clean_fragment_is_ok() {
        let frag = json!({
            "fillRef": "color.brand.primary",
            "styleRef": "typo.body.md",
            "padding": "space.md",
            "componentRef": "button.primary"
        });
        let r = handle_ds_validate(&ds(), &frag);
        assert!(r.ok, "{:?}", r.findings);
    }

    #[test]
    fn unresolved_fill_detected() {
        let frag = json!({"fillRef": "color.nonexistent"});
        let r = handle_ds_validate(&ds(), &frag);
        assert!(!r.ok);
        assert!(r.findings.iter().any(|f| f.code == "DSV-UNRESOLVED"));
    }

    #[test]
    fn prefix_enforced() {
        let frag = json!({"padding": "md"});
        let r = handle_ds_validate(&ds(), &frag);
        assert!(r.findings.iter().any(|f| f.code == "DSV-PREFIX-003"));
    }

    #[test]
    fn unknown_component_detected() {
        let frag = json!({"componentRef": "button.fancy"});
        let r = handle_ds_validate(&ds(), &frag);
        assert!(r.findings.iter().any(|f| f.code == "DSV-UNKNOWN-COMPONENT"));
    }

    #[test]
    fn nested_path_reported() {
        let frag = json!({"frames": [{"fillRef": "nope"}]});
        let r = handle_ds_validate(&ds(), &frag);
        assert!(r.findings[0].path.contains("frames[0].fillRef"));
    }
}
