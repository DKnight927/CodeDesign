//! cd-summarizer — isolated `ConversationRuntime` that compresses a long main
//! session into a compact `SessionSummary` without polluting the main budget.
//!
//! See `/DESIGN.md` §11 (compaction) and `/INHERITANCE.md` (why summarizer
//! runs as a second ConversationRuntime instance rather than a helper fn).
//!
//! # Two layers
//!   1. **Data contract** — `SessionSummary` + `to_injection_text()`. Inputs
//!      come from the main session; outputs are spliced back as
//!      `role=system, name=summary` per §11.2.
//!   2. **Isolated runtime** — `SummarizerClient` holds provider config and
//!      runs a *fresh* chat-completion request (summarizer system prompt +
//!      flattened transcript). The main session's token budget never sees
//!      this traffic.
//!
//! The runtime crate (or `cd-cli`) owns *when* to summarize (token threshold,
//! explicit command). This crate owns *how* — prompt, schema validation,
//! injection formatting.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const CRATE_NAME: &str = "cd-summarizer";

/// Compact record of a finished main session. Injected into the resumed
/// session as `role=system, name=summary` per §11.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSummary {
    /// Stable facts that must survive compression: platform, tone, brand refs,
    /// user constraints, DS identity.
    #[serde(default)]
    pub facts: Vec<String>,
    /// Decisions made during the session (direction picked, DS adopted,
    /// skills invoked, gates overridden).
    #[serde(default)]
    pub decisions: Vec<String>,
    /// Unresolved user asks or explicit deferrals.
    #[serde(default, rename = "openQuestions")]
    pub open_questions: Vec<String>,
    /// One-line digests of noteworthy tool results (so later turns don't
    /// re-issue the same query).
    #[serde(default, rename = "toolResultsDigest")]
    pub tool_results_digest: Vec<String>,
    /// Last known Plan id, if any — lets the resumed session reference it.
    #[serde(default, rename = "lastPlanId")]
    pub last_plan_id: Option<String>,
}

impl SessionSummary {
    pub fn from_json(s: &str) -> Result<Self, SummaryError> {
        serde_json::from_str(s).map_err(SummaryError::InvalidJson)
    }

    /// Render as the `content` payload we splice into the main session.
    /// See §11.2 for the exact injection shape.
    pub fn to_injection_text(&self) -> String {
        let mut out = String::new();
        out.push_str("=== PRIOR SESSION SUMMARY ===\n");
        if !self.facts.is_empty() {
            out.push_str("\nFacts:\n");
            for f in &self.facts {
                out.push_str("- ");
                out.push_str(f);
                out.push('\n');
            }
        }
        if !self.decisions.is_empty() {
            out.push_str("\nDecisions:\n");
            for d in &self.decisions {
                out.push_str("- ");
                out.push_str(d);
                out.push('\n');
            }
        }
        if !self.open_questions.is_empty() {
            out.push_str("\nOpen questions:\n");
            for q in &self.open_questions {
                out.push_str("- ");
                out.push_str(q);
                out.push('\n');
            }
        }
        if !self.tool_results_digest.is_empty() {
            out.push_str("\nTool results digest:\n");
            for d in &self.tool_results_digest {
                out.push_str("- ");
                out.push_str(d);
                out.push('\n');
            }
        }
        if let Some(p) = &self.last_plan_id {
            out.push_str("\nLastPlanId: ");
            out.push_str(p);
            out.push('\n');
        }
        out
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SummaryError {
    #[error("invalid summary json: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("http: {0}")]
    Http(String),
    #[error("provider returned no content. body={0}")]
    NoContent(String),
}

// ── Isolated-runtime summarizer ───────────────────────────────────────────

/// Provider config for the isolated summarizer session. Mirrors the shape
/// `cd-cli` uses for the main Plan/Critic calls but stays standalone so
/// callers can summarize without touching CLI state.
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl SummarizerConfig {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: model.into(),
            timeout_secs: 120,
        }
    }
}

/// The summarizer system prompt. Kept here (not in `cd-prompts`) so the
/// summarizer runtime stays narrow — it doesn't need DS / craft / skills.
pub const SUMMARIZER_SYSTEM_PROMPT: &str = "\
You are the CodeDesign session summarizer. You are running in an ISOLATED \
session whose token budget does NOT touch the main session.

Read the transcript below and return a JSON object with EXACTLY these fields:
{
  \"facts\":            [string, ...],  // stable facts: platform, brand, constraints, DS ref
  \"decisions\":        [string, ...],  // direction picked, skills activated, gates overridden
  \"openQuestions\":    [string, ...],  // user asks not yet answered
  \"toolResultsDigest\":[string, ...],  // one-line digests of noteworthy tool results
  \"lastPlanId\":       string | null
}

RULES:
- Output JSON only, no markdown, no commentary.
- Preserve every explicit user ask and stated constraint as a fact OR decision.
- Keep it compact: each entry ≤ 120 chars.
- Never invent facts not present in the transcript.";

/// Run the summarizer in an isolated session and return a validated
/// `SessionSummary`. Synchronous (uses ureq) to match the rest of the CLI.
///
/// The transcript should be a flat string with one message per line (see
/// [`flatten_transcript`]). Tool-use/tool-result pairs should be kept as
/// adjacent lines so the summarizer can digest them together.
pub fn summarize_transcript(
    cfg: &SummarizerConfig,
    transcript: &str,
) -> Result<SessionSummary, SummaryError> {
    let base = cfg.base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") || base.ends_with("/v1/") {
        format!("{}/chat/completions", base.trim_end_matches('/'))
    } else {
        format!("{}/v1/chat/completions", base)
    };

    let body = json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": SUMMARIZER_SYSTEM_PROMPT},
            {"role": "user",   "content": transcript}
        ],
        "response_format": {"type": "json_object"}
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build();
    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| SummaryError::Http(e.to_string()))?;

    let json: Value = resp.into_json().map_err(|e| SummaryError::Http(format!("decode: {e}")))?;
    let content = json
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| SummaryError::NoContent(json.to_string()))?;
    SessionSummary::from_json(content)
}

/// Flatten a list of `(role, content)` messages into the one-line-per-turn
/// text the summarizer expects. Tool results stay adjacent to their calls.
pub fn flatten_transcript<'a, I>(msgs: I) -> String
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut out = String::new();
    for (i, (role, content)) in msgs.into_iter().enumerate() {
        out.push_str(&format!("[{i}|{role}] {content}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_minimal() {
        let s = SessionSummary {
            facts: vec!["platform=mobile".into()],
            decisions: vec!["direction=neutral-modern".into()],
            open_questions: vec![],
            tool_results_digest: vec![],
            last_plan_id: Some("p_abc".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back = SessionSummary::from_json(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn injection_text_includes_sections() {
        let s = SessionSummary {
            facts: vec!["brand=acme".into()],
            decisions: vec![],
            open_questions: vec!["confirm tone?".into()],
            tool_results_digest: vec![],
            last_plan_id: None,
        };
        let t = s.to_injection_text();
        assert!(t.contains("Facts:"));
        assert!(t.contains("brand=acme"));
        assert!(t.contains("Open questions:"));
        assert!(!t.contains("Decisions:"));
    }

    #[test]
    fn flatten_transcript_preserves_order() {
        let out = flatten_transcript([
            ("user", "hi"),
            ("assistant", "hello"),
            ("tool", "{\"ok\":true}"),
        ]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("[0|user]"));
        assert!(lines[2].starts_with("[2|tool]"));
    }

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-summarizer");
    }
}
