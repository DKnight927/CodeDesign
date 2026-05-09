//! cd-plan — DesignPlan IR types + validator hooks.
//!
//! See /DESIGN.md §2.5 (DesignPlan IR) and §4.3 (Gate-1 validator contract).
//! The type model here is the canonical serde representation of what the model
//! emits and what the compiler/validators consume.

use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "cd-plan";

/// Top-level DesignPlan IR. Must round-trip with the JSON schema defined in
/// `/DESIGN.md §2.5`. All value-bearing leaf fields are `*Ref` strings, never
/// raw hex/px/font-name. Raw values trip Gate-1 (`validate_no_raw_values`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignPlan {
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "dsRef")]
    pub ds_ref: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    pub intent: Intent,
    #[serde(rename = "stepKind")]
    pub step_kind: StepKind,
    pub frames: Vec<Frame>,
    #[serde(default, rename = "components_to_create")]
    pub components_to_create: Vec<serde_json::Value>,
    #[serde(default, rename = "tokens_to_create")]
    pub tokens_to_create: Vec<serde_json::Value>,
    #[serde(default, rename = "craft_asserts")]
    pub craft_asserts: Vec<String>,
    #[serde(default, rename = "quality_gates")]
    pub quality_gates: Vec<String>,
    #[serde(default, rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    CreateScreens,
    RefineSelection,
    Restyle,
    AddState,
    ExtractDs,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepKind {
    Pages,
    Skeleton,
    Content,
    States,
    Annotations,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Frame {
    #[serde(rename = "localId")]
    pub local_id: String,
    pub name: String,
    pub page: String,
    pub size: Size,
    pub layout: Layout,
    #[serde(default, rename = "fillRef")]
    pub fill_ref: Option<String>,
    #[serde(default)]
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layout {
    pub mode: LayoutMode,
    pub padding: String,
    #[serde(default)]
    pub gap: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LayoutMode {
    Vertical,
    Horizontal,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node {
    Text {
        #[serde(rename = "styleRef")]
        style_ref: String,
        #[serde(rename = "fillRef")]
        fill_ref: String,
        content: String,
    },
    Instance {
        #[serde(rename = "componentRef")]
        component_ref: String,
        #[serde(default)]
        props: serde_json::Map<String, serde_json::Value>,
    },
    Frame(Box<Frame>),
}

impl DesignPlan {
    /// Parse a DesignPlan from a JSON string. Returns a structured error on
    /// schema violations (missing fields, wrong variants, etc.).
    pub fn from_json(s: &str) -> Result<Self, PlanError> {
        serde_json::from_str(s).map_err(PlanError::InvalidJson)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("invalid plan json: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_PLAN: &str = include_str!(
        "../../../evals/phase0/use-figma-compiler/cases/minimal-plan.json"
    );

    #[test]
    fn parse_minimal_plan() {
        let plan = DesignPlan::from_json(MINIMAL_PLAN).expect("parse ok");
        assert_eq!(plan.plan_id, "plan_s02_minimal");
        assert_eq!(plan.intent, Intent::CreateScreens);
        assert_eq!(plan.step_kind, StepKind::Skeleton);
        assert_eq!(plan.frames.len(), 3);
        assert_eq!(plan.frames[0].children.len(), 3);
    }

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-plan");
    }
}
