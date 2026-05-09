//! Session state machine.
//!
//! Phases (strict order):
//!   Intake     — raw PRD received
//!   Interpret  — Interpreter produced (or restored) a Brief
//!   Clarify    — Turn-1 form asked; awaiting answers
//!   DsBound    — DS loaded, direction pinned
//!   Planned    — DesignPlan emitted
//!   Executed   — MCP figma_execute returned a readback
//!   Critiqued  — Critic findings attached; terminal
//!
//! Each transition is fallible; invalid jumps return an error. Sessions
//! are serialisable so the CLI can resume from disk on `--resume`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::brief::{Brief, ClarifyAsk, ClarifyAnswer};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Intake,
    Interpret,
    Clarify,
    DsBound,
    Planned,
    Executed,
    Critiqued,
}

#[derive(Debug, Error)]
pub enum Transition {
    #[error("invalid transition from {from:?} to {to:?}")]
    Invalid { from: Phase, to: Phase },
    #[error("phase {0:?} requires {1} to be set")]
    Missing(Phase, &'static str),
    #[error("serde: {0}")]
    Json(#[from] serde_json::Error),
}

/// A full session. Fields accumulate as phases advance; earlier phases
/// never clear later-phase data (allows resume + replay).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub phase: Phase,
    pub prd: String,

    #[serde(default)]
    pub brief: Option<Brief>,
    #[serde(default)]
    pub asks: Vec<ClarifyAsk>,
    #[serde(default)]
    pub answers: Vec<ClarifyAnswer>,

    #[serde(default)]
    pub ds_ref: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,

    #[serde(default)]
    pub plan: Option<Value>,
    #[serde(default)]
    pub readback: Option<Value>,
    #[serde(default)]
    pub findings: Option<Value>,
}

impl Session {
    #[must_use]
    pub fn new(id: impl Into<String>, prd: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            phase: Phase::Intake,
            prd: prd.into(),
            brief: None,
            asks: Vec::new(),
            answers: Vec::new(),
            ds_ref: None,
            direction: None,
            plan: None,
            readback: None,
            findings: None,
        }
    }

    /// Advance to Interpret — attaches the Brief.
    pub fn interpret(&mut self, brief: Brief) -> Result<(), Transition> {
        self.check_from(&[Phase::Intake])?;
        self.brief = Some(brief);
        self.phase = Phase::Interpret;
        Ok(())
    }

    /// Queue Turn-1 questions.
    pub fn clarify_ask(&mut self, asks: Vec<ClarifyAsk>) -> Result<(), Transition> {
        self.check_from(&[Phase::Interpret])?;
        self.asks = asks;
        self.phase = Phase::Clarify;
        Ok(())
    }

    /// Record Turn-1 answers and merge them into the Brief's
    /// `direction` / `platform` / `screens` fields where applicable.
    pub fn clarify_answer(&mut self, answers: Vec<ClarifyAnswer>) -> Result<(), Transition> {
        self.check_from(&[Phase::Clarify])?;
        self.answers = answers;
        // promote direction if the Interpreter asked about it
        if let Some(ans) = self
            .answers
            .iter()
            .find(|a| a.id == "direction" && !a.values.is_empty())
        {
            self.direction = Some(ans.values[0].clone());
            if let Some(b) = self.brief.as_mut() {
                b.direction = Some(ans.values[0].clone());
            }
        }
        // stays in Clarify until DS bind promotes it
        Ok(())
    }

    /// Bind a DS and lock the visual direction.
    pub fn ds_bind(
        &mut self,
        ds_ref: impl Into<String>,
        direction: Option<String>,
    ) -> Result<(), Transition> {
        self.check_from(&[Phase::Interpret, Phase::Clarify])?;
        self.ds_ref = Some(ds_ref.into());
        if direction.is_some() {
            self.direction = direction.clone();
            if let Some(b) = self.brief.as_mut() {
                b.direction = direction;
            }
        }
        self.phase = Phase::DsBound;
        Ok(())
    }

