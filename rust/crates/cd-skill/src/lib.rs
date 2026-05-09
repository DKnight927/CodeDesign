//! cd-skill — SKILL.md frontmatter loader (P3.3) + per-tool
//! augmentation prompts (P3.4).
//!
//! A skill is a directory containing `SKILL.md`:
//!
//! ```text
//! ---
//! cd:
//!   name: onboarding
//!   version: 1.0.0
//!   product_kind: screens
//!   platform: mobile
//!   requires:
//!     ds_patterns: ["onboarding.*"]
//!     craft: ["state-coverage"]
//!     tokens: ["color.brand.primary"]
//!   tool_augmentation:
//!     plan_emit: |
//!       For onboarding flows, each step frame must carry a "skip"
//!       affordance and a progress indicator.
//! ---
//!
//! # Body — any free-form guidance injected verbatim into the role
//! # system prompt when the skill is active.
//! ```
//!
//! Everything before the second `---` is parsed as TOML (not YAML —
//! we already depend on `toml` workspace crate, and the shape is
//! shallow). Everything after is the body.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use cd_ds::DesignSystem;

pub const CRATE_NAME: &str = "cd-skill";

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("frontmatter: missing opening `---` fence")]
    MissingOpenFence,
    #[error("frontmatter: missing closing `---` fence")]
    MissingCloseFence,
    #[error("frontmatter toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unmet requirement: {0}")]
    Unmet(String),
}

/// A loaded skill, ready to contribute to prompts.
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub front: SkillFront,
    pub body: String,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SkillFrontCd {
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub product_kind: String, // screens|component|ds-extract|audit|refine
    #[serde(default)]
    pub platform: String, // mobile|desktop|responsive
    #[serde(default)]
    pub requires: SkillRequires,
    /// Per-tool prompt augmentations. Keyed by tool name
    /// (e.g. "plan_emit"). Prepended to the role system prompt
    /// when that tool is in the active role's tool_surface.
    #[serde(default)]
    pub tool_augmentation: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillRequires {
    #[serde(default)]
    pub ds_patterns: Vec<String>,
    #[serde(default)]
    pub craft: Vec<String>,
    #[serde(default)]
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillFront {
    pub cd: SkillFrontCd,
}

impl Skill {
    /// Load a skill from its directory (expects `<dir>/SKILL.md`).
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, SkillError> {
        let dir = dir.as_ref().to_path_buf();
        let md = fs::read_to_string(dir.join("SKILL.md"))?;
        let (front, body) = split_frontmatter(&md)?;
        let parsed: SkillFront = toml::from_str(front)?;
        Ok(Self {
            front: parsed,
            body: body.trim().to_owned(),
            root: dir,
        })
    }

    /// Check every `requires.*` against a loaded DS. Returns `Ok(())`
    /// when satisfied, otherwise `Err(SkillError::Unmet(...))`.
    pub fn check_requirements(&self, ds: &DesignSystem) -> Result<(), SkillError> {
        for tok in &self.front.cd.requires.tokens {
            if !ds.knows_ref(tok) {
                return Err(SkillError::Unmet(format!("token `{tok}` not in DS")));
            }
        }
        for rule in &self.front.cd.requires.craft {
            if !ds.constraints.craft.requires.iter().any(|r| r == rule) {
                return Err(SkillError::Unmet(format!(
                    "craft rule `{rule}` not required by DS constraints"
                )));
            }
        }
        for pat in &self.front.cd.requires.ds_patterns {
            if !pattern_present(&ds.patterns, pat) {
                return Err(SkillError::Unmet(format!(
                    "DS patterns missing match for `{pat}`"
                )));
            }
        }
        Ok(())
    }

    /// Compose a prompt fragment for a role whose `tool_surface`
    /// contains any tool the skill augments, plus the body.
    #[must_use]
    pub fn prompt_fragment(&self, active_tools: &[String]) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "\n# Active skill: {} ({})\n",
            self.front.cd.name, self.front.cd.product_kind
        ));
        if !self.body.is_empty() {
            out.push_str(&self.body);
            out.push('\n');
        }
        for (tool, aug) in &self.front.cd.tool_augmentation {
            if active_tools.iter().any(|t| t == tool) {
                out.push_str(&format!("\n## tool augmentation: {tool}\n{aug}\n"));
            }
        }
        out
    }
}

fn pattern_present(patterns: &serde_json::Value, needle: &str) -> bool {
    let prefix = needle.trim_end_matches('*');
    match patterns {
        serde_json::Value::Object(map) => map.keys().any(|k| k.starts_with(prefix)),
        _ => false,
    }
}

fn split_frontmatter(md: &str) -> Result<(&str, &str), SkillError> {
    let rest = md.strip_prefix("---\n").ok_or(SkillError::MissingOpenFence)?;
    let end = rest.find("\n---").ok_or(SkillError::MissingCloseFence)?;
    let front = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n');
    Ok((front, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_skill(front: &str, body: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cd-skill-test-{}-{:?}-{unique}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let md = format!("---\n{front}\n---\n\n{body}\n");
        std::fs::write(dir.join("SKILL.md"), md).unwrap();
        dir
    }

    fn rand_suffix() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
    }

    #[test]
    fn load_minimal_skill() {
        let dir = tmp_skill(
            r#"[cd]
name = "screens"
version = "1.0.0"
product_kind = "screens"
platform = "mobile"
"#,
            "Some body text.",
        );
        let s = Skill::load(&dir).unwrap();
        assert_eq!(s.front.cd.name, "screens");
        assert_eq!(s.body, "Some body text.");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_fence_errors() {
        let dir = std::env::temp_dir().join(format!("cd-skill-bad-{}-{}", std::process::id(), rand_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "no frontmatter here").unwrap();
        assert!(matches!(Skill::load(&dir), Err(SkillError::MissingOpenFence)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requirements_satisfied_against_bundled_ds() {
        let ds = DesignSystem::bundled_default().unwrap();
        let dir = tmp_skill(
            r#"[cd]
name = "ok"
product_kind = "audit"
[cd.requires]
tokens = ["color.brand.primary"]
craft = ["anti-ai-slop"]
"#,
            "",
        );
        let s = Skill::load(&dir).unwrap();
        s.check_requirements(&ds).expect("bundled DS satisfies");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requirement_unmet_surfaces_error() {
        let ds = DesignSystem::bundled_default().unwrap();
        let dir = tmp_skill(
            r#"[cd]
name = "bad"
product_kind = "audit"
[cd.requires]
tokens = ["color.nonexistent"]
"#,
            "",
        );
        let s = Skill::load(&dir).unwrap();
        assert!(matches!(s.check_requirements(&ds), Err(SkillError::Unmet(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prompt_fragment_includes_tool_augmentation() {
        let dir = tmp_skill(
            r#"[cd]
name = "onb"
product_kind = "screens"
[cd.tool_augmentation]
plan_emit = "Onboarding rule: always include skip."
"#,
            "body",
        );
        let s = Skill::load(&dir).unwrap();
        let f = s.prompt_fragment(&["plan_emit".to_string()]);
        assert!(f.contains("Active skill: onb"));
        assert!(f.contains("Onboarding rule"));
        let f2 = s.prompt_fragment(&["critic_emit".to_string()]);
        assert!(!f2.contains("Onboarding rule"), "augmentation hides when tool inactive");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bundled_sample_skills_parse() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .join("assets/skills");
        for name in ["screens", "audit"] {
            let s = Skill::load(root.join(name)).unwrap_or_else(|e| panic!("skill {name}: {e}"));
            assert_eq!(s.front.cd.name, name);
        }
    }
}
