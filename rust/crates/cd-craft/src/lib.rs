//! cd-craft — loader for `craft/*.md` (rules) and `craft/directions/*.json`
//! (visual-direction presets for the VDP picker).
//!
//! Craft rules are bundled with the binary; a project may shadow them by
//! placing a same-named file under `./.codedesign/craft/`. This crate is
//! intentionally small — its job is to hand deterministic text blobs and
//! direction metadata to cd-prompts.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

pub const CRATE_NAME: &str = "cd-craft";

// ── craft rule files ──────────────────────────────────────────────────────

/// Markdown rule files bundled into the binary. Keys are the stem of the
/// file (e.g. `"accessibility-baseline"`). Values are the raw markdown.
#[must_use]
pub fn bundled_rules() -> BTreeMap<&'static str, &'static str> {
    let mut m = BTreeMap::new();
    m.insert("accessibility-baseline", include_str!("../../../assets/craft/accessibility-baseline.md"));
    m.insert("animation-discipline",   include_str!("../../../assets/craft/animation-discipline.md"));
    m.insert("anti-ai-slop",           include_str!("../../../assets/craft/anti-ai-slop.md"));
    m.insert("color",                  include_str!("../../../assets/craft/color.md"));
    m.insert("form-validation",        include_str!("../../../assets/craft/form-validation.md"));
    m.insert("laws-of-ux",             include_str!("../../../assets/craft/laws-of-ux.md"));
    m.insert("rtl-and-bidi",           include_str!("../../../assets/craft/rtl-and-bidi.md"));
    m.insert("spacing",                include_str!("../../../assets/craft/spacing.md"));
    m.insert("state-coverage",         include_str!("../../../assets/craft/state-coverage.md"));
    m.insert("typography",             include_str!("../../../assets/craft/typography.md"));
    m.insert("typography-hierarchy",   include_str!("../../../assets/craft/typography-hierarchy.md"));
    m.insert("typography-hierarchy-editorial", include_str!("../../../assets/craft/typography-hierarchy-editorial.md"));
    m
}

/// Resolve a required-rule name against a project-local override directory.
/// Returns the first hit in: `{override_dir}/{name}.md` → bundled.
pub fn rule_text(name: &str, override_dir: Option<&Path>) -> Option<String> {
    if let Some(dir) = override_dir {
        let p = dir.join(format!("{name}.md"));
        if let Ok(s) = std::fs::read_to_string(&p) {
            return Some(s);
        }
    }
    bundled_rules().get(name).map(|s| (*s).to_owned())
}

// ── visual direction picker ───────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Direction {
    pub id: String,
    pub name: String,
    #[serde(rename = "oneLine", default)]
    pub one_line: String,
    #[serde(default)]
    pub posture: serde_json::Value,
    #[serde(default)]
    pub palette: serde_json::Value,
    #[serde(default)]
    pub typography: serde_json::Value,
    #[serde(rename = "spacingBase", default)]
    pub spacing_base: Option<u32>,
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Error)]
pub enum DirectionError {
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("direction `{0}` not found")]
    NotFound(String),
}

/// All bundled directions, keyed by id, in insertion order.
#[must_use]
pub fn bundled_directions() -> Vec<Direction> {
    const FILES: &[(&str, &str)] = &[
        ("brutalist",       include_str!("../../../assets/craft/directions/brutalist.json")),
        ("neutral-modern",  include_str!("../../../assets/craft/directions/neutral-modern.json")),
        ("soft-playful",    include_str!("../../../assets/craft/directions/soft-playful.json")),
        ("tech-utility",    include_str!("../../../assets/craft/directions/tech-utility.json")),
        ("warm-editorial",  include_str!("../../../assets/craft/directions/warm-editorial.json")),
    ];
    FILES
        .iter()
        .filter_map(|(_, raw)| serde_json::from_str(raw).ok())
        .collect()
}

/// One-line summary of each direction, for the VDP picker presented to
/// the user in Turn-1. Deterministic order.
#[must_use]
pub fn direction_summaries() -> Vec<(String, String)> {
    bundled_directions()
        .into_iter()
        .map(|d| (d.id, d.one_line))
        .collect()
}

/// Fetch a direction by id.
pub fn direction(id: &str) -> Result<Direction, DirectionError> {
    bundled_directions()
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| DirectionError::NotFound(id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-craft");
    }

    #[test]
    fn bundled_rules_complete() {
        let r = bundled_rules();
        for key in [
            "accessibility-baseline",
            "animation-discipline",
            "anti-ai-slop",
            "form-validation",
            "laws-of-ux",
            "rtl-and-bidi",
            "typography-hierarchy",
            "typography-hierarchy-editorial",
        ] {
            assert!(r.contains_key(key), "missing rule file: {key}");
            assert!(!r[key].trim().is_empty());
        }
    }

    #[test]
    fn rule_text_fallback_to_bundled() {
        let t = rule_text("anti-ai-slop", None).unwrap();
        assert!(!t.is_empty());
    }

    #[test]
    fn bundled_directions_has_five() {
        let d = bundled_directions();
        assert_eq!(d.len(), 5);
        assert!(d.iter().any(|x| x.id == "neutral-modern"));
    }

    #[test]
    fn direction_summaries_ordered() {
        let s = direction_summaries();
        assert_eq!(s.len(), 5);
        assert!(s.iter().all(|(_, l)| !l.is_empty()));
    }

    #[test]
    fn direction_lookup_found_and_missing() {
        assert!(direction("tech-utility").is_ok());
        assert!(direction("nope").is_err());
    }
}
