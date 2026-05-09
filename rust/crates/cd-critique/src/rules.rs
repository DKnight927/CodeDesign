//! Anti-slop rule loading from `anti-ai-slop.json`.

use std::path::Path;

use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RulesError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid regex `{pattern}`: {source}")]
    BadRegex { pattern: String, source: regex::Error },
}

/// Compiled, ready-to-use rule set.
#[derive(Debug, Clone)]
pub struct AntiSlopRules {
    pub version: u32,
    pub banned_hex: Vec<String>,
    pub banned_emoji: Vec<String>,
    pub banned_display_fonts: Vec<String>,
    pub placeholder_regex: Vec<Regex>,
    pub fabricated_metric_regex: Vec<Regex>,
}

#[derive(Deserialize)]
struct RawRules {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(rename = "bannedHex", default)]
    banned_hex: Vec<String>,
    #[serde(rename = "bannedEmojiInFunctionalRole", default)]
    banned_emoji: Vec<String>,
    #[serde(rename = "bannedDisplayFonts", default)]
    banned_display_fonts: Vec<String>,
    #[serde(rename = "bannedPlaceholderRegex", default)]
    placeholder_regex: Vec<String>,
    #[serde(rename = "fabricatedMetricRegex", default)]
    fabricated_metric_regex: Vec<String>,
}

fn default_version() -> u32 { 1 }

impl AntiSlopRules {
    /// Compile rules from raw JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RulesError> {
        let raw: RawRules = serde_json::from_slice(bytes)?;
        let banned_hex = raw
            .banned_hex
            .into_iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        let placeholder_regex = compile(raw.placeholder_regex)?;
        let fabricated_metric_regex = compile(raw.fabricated_metric_regex)?;
        Ok(Self {
            version: raw.version,
            banned_hex,
            banned_emoji: raw.banned_emoji,
            banned_display_fonts: raw.banned_display_fonts,
            placeholder_regex,
            fabricated_metric_regex,
        })
    }

    /// Load from a filesystem path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RulesError> {
        let bytes = std::fs::read(path)?;
        Self::from_json(&bytes)
    }

    /// Load the rules embedded at compile time from the workspace asset.
    pub fn bundled() -> Result<Self, RulesError> {
        const BYTES: &[u8] = include_bytes!(
            "../../../assets/craft/anti-ai-slop.json"
        );
        Self::from_json(BYTES)
    }
}

fn compile(patterns: Vec<String>) -> Result<Vec<Regex>, RulesError> {
    patterns
        .into_iter()
        .map(|p| Regex::new(&p).map_err(|source| RulesError::BadRegex { pattern: p, source }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_loads() {
        let r = AntiSlopRules::bundled().unwrap();
        assert_eq!(r.banned_hex.len(), 7);
        assert_eq!(r.banned_emoji.len(), 17);
        assert!(!r.placeholder_regex.is_empty());
        // all hex lowercased
        assert!(r.banned_hex.iter().all(|h| h == &h.to_ascii_lowercase()));
    }
}
