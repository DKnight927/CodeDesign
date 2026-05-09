//! cd-critique — deterministic anti-slop lint over a Figma readback JSON.
//!
//! Input shape is the readback returned by `figma_execute` (Figma Plugin
//! API scenegraph serialisation). We walk it recursively and emit a
//! stable list of [`Finding`]s. Rules live in `anti-ai-slop.json`
//! (asset) and may be overridden per-project.
//!
//! Every rule is pure and deterministic — no LLM calls. This is the
//! floor that every generation must pass before we hand the readback
//! to the (LLM-backed) Critic role for judgement.

pub mod rubric;
pub mod rules;

use serde::Serialize;
use serde_json::Value;

pub use rubric::{
    critic_emit_schema, validate_report, Band, CritiqueReport, CritiqueScores, Dimension,
    DimensionScore, RubricError, RubricFinding, BANNED_EVIDENCE_PHRASES, RUBRIC_MD,
};
pub use rules::{AntiSlopRules, RulesError};

pub const CRATE_NAME: &str = "cd-critique";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
    Info,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub id: &'static str,
    pub message: String,
    pub fix: String,
    /// Short JSON-path breadcrumb into the readback, e.g.
    /// `"pages[0].children[2].fills[0].color"`.
    pub path: String,
    /// Offending substring / rendered value, trimmed to 120 chars.
    pub snippet: String,
}

/// Run every rule over `readback` and return findings in stable order.
///
/// Order is: traversal-order × rule-order-within-node. Re-running the
/// lint on the same input always produces the same vector.
#[must_use]
pub fn lint(readback: &Value, rules: &AntiSlopRules) -> Vec<Finding> {
    let mut findings = Vec::new();
    walk(readback, String::new(), rules, &mut findings);
    findings
}

// ── traversal ──────────────────────────────────────────────────────────────

fn walk(v: &Value, path: String, rules: &AntiSlopRules, out: &mut Vec<Finding>) {
    match v {
        Value::Object(map) => {
            // per-node rules: inspect well-known keys first
            if let Some(chars) = map.get("characters").and_then(Value::as_str) {
                let p = child_path(&path, "characters");
                check_text(chars, &p, rules, out);
            }
            if let Some(name) = map.get("name").and_then(Value::as_str) {
                // names are often decorative ("✨ Sparkle") — scan for banned emoji only
                let p = child_path(&path, "name");
                check_name_emoji(name, &p, rules, out);
            }
            if let Some(fills) = map.get("fills") {
                let p = child_path(&path, "fills");
                check_fills(fills, &p, rules, out);
            }
            if let Some(font) = map.get("fontName").and_then(Value::as_object) {
                if let Some(family) = font.get("family").and_then(Value::as_str) {
                    let p = child_path(&path, "fontName.family");
                    check_font(family, &p, rules, out);
                }
            }
            // recurse everywhere (handles children / pages / frames / etc.)
            for (k, child) in map {
                if matches!(k.as_str(), "characters" | "fills" | "fontName" | "name") {
                    continue;
                }
                walk(child, child_path(&path, k), rules, out);
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                walk(item, index_path(&path, i), rules, out);
            }
        }
        _ => {}
    }
}

fn child_path(base: &str, key: &str) -> String {
    if base.is_empty() { key.to_owned() } else { format!("{base}.{key}") }
}
fn index_path(base: &str, i: usize) -> String {
    format!("{base}[{i}]")
}

// ── rule checks ────────────────────────────────────────────────────────────

fn check_text(text: &str, path: &str, rules: &AntiSlopRules, out: &mut Vec<Finding>) {
    // banned emoji in functional role
    for emo in &rules.banned_emoji {
        if text.contains(emo.as_str()) {
            out.push(Finding {
                severity: Severity::Error,
                id: "SLOP-EMOJI-001",
                message: format!("emoji `{emo}` used in functional text"),
                fix: "remove the emoji; rely on icon tokens + text".into(),
                path: path.into(),
                snippet: snippet(text),
            });
        }
    }
    // placeholder regex
    for re in &rules.placeholder_regex {
        if re.is_match(text) {
            out.push(Finding {
                severity: Severity::Error,
                id: "SLOP-PLACEHOLDER-001",
                message: "placeholder / lorem / feature-numbering detected".into(),
                fix: "replace with domain-specific copy written in the DS voice".into(),
                path: path.into(),
                snippet: snippet(text),
            });
            break;
        }
    }
    // fabricated metrics
    for re in &rules.fabricated_metric_regex {
        if re.is_match(text) {
            out.push(Finding {
                severity: Severity::Warn,
                id: "SLOP-METRIC-001",
                message: "fabricated metric / bold claim detected".into(),
                fix: "remove the claim or cite a source in the readback".into(),
                path: path.into(),
                snippet: snippet(text),
            });
            break;
        }
    }
}

fn check_name_emoji(name: &str, path: &str, rules: &AntiSlopRules, out: &mut Vec<Finding>) {
    for emo in &rules.banned_emoji {
        if name.contains(emo.as_str()) {
            out.push(Finding {
                severity: Severity::Warn,
                id: "SLOP-EMOJI-002",
                message: format!("emoji `{emo}` in node name"),
                fix: "rename the node with a semantic label".into(),
                path: path.into(),
                snippet: snippet(name),
            });
        }
    }
}

