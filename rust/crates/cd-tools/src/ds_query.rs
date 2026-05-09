//! `ds.query` — fuzzy lookup in the active DesignSystem (tokens/components).
//!
//! Pure over (DS, query). Returns ranked results with a confidence
//! heuristic. No external calls.

use serde::Serialize;
use serde_json::{json, Value};

use cd_ds::DesignSystem;

#[derive(Debug, Clone, Serialize)]
pub struct DsQueryHit {
    pub kind: &'static str, // "token" | "component"
    #[serde(rename = "ref")]
    pub reference: String,
    pub value: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DsQueryResult {
    pub query: String,
    pub hits: Vec<DsQueryHit>,
}

/// JSON schema for the `ds.query` tool.
#[must_use]
pub fn ds_query_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "ds.query",
            "description": "Fuzzy-lookup tokens and components in the active DesignSystem. Returns ranked hits with confidence scores. Use before inventing a new ref.",
            "parameters": {
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {"type": "string", "description": "free-text query, e.g. `primary cta` or `space md`"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 20, "default": 8},
                    "kind":  {"type": "string", "enum": ["token","component","any"], "default": "any"}
                },
                "additionalProperties": false
            }
        }
    })
}

/// Pure handler. `limit` default 8, `kind` default "any".
#[must_use]
pub fn handle_ds_query(ds: &DesignSystem, query: &str, limit: usize, kind: &str) -> DsQueryResult {
    let q = query.to_ascii_lowercase();
    let mut hits: Vec<DsQueryHit> = Vec::new();

    if kind != "component" {
        for tok_ref in ds.tokens.all_refs() {
            let s = score(&q, &tok_ref.to_ascii_lowercase());
            if s > 0.0 {
                let value = ds.value_for_ref(&tok_ref).unwrap_or_default();
                hits.push(DsQueryHit {
                    kind: "token",
                    reference: tok_ref,
                    value,
                    score: s,
                });
            }
        }
    }
    if kind != "token" {
        for comp in ds.component_names() {
            let s = score(&q, &comp.to_ascii_lowercase());
            if s > 0.0 {
                hits.push(DsQueryHit {
                    kind: "component",
                    reference: comp.clone(),
                    value: comp,
                    score: s,
                });
            }
        }
    }

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(limit);

    DsQueryResult { query: query.to_owned(), hits }
}

fn score(query: &str, candidate: &str) -> f32 {
    if candidate == query {
        return 1.0;
    }
    if candidate.contains(query) {
        // Longer substring match scores higher.
        let ratio = query.len() as f32 / candidate.len() as f32;
        return 0.5 + 0.5 * ratio;
    }
    // Token-level overlap: split on '.' and whitespace.
    let qt: Vec<&str> = query.split(|c: char| c == '.' || c.is_whitespace()).filter(|s| !s.is_empty()).collect();
    let ct: Vec<&str> = candidate.split(|c: char| c == '.' || c.is_whitespace()).filter(|s| !s.is_empty()).collect();
    if qt.is_empty() || ct.is_empty() {
        return 0.0;
    }
    let hits = qt.iter().filter(|t| ct.iter().any(|c| c.contains(*t))).count();
    if hits == 0 {
        return 0.0;
    }
    0.3 * (hits as f32 / qt.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds() -> DesignSystem {
        DesignSystem::bundled_default().unwrap()
    }

    #[test]
    fn exact_token_ref_scores_highest() {
        let r = handle_ds_query(&ds(), "color.brand.primary", 5, "any");
        assert!(r.hits.iter().any(|h| h.reference == "color.brand.primary" && h.score >= 0.9));
    }

    #[test]
    fn component_kind_filter_works() {
        let r = handle_ds_query(&ds(), "button", 5, "component");
        assert!(!r.hits.is_empty());
        assert!(r.hits.iter().all(|h| h.kind == "component"));
    }

    #[test]
    fn limit_respected() {
        let r = handle_ds_query(&ds(), "color", 2, "any");
        assert!(r.hits.len() <= 2);
    }

    #[test]
    fn empty_query_no_hits() {
        let r = handle_ds_query(&ds(), "nonexistentthing", 8, "any");
        assert!(r.hits.is_empty());
    }

    #[test]
    fn schema_has_required_query_field() {
        let s = ds_query_schema();
        let params = &s["function"]["parameters"];
        assert_eq!(params["required"], json!(["query"]));
    }
}
