//! 5-dimensional critique rubric (P3.2).
//!
//! The Critic LLM emits a JSON object via the `critic_emit` tool. This
//! module owns:
//!   1. The embedded prompt text (`RUBRIC_MD`) carrying the five
//!      dimensions, discipline rules, and output contract.
//!   2. The JSON Schema for `critic_emit` (`critic_emit_schema`).
//!   3. Pure validation (`validate_report`) that enforces the
//!      discipline rules the LLM may otherwise skip (e.g. worst-of-5,
//!      `why_not_5` / `why_not_7`, innovation cap ≤ 8, banned vacuous
//!      adjectives in evidence).
//!
//! No LLM calls happen here — deterministic gate only.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

/// Embedded rubric prompt. Roles that need it (Critic) paste this
/// verbatim. Changes to `assets/prompts/critique.md` take effect at
/// next build via `include_str!`.
pub const RUBRIC_MD: &str = include_str!("../../../assets/prompts/critique.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dimension {
    Philosophy,
    Hierarchy,
    Craft,
    Functionality,
    Innovation,
}

impl Dimension {
    pub const ALL: [Dimension; 5] = [
        Dimension::Philosophy,
        Dimension::Hierarchy,
        Dimension::Craft,
        Dimension::Functionality,
        Dimension::Innovation,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Dimension::Philosophy    => "philosophy",
            Dimension::Hierarchy     => "hierarchy",
            Dimension::Craft         => "craft",
            Dimension::Functionality => "functionality",
            Dimension::Innovation    => "innovation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    /// 1–2
    Broken,
    /// 3–4
    NeedsWork,
    /// 5–6
    Acceptable,
    /// 7–8
    Good,
    /// 9–10 — forbidden except for landmark work (and reserved for
    /// dimensions other than `innovation`).
    Exemplary,
}

impl Band {
    #[must_use]
    pub fn from_score(score: u8) -> Option<Self> {
        Some(match score {
            1..=2  => Band::Broken,
            3..=4  => Band::NeedsWork,
            5..=6  => Band::Acceptable,
            7..=8  => Band::Good,
            9..=10 => Band::Exemplary,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub score: u8,
    pub band: Band,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_not_5: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_not_7: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixOp {
    pub op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(rename = "instanceRef", default, skip_serializing_if = "Option::is_none")]
    pub instance_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub props: Option<Value>,
    #[serde(flatten, default)]
    pub extra: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricFinding {
    pub dimension: Dimension,
    pub severity: String, // P0 / P1 / P2
    #[serde(rename = "nodeId", default)]
    pub node_id: String,
    pub evidence: String,
    #[serde(rename = "fixOps", default)]
    pub fix_ops: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueReport {
    #[serde(rename = "planId", default)]
    pub plan_id: String,
    pub scores: CritiqueScores,
    #[serde(rename = "totalScore")]
    pub total_score: u8,
    #[serde(rename = "worstDimension")]
    pub worst_dimension: Dimension,
    #[serde(default)]
    pub findings: Vec<RubricFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueScores {
    pub philosophy: DimensionScore,
    pub hierarchy: DimensionScore,
    pub craft: DimensionScore,
    pub functionality: DimensionScore,
    pub innovation: DimensionScore,
}

impl CritiqueScores {
    #[must_use]
    pub fn by_dimension(&self, d: Dimension) -> &DimensionScore {
        match d {
            Dimension::Philosophy    => &self.philosophy,
            Dimension::Hierarchy     => &self.hierarchy,
            Dimension::Craft         => &self.craft,
            Dimension::Functionality => &self.functionality,
            Dimension::Innovation    => &self.innovation,
        }
    }

    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = (Dimension, &DimensionScore)> {
        Dimension::ALL
            .into_iter()
            .map(move |d| (d, self.by_dimension(d)))
    }
}

#[derive(Debug, Error)]
pub enum RubricError {
    #[error("json decode: {0}")]
    Decode(String),
    #[error("score {0} for `{1}` out of range (1..=10)")]
    OutOfRange(u8, &'static str),
    #[error("innovation score {0} exceeds cap 8 (reserved)")]
    InnovationCap(u8),
    #[error("dimension `{0}` has score {1} but missing required `why_not_5`")]
    MissingWhyNot5(&'static str, u8),
    #[error("dimension `{0}` has score {1} but missing required `why_not_7`")]
    MissingWhyNot7(&'static str, u8),
    #[error("totalScore {claimed} disagrees with worst-of-5 {actual}")]
    TotalScoreMismatch { claimed: u8, actual: u8 },
    #[error("worstDimension `{claimed}` disagrees with computed `{actual}`")]
    WorstDimensionMismatch { claimed: String, actual: &'static str },
    #[error("band `{band}` does not match score {score} for `{dim}`")]
    BandMismatch { dim: &'static str, score: u8, band: String },
    #[error("vacuous adjective `{0}` in evidence — rewrite with concrete observation")]
    VacuousEvidence(&'static str),
    #[error("finding evidence is empty")]
    EmptyEvidence,
}

/// Banned vacuous adjectives that must not appear in `evidence`.
/// Keeps the Critic honest — see critique.md "禁用空话".
pub const BANNED_EVIDENCE_PHRASES: &[&str] = &[
    "looks good",
    "feels clean",
    "professional",
    "modern",
    "sleek",
    "polished",
    "clean look",
    "nice touch",
];

/// Parse + validate a CritiqueReport JSON, enforcing the discipline
/// rules declared in `critique.md`.
///
/// Returns the parsed report only when every rule holds.
pub fn validate_report(v: &Value) -> Result<CritiqueReport, RubricError> {
    let report: CritiqueReport =
        serde_json::from_value(v.clone()).map_err(|e| RubricError::Decode(e.to_string()))?;

    for (dim, s) in report.scores.iter() {
        check_dim_score(dim, s)?;
    }

    let (actual_worst_dim, actual_worst_score) = worst_of_five(&report.scores);
    if report.total_score != actual_worst_score {
        return Err(RubricError::TotalScoreMismatch {
            claimed: report.total_score,
            actual: actual_worst_score,
        });
    }
    if report.worst_dimension != actual_worst_dim {
        return Err(RubricError::WorstDimensionMismatch {
            claimed: report.worst_dimension.as_str().to_owned(),
            actual: actual_worst_dim.as_str(),
        });
    }

    for f in &report.findings {
        if f.evidence.trim().is_empty() {
            return Err(RubricError::EmptyEvidence);
        }
        let ev_low = f.evidence.to_ascii_lowercase();
        for banned in BANNED_EVIDENCE_PHRASES {
            if ev_low.contains(banned) {
                return Err(RubricError::VacuousEvidence(banned));
            }
        }
    }

    Ok(report)
}

fn check_dim_score(dim: Dimension, s: &DimensionScore) -> Result<(), RubricError> {
    let name = dim.as_str_static();
    if !(1..=10).contains(&s.score) {
        return Err(RubricError::OutOfRange(s.score, name));
    }
    if dim == Dimension::Innovation && s.score > 8 {
        return Err(RubricError::InnovationCap(s.score));
    }
    // band must match score
    let expected = Band::from_score(s.score).expect("range checked");
    if expected != s.band {
        return Err(RubricError::BandMismatch {
            dim: name,
            score: s.score,
            band: format!("{:?}", s.band),
        });
    }
    if s.score >= 6 && s.why_not_5.as_deref().map_or(true, str::is_empty) {
        return Err(RubricError::MissingWhyNot5(name, s.score));
    }
    if s.score >= 8 && s.why_not_7.as_deref().map_or(true, str::is_empty) {
        return Err(RubricError::MissingWhyNot7(name, s.score));
    }
    Ok(())
}

fn worst_of_five(scores: &CritiqueScores) -> (Dimension, u8) {
    let mut worst = (Dimension::Philosophy, scores.philosophy.score);
    for (d, s) in scores.iter() {
        if s.score < worst.1 {
            worst = (d, s.score);
        }
    }
    worst
}

impl Dimension {
    fn as_str_static(self) -> &'static str {
        self.as_str()
    }
}

/// JSON schema for the `critic_emit` tool, matching `CritiqueReport`.
#[must_use]
pub fn critic_emit_schema() -> Value {
    let dim_enum = json!(["philosophy","hierarchy","craft","functionality","innovation"]);
    let band_enum = json!(["broken","needs_work","acceptable","good","exemplary"]);

    let dim_score = |dim: &str, cap: u8| {
        json!({
            "type": "object",
            "required": ["score", "band"],
            "properties": {
                "score": {"type": "integer", "minimum": 1, "maximum": cap},
                "band":  {"type": "string", "enum": band_enum},
                "why_not_5": {"type": "string"},
                "why_not_7": {"type": "string"}
            },
            "description": format!("{} dimension score; if score>=6 why_not_5 is REQUIRED, if score>=8 why_not_7 is REQUIRED", dim),
            "additionalProperties": false
        })
    };

    json!({
        "type": "function",
        "function": {
            "name": "critic_emit",
            "description": "Emit a 5-dimensional critique report. totalScore = min of the 5 scores (worst-of-5).",
            "parameters": {
                "type": "object",
                "required": ["planId", "scores", "totalScore", "worstDimension", "findings"],
                "properties": {
                    "planId":         {"type": "string"},
                    "scores": {
                        "type": "object",
                        "required": ["philosophy","hierarchy","craft","functionality","innovation"],
                        "properties": {
                            "philosophy":    dim_score("philosophy", 10),
                            "hierarchy":     dim_score("hierarchy", 10),
                            "craft":         dim_score("craft", 10),
                            "functionality": dim_score("functionality", 10),
                            "innovation":    dim_score("innovation", 8)
                        },
                        "additionalProperties": false
                    },
                    "totalScore":     {"type": "integer", "minimum": 1, "maximum": 10},
                    "worstDimension": {"type": "string", "enum": dim_enum},
                    "findings": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["dimension", "severity", "evidence"],
                            "properties": {
                                "dimension": {"type": "string", "enum": dim_enum},
                                "severity":  {"type": "string", "enum": ["P0","P1","P2"]},
                                "nodeId":    {"type": "string"},
                                "evidence":  {"type": "string"},
                                "fixOps":    {"type": "array"}
                            },
                            "additionalProperties": false
                        }
                    }
                },
                "additionalProperties": false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn good_report() -> Value {
        json!({
            "planId": "p-1",
            "scores": {
                "philosophy":    {"score": 5, "band": "acceptable"},
                "hierarchy":     {"score": 7, "band": "good", "why_not_5": "clear primary-CTA vs. secondary link hierarchy", "why_not_7": "could still tighten label-field pairing"},
                "craft":         {"score": 6, "band": "acceptable", "why_not_5": "consistent 8pt grid, no orphan alignments"},
                "functionality": {"score": 4, "band": "needs_work"},
                "innovation":    {"score": 5, "band": "acceptable"}
            },
            "totalScore": 4,
            "worstDimension": "functionality",
            "findings": [
                {"dimension": "functionality", "severity": "P1", "nodeId": "1:2", "evidence": "error state has no retry affordance"}
            ]
        })
    }

    #[test]
    fn valid_report_passes() {
        let r = validate_report(&good_report()).unwrap();
        assert_eq!(r.total_score, 4);
        assert_eq!(r.worst_dimension, Dimension::Functionality);
    }

    #[test]
    fn innovation_cap_enforced() {
        let mut r = good_report();
        r["scores"]["innovation"] = json!({"score": 9, "band": "exemplary", "why_not_5": "x", "why_not_7": "y"});
        assert!(matches!(validate_report(&r), Err(RubricError::InnovationCap(9))));
    }

    #[test]
    fn score_6_requires_why_not_5() {
        let mut r = good_report();
        r["scores"]["craft"] = json!({"score": 6, "band": "acceptable"});
        assert!(matches!(validate_report(&r), Err(RubricError::MissingWhyNot5(_, 6))));
    }

    #[test]
    fn score_8_requires_why_not_7() {
        let mut r = good_report();
        r["scores"]["hierarchy"] = json!({"score": 8, "band": "good", "why_not_5": "x"});
        assert!(matches!(validate_report(&r), Err(RubricError::MissingWhyNot7(_, 8))));
    }

    #[test]
    fn total_must_equal_worst() {
        let mut r = good_report();
        r["totalScore"] = json!(7);
        assert!(matches!(
            validate_report(&r),
            Err(RubricError::TotalScoreMismatch { claimed: 7, actual: 4 })
        ));
    }

    #[test]
    fn worst_dim_must_match() {
        let mut r = good_report();
        r["worstDimension"] = json!("craft");
        assert!(matches!(
            validate_report(&r),
            Err(RubricError::WorstDimensionMismatch { .. })
        ));
    }

    #[test]
    fn band_must_match_score() {
        let mut r = good_report();
        r["scores"]["philosophy"] = json!({"score": 5, "band": "good"});
        assert!(matches!(
            validate_report(&r),
            Err(RubricError::BandMismatch { .. })
        ));
    }

    #[test]
    fn vacuous_adjective_rejected() {
        let mut r = good_report();
        r["findings"] = json!([{
            "dimension": "craft",
            "severity": "P2",
            "evidence": "overall looks good, feels clean"
        }]);
        assert!(matches!(validate_report(&r), Err(RubricError::VacuousEvidence(_))));
    }

    #[test]
    fn empty_evidence_rejected() {
        let mut r = good_report();
        r["findings"] = json!([{"dimension": "craft", "severity": "P2", "evidence": "   "}]);
        assert!(matches!(validate_report(&r), Err(RubricError::EmptyEvidence)));
    }

    #[test]
    fn schema_compiles_as_valid_json() {
        let s = critic_emit_schema();
        assert_eq!(s["function"]["name"], "critic_emit");
    }

    #[test]
    fn rubric_md_embedded() {
        assert!(RUBRIC_MD.contains("Philosophy consistency"));
        assert!(RUBRIC_MD.contains("worst-of-5"));
    }
}
