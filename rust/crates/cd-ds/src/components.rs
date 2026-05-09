//! components.json parser.
//!
//! A component has three parts: `variants` (axes + enums), `slots` (named
//! content holes, optionally optional via a trailing `?` in the schema
//! literal), and `tokens` (named token refs resolved via [`crate::Tokens`]).

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

/// One component contract, e.g. `button.primary`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ComponentSpec {
    #[serde(default)]
    pub variants: BTreeMap<String, Vec<String>>,

    #[serde(default)]
    pub slots: BTreeMap<String, SlotKind>,

    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
}

/// A slot's accepted content kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotKind {
    pub kind: String,
    pub optional: bool,
}

impl<'de> Deserialize<'de> for SlotKind {
    fn deserialize<D>(d: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(d)?;
        let (kind, optional) = raw
            .strip_suffix('?')
            .map_or_else(|| (raw.as_str(), false), |k| (k, true));
        Ok(SlotKind { kind: kind.to_owned(), optional })
    }
}

/// Parsed components.json.
#[derive(Debug, Clone, Default)]
pub struct Components {
    pub version: String,
    pub items: BTreeMap<String, ComponentSpec>,
}

impl Components {
    pub fn parse(json: &Value) -> Result<Self> {
        let obj = json.as_object().ok_or(Error::Malformed {
            piece: "components.json",
            detail: "root must be an object".into(),
        })?;

        let version = obj
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("0.0.0")
            .to_owned();

        let items = match obj.get("components") {
            Some(v) => serde_json::from_value::<BTreeMap<String, ComponentSpec>>(v.clone())
                .map_err(|e| Error::Malformed {
                    piece: "components.json",
                    detail: e.to_string(),
                })?,
            None => BTreeMap::new(),
        };

        Ok(Self { version, items })
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ComponentSpec> {
        self.items.get(name)
    }

    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.items.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!(
        "../../../assets/design-systems/default/components.json"
    );

    #[test]
    fn parses_default_components() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        let c = Components::parse(&v).unwrap();
        assert!(c.get("button.primary").is_some());
        let btn = c.get("button.primary").unwrap();
        assert!(btn.variants.contains_key("size"));
        assert_eq!(btn.tokens.get("bg").unwrap(), "brand.primary");

        // optional slot parsing
        let input = c.get("input.text").unwrap();
        assert!(input.slots.get("helper").unwrap().optional);
        assert!(!input.slots.get("label").unwrap().optional);
    }
}