    /// Attach a DesignPlan.
    pub fn plan(&mut self, plan: Value) -> Result<(), Transition> {
        self.check_from(&[Phase::DsBound])?;
        self.plan = Some(plan);
        self.phase = Phase::Planned;
        Ok(())
    }

    /// Attach an MCP readback.
    pub fn execute(&mut self, readback: Value) -> Result<(), Transition> {
        self.check_from(&[Phase::Planned])?;
        self.readback = Some(readback);
        self.phase = Phase::Executed;
        Ok(())
    }

    /// Attach Critic findings (terminal).
    pub fn critique(&mut self, findings: Value) -> Result<(), Transition> {
        self.check_from(&[Phase::Executed])?;
        self.findings = Some(findings);
        self.phase = Phase::Critiqued;
        Ok(())
    }

    /// Serialise for resume.
    pub fn save_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Restore from serialised form.
    pub fn load_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    fn check_from(&self, allowed: &[Phase]) -> Result<(), Transition> {
        if !allowed.contains(&self.phase) {
            return Err(Transition::Invalid { from: self.phase, to: next_phase(self.phase) });
        }
        Ok(())
    }
}

fn next_phase(p: Phase) -> Phase {
    match p {
        Phase::Intake => Phase::Interpret,
        Phase::Interpret => Phase::Clarify,
        Phase::Clarify => Phase::DsBound,
        Phase::DsBound => Phase::Planned,
        Phase::Planned => Phase::Executed,
        Phase::Executed | Phase::Critiqued => Phase::Critiqued,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brief::{Brief, Platform};
    use serde_json::json;

    fn sample_brief() -> Brief {
        Brief {
            intent: "signup".into(),
            platform: Some(Platform::Mobile),
            screens: vec!["welcome".into()],
            ..Default::default()
        }
    }

    #[test]
    fn full_happy_path() {
        let mut s = Session::new("sess-1", "design a signup");
        assert_eq!(s.phase, Phase::Intake);

        s.interpret(sample_brief()).unwrap();
        assert_eq!(s.phase, Phase::Interpret);

        s.clarify_ask(vec![ClarifyAsk {
            id: "direction".into(),
            prompt: "pick a direction".into(),
            options: vec!["neutral-modern".into(), "tech-utility".into()],
            allow_multiple: false,
        }])
        .unwrap();
        assert_eq!(s.phase, Phase::Clarify);

        s.clarify_answer(vec![ClarifyAnswer {
            id: "direction".into(),
            values: vec!["tech-utility".into()],
        }])
        .unwrap();
        assert_eq!(s.direction.as_deref(), Some("tech-utility"));

        s.ds_bind("default@1.0.0", s.direction.clone()).unwrap();
        assert_eq!(s.phase, Phase::DsBound);

        s.plan(json!({"frames": []})).unwrap();
        assert_eq!(s.phase, Phase::Planned);

        s.execute(json!({"nodes": []})).unwrap();
        assert_eq!(s.phase, Phase::Executed);

        s.critique(json!([])).unwrap();
        assert_eq!(s.phase, Phase::Critiqued);
    }

    #[test]
    fn rejects_skipped_transitions() {
        let mut s = Session::new("sess-2", "x");
        // can't plan before ds_bind
        assert!(s.plan(json!({})).is_err());
    }

    #[test]
    fn resume_roundtrip() {
        let mut s = Session::new("sess-3", "x");
        s.interpret(sample_brief()).unwrap();
        let raw = s.save_json().unwrap();
        let back = Session::load_json(&raw).unwrap();
        assert_eq!(back.phase, Phase::Interpret);
        assert_eq!(back.id, "sess-3");
    }

    #[test]
    fn ds_bind_from_interpret_without_clarify() {
        let mut s = Session::new("sess-4", "x");
        s.interpret(sample_brief()).unwrap();
        s.ds_bind("default@1.0.0", None).unwrap();
        assert_eq!(s.phase, Phase::DsBound);
    }
}
