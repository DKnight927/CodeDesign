//! The `DesignSystem` aggregate — loads all seven pieces from a directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::components::Components;
use crate::constraints::Constraints;
use crate::error::{Error, Result};
use crate::tokens::Tokens;

/// Metadata derived from the DS root (name, version, path).
#[derive(Debug, Clone)]
pub struct DsMeta {
    pub name: String,
    pub version: String,
    pub root: PathBuf,
}

/// One entry in `cases/index.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CaseEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub note: String,
}

/// Parsed cases/index.json.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CaseIndex {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub cases: Vec<CaseEntry>,
}

/// A fully loaded, validated DesignSystem.
#[derive(Debug, Clone)]
pub struct DesignSystem {
    pub meta: DsMeta,
    pub tokens: Tokens,
    pub components: Components,
    pub patterns: Value,
    pub constraints: Constraints,
    pub voice: String,
    pub design_md: String,
    pub cases: CaseIndex,
}

impl DesignSystem {
    /// Load a DS from its root directory. All seven pieces must be
    /// present and parse cleanly except `cases/index.json`, which is
    /// tolerated as missing (treated as an empty library).
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();

        let tokens_v = read_json(&root, "tokens.json")?;
        let tokens = Tokens::parse(&tokens_v)?;

        let components_v = read_json(&root, "components.json")?;
        let components = Components::parse(&components_v)?;

        let patterns = read_json(&root, "patterns.json")?;

        let constraints_v = read_json(&root, "constraints.json")?;
        let constraints = Constraints::parse(&constraints_v)?;

        let voice = read_text(&root, "voice.md")?;
        let design_md = read_text(&root, "DESIGN.md")?;

        let cases_path = root.join("cases").join("index.json");
        let cases = if cases_path.exists() {
            let raw = fs::read_to_string(&cases_path)?;
            let v: Value = serde_json::from_str(&raw)?;
            serde_json::from_value(v).map_err(|e| Error::Malformed {
                piece: "cases/index.json",
                detail: e.to_string(),
            })?
        } else {
            CaseIndex::default()
        };

        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_owned();

        let meta = DsMeta {
            name,
            version: tokens.version.clone(),
            root,
        };

        Ok(Self {
            meta,
            tokens,
            components,
            patterns,
            constraints,
            voice,
            design_md,
            cases,
        })
    }

    /// Load the default DesignSystem embedded in the binary at compile
    /// time. No filesystem dependency — this is what the CLI uses when
    /// the user hasn't configured a DS override.
    pub fn bundled_default() -> Result<Self> {
        const TOKENS: &str      = include_str!("../../../assets/design-systems/default/tokens.json");
        const COMPONENTS: &str  = include_str!("../../../assets/design-systems/default/components.json");
        const PATTERNS: &str    = include_str!("../../../assets/design-systems/default/patterns.json");
        const CONSTRAINTS: &str = include_str!("../../../assets/design-systems/default/constraints.json");
        const VOICE: &str       = include_str!("../../../assets/design-systems/default/voice.md");
        const DESIGN_MD: &str   = include_str!("../../../assets/design-systems/default/DESIGN.md");
        const CASES: &str       = include_str!("../../../assets/design-systems/default/cases/index.json");

        let tokens = Tokens::parse(&serde_json::from_str(TOKENS)?)?;
        let components = Components::parse(&serde_json::from_str(COMPONENTS)?)?;
        let patterns: Value = serde_json::from_str(PATTERNS)?;
        let constraints = Constraints::parse(&serde_json::from_str(CONSTRAINTS)?)?;
        let cases: CaseIndex = serde_json::from_str(CASES).map_err(|e| Error::Malformed {
            piece: "cases/index.json",
            detail: e.to_string(),
        })?;

        let meta = DsMeta {
            name: "default".into(),
            version: tokens.version.clone(),
            root: std::path::PathBuf::from("<bundled>"),
        };

        Ok(Self {
            meta,
            tokens,
            components,
            patterns,
            constraints,
            voice: VOICE.to_owned(),
            design_md: DESIGN_MD.to_owned(),
            cases,
        })
    }

    /// Resolve a token ref (e.g. `"color.bg.base"`) to its human-readable
    /// value string. Returns an error when the ref is unknown.
    pub fn value_for_ref(&self, token_ref: &str) -> Result<String> {
        self.tokens
            .get(token_ref)
            .map(super::tokens::TokenValue::display)
            .ok_or_else(|| Error::UnknownRef(token_ref.to_owned()))
    }

    /// True iff the ref resolves in any section of the tokens file.
    #[must_use]
    pub fn knows_ref(&self, token_ref: &str) -> bool {
        self.tokens.get(token_ref).is_some()
    }

    /// Component names defined in this DS, for Plan role enumeration.
    #[must_use]
    pub fn component_names(&self) -> Vec<String> {
        self.components.names()
    }

    /// Resolve case image entries to absolute paths on disk. Filters
    /// out entries whose `image` is empty or whose file does not
    /// resolve (bundled DS has no on-disk images; pass through).
    #[must_use]
    pub fn resolved_case_images(&self) -> Vec<(CaseEntry, PathBuf)> {
        if self.meta.root.as_os_str() == "<bundled>" {
            return Vec::new();
        }
        let mut out = Vec::new();
        for c in &self.cases.cases {
            if c.image.is_empty() {
                continue;
            }
            let path = self.meta.root.join("cases").join(&c.image);
            if path.exists() {
                out.push((c.clone(), path));
            }
        }
        out
    }
}

