//! cd-ds — DesignSystem loader (7-piece canonical schema).
//!
//! A CodeDesign DesignSystem lives on disk as a directory with seven
//! pieces. This crate reads that directory and produces a single
//! [`DesignSystem`] struct that the Plan / Critique / Fixer roles can
//! bind against. Everything else (hex resolution, component lookup,
//! prompt formatting) is a method on this struct.
//!
//! Layout (rooted at a DS dir):
//!
//!   tokens.json       — colour / space / radius / shadow / typo tokens
//!   components.json   — component contracts with variants + token refs
//!   patterns.json     — compound UX patterns (onboarding, empty state…)
//!   constraints.json  — enforcement mode + craft requires + limits
//!   voice.md          — tone-of-voice guide
//!   DESIGN.md         — philosophy + IA + Do/Don't (version in frontmatter)
//!   cases/index.json  — case library metadata (images + labels)
//!
//! A DS is "valid" iff all seven pieces parse. Missing `cases/` is
//! tolerated: an empty case library is a valid state.

pub mod components;
pub mod constraints;
pub mod design_system;
pub mod error;
pub mod prompt;
pub mod tokens;

pub use components::{ComponentSpec, Components};
pub use constraints::Constraints;
pub use design_system::{CaseEntry, CaseIndex, DesignSystem, DsMeta};
pub use error::{Error, Result};
pub use tokens::{TokenValue, Tokens};

/// Crate name constant; used in smoke tests to prove wiring across the workspace.
pub const CRATE_NAME: &str = "cd-ds";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-ds");
    }
}
