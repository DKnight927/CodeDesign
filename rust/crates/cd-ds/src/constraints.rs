//! constraints.json parser.
//!
//! Constraints express the DS's enforcement posture: what the Critic
//! should treat as a hard fail vs. a soft warning. The schema is narrow
//! on purpose — anything ad-hoc belongs in craft/ markdown files.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::{Error, Result};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Enforcement {
    Strict,
    #[default]
    Balanced,
    Lenient,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AllowExtension {
    #[serde(default)]
    pub tokens: bool,
    #[serde(default)]
    pub components: bool,
    #[serde(default)]
    pub patterns: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Craft {
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub overrides: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct A11y {
    #[serde(rename = "contrastMin", default = "default_contrast_min")]
    pub contrast_min: f64,
    #[serde(rename = "largeContrastMin", default = "default_large_contrast_min")]
    pub large_contrast_min: f64,
    #[serde(rename = "requireFocusRing", default = "default_true")]
    pub require_focus_ring: bool,
}

fn default_contrast_min() -> f64 { 4.5 }
fn default_large_contrast_min() -> f64 { 3.0 }
fn default_true() -> bool { true }

impl Default for A11y {
    fn default() -> Self {
        Self { contrast_min: 4.5, large_contrast_min: 3.0, require_focus_ring: true }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Typography {
    #[serde(rename = "maxSizesPerFrame", default)]
    pub max_sizes_per_frame: Option<u32>,
    #[serde(rename = "maxWeightsPerFrame", default)]
    pub max_weights_per_frame: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Spacing {
    #[serde(default = "default_space_base")]
    pub base: u32,
    #[serde(rename = "disallowOddPx", default)]
    pub disallow_odd_px: bool,
}

fn default_space_base() -> u32 { 4 }

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Color {
    #[serde(rename = "maxHuesPerFrame", default)]
    pub max_hues_per_frame: Option<u32>,
    #[serde(rename = "brandColorForInteractiveOnly", default)]
    pub brand_color_for_interactive_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Constraints {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub enforcement: Enforcement,
    #[serde(rename = "allowExtension", default)]
    pub allow_extension: AllowExtension,
    #[serde(default)]
    pub craft: Craft,
    #[serde(default)]
    pub a11y: A11y,
    #[serde(default)]
    pub typography: Typography,
    #[serde(default)]
    pub spacing: Spacing,
    #[serde(default)]
    pub color: Color,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            version: "0.0.0".into(),
            enforcement: Enforcement::Balanced,
            allow_extension: AllowExtension::default(),
            craft: Craft::default(),
            a11y: A11y::default(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            color: Color::default(),
        }
    }
}

impl Constraints {
    pub fn parse(json: &Value) -> Result<Self> {
        serde_json::from_value(json.clone()).map_err(|e| Error::Malformed {
            piece: "constraints.json",
            detail: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!(
        "../../../assets/design-systems/default/constraints.json"
    );

    #[test]
    fn parses_default_constraints() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        let c = Constraints::parse(&v).unwrap();
        assert_eq!(c.enforcement, Enforcement::Balanced);
        assert_eq!(c.a11y.contrast_min, 4.5);
        assert!(c.a11y.require_focus_ring);
        assert!(c.spacing.disallow_odd_px);
        assert!(c.craft.requires.iter().any(|r| r == "anti-ai-slop"));
        assert!(!c.allow_extension.tokens);
        assert!(c.allow_extension.patterns);
    }
}
