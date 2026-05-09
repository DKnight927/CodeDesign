//! The Brief — Interpreter output. Feeds the Planner.
//!
//! A Brief is what we extract from the user's PRD + Turn-1 answers.
//! It is deliberately shallow: the Planner lives downstream and needs
//! only the structure, not prose. Anything fuzzy goes in `notes`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Mobile,
    Desktop,
    Responsive,
}

/// A single question the Interpreter needs answered in Turn-1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClarifyAsk {
    pub id: String,
    pub prompt: String,
    pub options: Vec<String>,
    /// True iff this ask may be multi-select.
    #[serde(default)]
    pub allow_multiple: bool,
}

/// The user's answer to a [`ClarifyAsk`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClarifyAnswer {
    pub id: String,
    pub values: Vec<String>,
}

/// Interpreter output — the structured brief the Planner binds against.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Brief {
    /// One-line restatement of what the user wants.
    pub intent: String,

    /// Target platform (narrows Planner frame sizes).
    pub platform: Option<Platform>,

    /// Primary users / audience, in short phrases.
    #[serde(default)]
    pub users: Vec<String>,

    /// Jobs-to-be-done / user goals.
    #[serde(default)]
    pub jobs: Vec<String>,

    /// Screens the Planner should produce (short labels, not IDs).
    #[serde(default)]
    pub screens: Vec<String>,

    /// Hard constraints (e.g. "must fit on 375 wide", "offline-first").
    #[serde(default)]
    pub constraints: Vec<String>,

    /// Success signals — what 'good' looks like from the user's side.
    #[serde(default)]
    pub success_signals: Vec<String>,

    /// Chosen visual direction id from the VDP (if any).
    pub direction: Option<String>,

    /// Free-form notes captured by the Interpreter.
    pub notes: Option<String>,
}

impl Brief {
    /// True iff the Brief has enough signal for the Planner to proceed.
    #[must_use]
    pub fn is_plan_ready(&self) -> bool {
        !self.intent.trim().is_empty()
            && self.platform.is_some()
            && !self.screens.is_empty()
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_ready_requires_intent_platform_screens() {
        let mut b = Brief::default();
        assert!(!b.is_plan_ready());
        b.intent = "signup flow".into();
        assert!(!b.is_plan_ready());
        b.platform = Some(Platform::Mobile);
        assert!(!b.is_plan_ready());
        b.screens.push("welcome".into());
        assert!(b.is_plan_ready());
    }

    #[test]
    fn roundtrip_serde() {
        let b = Brief {
            intent: "onboarding".into(),
            platform: Some(Platform::Mobile),
            screens: vec!["welcome".into(), "signup".into()],
            direction: Some("neutral-modern".into()),
            ..Default::default()
        };
        let s = b.to_json_pretty().unwrap();
        let back: Brief = serde_json::from_str(&s).unwrap();
        assert_eq!(b, back);
    }
}