fn check_fills(fills: &Value, path: &str, rules: &AntiSlopRules, out: &mut Vec<Finding>) {
    let Some(arr) = fills.as_array() else { return };
    for (i, fill) in arr.iter().enumerate() {
        let Some(obj) = fill.as_object() else { continue };
        if obj.get("type").and_then(Value::as_str) != Some("SOLID") {
            continue;
        }
        let Some(color) = obj.get("color").and_then(Value::as_object) else { continue };
        let r = color.get("r").and_then(Value::as_f64).unwrap_or(-1.0);
        let g = color.get("g").and_then(Value::as_f64).unwrap_or(-1.0);
        let b = color.get("b").and_then(Value::as_f64).unwrap_or(-1.0);
        if r < 0.0 || g < 0.0 || b < 0.0 {
            continue;
        }
        let hex = rgb_to_hex(r, g, b);
        if rules.banned_hex.iter().any(|h| h.eq_ignore_ascii_case(&hex)) {
            out.push(Finding {
                severity: Severity::Error,
                id: "SLOP-HEX-001",
                message: format!("banned AI-purple hex `{hex}` in fill"),
                fix: "bind to a DS colour token (brand.primary / state.*)".into(),
                path: format!("{path}[{i}].color"),
                snippet: hex,
            });
        }
    }
}

fn check_font(family: &str, path: &str, rules: &AntiSlopRules, out: &mut Vec<Finding>) {
    for banned in &rules.banned_display_fonts {
        if family.eq_ignore_ascii_case(banned) {
            out.push(Finding {
                severity: Severity::Warn,
                id: "SLOP-FONT-001",
                message: format!("generic/system font `{family}` used for display"),
                fix: "use a DS-declared display font (e.g. Inter Display)".into(),
                path: path.into(),
                snippet: family.to_owned(),
            });
            break;
        }
    }
}

// ── helpers ────────────────────────────────────────────────────────────────

fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    fn c(x: f64) -> u8 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (x.clamp(0.0, 1.0) * 255.0).round() as i32;
        v.clamp(0, 255) as u8
    }
    format!("#{:02x}{:02x}{:02x}", c(r), c(g), c(b))
}

fn snippet(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= 120 {
        t.to_owned()
    } else {
        let cut: String = t.chars().take(117).collect();
        format!("{cut}...")
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rules() -> AntiSlopRules {
        AntiSlopRules::bundled().unwrap()
    }

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-critique");
    }

    #[test]
    fn clean_readback_no_findings() {
        let rb = json!({
            "type": "FRAME",
            "name": "signup",
            "children": [{
                "type": "TEXT",
                "name": "title",
                "characters": "创建账号开始使用",
                "fills": [{"type": "SOLID", "color": {"r": 0.2, "g": 0.2, "b": 0.2}}],
                "fontName": {"family": "Inter", "style": "Regular"}
            }]
        });
        assert!(lint(&rb, &rules()).is_empty());
    }

    #[test]
    fn detects_banned_hex() {
        let rb = json!({
            "type": "FRAME",
            "fills": [{"type": "SOLID", "color": {"r": 0.388, "g": 0.4, "b": 0.945}}]
        });
        // 0.388 → 99, 0.4 → 102, 0.945 → 241 → #6366f1
        let findings = lint(&rb, &rules());
        assert!(findings.iter().any(|f| f.id == "SLOP-HEX-001"));
    }

    #[test]
    fn does_not_flag_legal_hex() {
        let rb = json!({
            "type": "FRAME",
            "fills": [{"type": "SOLID", "color": {"r": 0.2, "g": 0.4, "b": 0.6}}]
        });
        assert!(lint(&rb, &rules()).iter().all(|f| f.id != "SLOP-HEX-001"));
    }

    #[test]
    fn detects_banned_emoji_in_text() {
        let rb = json!({
            "type": "TEXT",
            "characters": "开始你的旅程 🚀"
        });
        let f = lint(&rb, &rules());
        assert!(f.iter().any(|x| x.id == "SLOP-EMOJI-001"));
    }

    #[test]
    fn does_not_flag_allowed_emoji() {
        let rb = json!({"type": "TEXT", "characters": "下一步 →"});
        let f = lint(&rb, &rules());
        assert!(f.iter().all(|x| !x.id.starts_with("SLOP-EMOJI")));
    }

    #[test]
    fn detects_lorem_placeholder() {
        let rb = json!({"type": "TEXT", "characters": "Lorem ipsum dolor sit amet"});
        let f = lint(&rb, &rules());
        assert!(f.iter().any(|x| x.id == "SLOP-PLACEHOLDER-001"));
    }

    #[test]
    fn detects_feature_numbering() {
        let rb = json!({"type": "TEXT", "characters": "Feature One — fast sync"});
        let f = lint(&rb, &rules());
        assert!(f.iter().any(|x| x.id == "SLOP-PLACEHOLDER-001"));
    }

    #[test]
    fn detects_fabricated_metric() {
        let rb = json!({"type": "TEXT", "characters": "10x faster than the competition"});
        let f = lint(&rb, &rules());
        assert!(f.iter().any(|x| x.id == "SLOP-METRIC-001"));
    }

    #[test]
    fn detects_generic_display_font() {
        let rb = json!({
            "type": "TEXT",
            "characters": "Hi",
            "fontName": {"family": "Helvetica Neue", "style": "Bold"}
        });
        let f = lint(&rb, &rules());
        assert!(f.iter().any(|x| x.id == "SLOP-FONT-001"));
    }

    #[test]
    fn deterministic_order() {
        let rb = json!({
            "children": [
                {"type": "TEXT", "characters": "🚀 Lorem ipsum"},
                {"type": "TEXT", "characters": "10x more"}
            ]
        });
        let a = lint(&rb, &rules());
        let b = lint(&rb, &rules());
        assert_eq!(a, b);
        assert!(a.len() >= 3);
    }
}
