//! tokens.json parser and lookup helpers.
//!
//! The file schema (v1) groups tokens under five sections:
//!   color   — OKLCH strings (e.g. `"oklch(0.99 0 0)"`)
//!   space   — integer px values
//!   radius  — integer px values
//!   shadow  — CSS shadow strings (e.g. `"0 1px 2px oklch(0 0 0 / 0.06)"`)
//!   typo    — typography objects (family/weight/size/lineHeight)
//!
//! The `space` block additionally carries a top-level `base` key holding
//! the base grid in px — that is kept on [`Tokens::space_base`] and
//! stripped from the map.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

/// A single typography token.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TypoSpec {
    pub family: String,
    pub weight: u32,
    pub size: f64,
    #[serde(rename = "lineHeight")]
    pub line_height: f64,
}

/// Uniform value type — tokens.json mixes scalar / string / object shapes.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Color(String),
    Number(f64),
    Text(String),
    Typo(TypoSpec),
}

impl TokenValue {
    /// Return the raw string form usable in prompts / critique snippets.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            TokenValue::Color(s) | TokenValue::Text(s) => s.clone(),
            TokenValue::Number(n) => format_number(*n),
            TokenValue::Typo(t) => format!(
                "{}/{} {}px/{}",
                t.family, t.weight, t.size, t.line_height
            ),
        }
    }
}

fn format_number(n: f64) -> String {
    if (n.fract()).abs() < f64::EPSILON {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Parsed tokens.json.
#[derive(Debug, Clone, Default)]
pub struct Tokens {
    pub version: String,
    pub space_base: u32,
    pub color: BTreeMap<String, TokenValue>,
    pub space: BTreeMap<String, TokenValue>,
    pub radius: BTreeMap<String, TokenValue>,
    pub shadow: BTreeMap<String, TokenValue>,
    pub typo: BTreeMap<String, TokenValue>,
}

impl Tokens {
    /// Parse a raw tokens.json document.
    pub fn parse(json: &Value) -> Result<Self> {
        let obj = json
            .as_object()
            .ok_or(Error::Malformed { piece: "tokens.json", detail: "root must be an object".into() })?;

        let version = obj
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("0.0.0")
            .to_owned();

        let color = parse_scalar_section(obj.get("color"), "color", parse_color)?;
        let radius = parse_scalar_section(obj.get("radius"), "radius", parse_number)?;
        let shadow = parse_scalar_section(obj.get("shadow"), "shadow", parse_text)?;

        // space has a bare `base` int alongside the { value } entries.
        let (space_base, space) = parse_space_section(obj.get("space"))?;

        // typo entries are objects, not { value: … } wrappers.
        let typo = parse_typo_section(obj.get("typo"))?;

        Ok(Self { version, space_base, color, space, radius, shadow, typo })
    }

    /// Lookup a token by dotted ref (e.g. `"color.bg.base"` or `"typo.title.lg"`).
    ///
    /// The ref's first segment selects the section; the remainder is the
    /// token key within that section (itself often a dotted leaf name).
    #[must_use]
    pub fn get(&self, token_ref: &str) -> Option<&TokenValue> {
        let (section, rest) = token_ref.split_once('.')?;
        match section {
            "color" => self.color.get(rest),
            "space" => self.space.get(rest),
            "radius" => self.radius.get(rest),
            "shadow" => self.shadow.get(rest),
            "typo" => self.typo.get(rest),
            _ => None,
        }
    }

    /// All valid ref strings, grouped by section, in stable order.
    #[must_use]
    pub fn all_refs(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (sec, map) in [
            ("color", &self.color),
            ("space", &self.space),
            ("radius", &self.radius),
            ("shadow", &self.shadow),
            ("typo", &self.typo),
        ] {
            for k in map.keys() {
                out.push(format!("{sec}.{k}"));
            }
        }
        out
    }
}

// ── section parsers ─────────────────────────────────────────────────────────

fn parse_scalar_section(
    section: Option<&Value>,
    name: &'static str,
    leaf: fn(&Value) -> Result<TokenValue>,
) -> Result<BTreeMap<String, TokenValue>> {
    let Some(obj) = section.and_then(Value::as_object) else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        let val = v.get("value").ok_or(Error::Malformed {
            piece: "tokens.json",
            detail: format!("{name}.{k} is missing `value`"),
        })?;
        out.insert(k.clone(), leaf(val)?);
    }
    Ok(out)
}

fn parse_color(v: &Value) -> Result<TokenValue> {
    v.as_str()
        .map(|s| TokenValue::Color(s.to_owned()))
        .ok_or(Error::Malformed { piece: "tokens.json", detail: "color value must be a string".into() })
}

fn parse_text(v: &Value) -> Result<TokenValue> {
    v.as_str()
        .map(|s| TokenValue::Text(s.to_owned()))
        .ok_or(Error::Malformed { piece: "tokens.json", detail: "shadow value must be a string".into() })
}

fn parse_number(v: &Value) -> Result<TokenValue> {
    v.as_f64()
        .map(TokenValue::Number)
        .ok_or(Error::Malformed { piece: "tokens.json", detail: "numeric value expected".into() })
}

fn parse_space_section(section: Option<&Value>) -> Result<(u32, BTreeMap<String, TokenValue>)> {
    let Some(obj) = section.and_then(Value::as_object) else {
        return Ok((4, BTreeMap::new()));
    };
    let mut base: u32 = 4;
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        if k == "base" {
            if let Some(n) = v.as_u64() {
                base = u32::try_from(n).unwrap_or(4);
            }
            continue;
        }
        let val = v.get("value").ok_or(Error::Malformed {
            piece: "tokens.json",
            detail: format!("space.{k} is missing `value`"),
        })?;
        out.insert(k.clone(), parse_number(val)?);
    }
    Ok((base, out))
}

fn parse_typo_section(section: Option<&Value>) -> Result<BTreeMap<String, TokenValue>> {
    let Some(obj) = section.and_then(Value::as_object) else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        let spec: TypoSpec = serde_json::from_value(v.clone()).map_err(|e| Error::Malformed {
            piece: "tokens.json",
            detail: format!("typo.{k}: {e}"),
        })?;
        out.insert(k.clone(), TokenValue::Typo(spec));
    }
    Ok(out)
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!(
        "../../../assets/design-systems/default/tokens.json"
    );

    #[test]
    fn parses_default_tokens() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        let t = Tokens::parse(&v).unwrap();
        assert_eq!(t.space_base, 4);
        assert!(t.color.contains_key("bg.base"));
        assert!(t.space.contains_key("md"));
        assert!(t.typo.contains_key("title.lg"));
    }

    #[test]
    fn get_by_ref() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        let t = Tokens::parse(&v).unwrap();
        let c = t.get("color.brand.primary").unwrap();
        match c {
            TokenValue::Color(s) => assert!(s.starts_with("oklch(")),
            _ => panic!("wrong variant"),
        }
        assert!(t.get("color.does.not.exist").is_none());
        assert!(t.get("bogus.section").is_none());
    }

    #[test]
    fn all_refs_contains_expected() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        let t = Tokens::parse(&v).unwrap();
        let refs = t.all_refs();
        assert!(refs.iter().any(|r| r == "color.bg.base"));
        assert!(refs.iter().any(|r| r == "typo.title.lg"));
    }
}