fn read_json(root: &Path, name: &'static str) -> Result<Value> {
    let path = root.join(name);
    if !path.exists() {
        return Err(Error::MissingPiece(name));
    }
    let raw = fs::read_to_string(&path)?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| Error::Malformed {
        piece: name,
        detail: e.to_string(),
    })?;
    Ok(v)
}

fn read_text(root: &Path, name: &'static str) -> Result<String> {
    let path = root.join(name);
    if !path.exists() {
        return Err(Error::MissingPiece(name));
    }
    Ok(fs::read_to_string(&path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_ds_path() -> PathBuf {
        // tests run from crate root
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("assets")
            .join("design-systems")
            .join("default")
    }

    #[test]
    fn loads_default_ds() {
        let ds = DesignSystem::load(default_ds_path()).unwrap();
        assert_eq!(ds.meta.name, "default");
        assert!(ds.knows_ref("color.brand.primary"));
        assert!(ds.component_names().iter().any(|n| n == "button.primary"));
        assert!(ds.voice.contains("中性"));
        assert!(ds.design_md.contains("Philosophy"));
        assert_eq!(ds.cases.cases.len(), 0);
    }

    #[test]
    fn value_for_ref_errors_on_unknown() {
        let ds = DesignSystem::load(default_ds_path()).unwrap();
        assert!(ds.value_for_ref("color.nonexistent").is_err());
        let v = ds.value_for_ref("space.md").unwrap();
        assert_eq!(v, "12");
    }

    #[test]
    fn missing_piece_errors() {
        let tmp = tempdir_empty();
        let err = DesignSystem::load(&tmp).unwrap_err();
        assert!(matches!(err, Error::MissingPiece(_)));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn bundled_default_loads() {
        let ds = DesignSystem::bundled_default().unwrap();
        assert_eq!(ds.meta.name, "default");
        assert!(ds.knows_ref("color.brand.primary"));
        assert!(ds.component_names().iter().any(|n| n == "button.primary"));
    }

    #[test]
    fn resolved_case_images_filters_missing_and_bundled() {
        let bundled = DesignSystem::bundled_default().unwrap();
        assert!(bundled.resolved_case_images().is_empty(), "bundled has no on-disk cases");

        // Synthesize a DS directory with one valid + one missing case image.
        let tmp = std::env::temp_dir().join(format!("cd-ds-cases-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("cases")).unwrap();
        let default_root = default_ds_path();
        for f in ["tokens.json","components.json","patterns.json","constraints.json","voice.md","DESIGN.md"] {
            fs::copy(default_root.join(f), tmp.join(f)).unwrap();
        }
        fs::write(tmp.join("cases").join("ok.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        fs::write(
            tmp.join("cases").join("index.json"),
            r#"{"version":"1","cases":[
                {"id":"a","title":"A","image":"ok.png","note":"","tags":[]},
                {"id":"b","title":"B","image":"missing.png","note":"","tags":[]},
                {"id":"c","title":"C","image":"","note":"","tags":[]}
            ]}"#,
        ).unwrap();

        let ds = DesignSystem::load(&tmp).unwrap();
        let resolved = ds.resolved_case_images();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].0.id, "a");

        let _ = fs::remove_dir_all(&tmp);
    }

    fn tempdir_empty() -> PathBuf {
        let p = std::env::temp_dir().join(format!("cd-ds-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
}
