//! cd-prompts — role prompts for the CodeDesign agent.
//!
//! Five roles make up the pipeline. Each exposes three things:
//!   1. A static system-prompt preamble (role identity + hard rules).
//!   2. A DS-bound extension — produced at runtime via cd-ds/cd-craft.
//!   3. A visible tool surface — `Vec<String>` (names only).
//!
//! Tool surface guards (see memory `feedback_no_shortcuts.md`):
//!   - `bash` is NEVER in any role's visible tools (P1.2).
//!   - `edit` is present only if the caller passes a whitelist (P1.1);
//!     absent whitelist → absent tool.
//!
//! Turn-1 hard rule (for the Interpreter role):
//!   The first user interaction may only produce (a) one prose line and
//!   (b) one `ask_user_question` call. No file I/O, no bash (which is
//!   not visible anyway), no thinking blocks. Enforced as prompt text;
//!   post-hoc check is a separate crate (cd-brief).

use serde::Serialize;

use cd_craft::{direction_summaries, rule_text};
use cd_ds::DesignSystem;
use cd_skill::Skill;

pub const CRATE_NAME: &str = "cd-prompts";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Interpreter,
    Planner,
    Critic,
    Fixer,
    Extractor,
}

impl Role {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Role::Interpreter => "interpreter",
            Role::Planner => "planner",
            Role::Critic => "critic",
            Role::Fixer => "fixer",
            Role::Extractor => "extractor",
        }
    }
}

/// A fully composed prompt ready to send to the provider.
#[derive(Debug, Clone, Serialize)]
pub struct RolePrompt {
    pub role: Role,
    /// System prompt contents (identity + DS + craft + hard rules).
    pub system: String,
    /// Tools visible to the model, by name. Never contains `bash`.
    pub tools: Vec<String>,
    /// True iff this role is allowed to call `ask_user_question`.
    pub allow_ask_user: bool,
}

/// Options the CLI forwards when building a prompt.
#[derive(Debug, Clone, Default)]
pub struct PromptOptions {
    /// Whitelist of path prefixes the `edit` tool may write to. An empty
    /// or missing whitelist suppresses the `edit` tool entirely.
    pub edit_whitelist: Vec<String>,
    /// If true, include the visual-direction picker block (Turn-1 only).
    pub include_vdp: bool,
    /// Optional user-chosen direction id (pins subsequent roles to it).
    pub chosen_direction: Option<String>,
    /// Skills whose requirements pass `check_requirements` against the DS.
    /// Each is injected as a prompt fragment after craft/rubric/case blocks.
    pub skills: Vec<Skill>,
}

/// Forbidden tool names — enforced at prompt-build time so the model
/// never sees them, regardless of how the underlying tool crate is wired.
pub const FORBIDDEN_TOOLS: &[&str] = &["bash", "PowerShell", "shell", "run_command"];

/// Build a prompt for the given role, bound to a loaded DS.
pub fn build(role: Role, ds: &DesignSystem, opts: &PromptOptions) -> RolePrompt {
    let mut system = String::new();
    system.push_str(role_identity(role));
    system.push_str("\n\n");
    system.push_str(&cd_ds::prompt::plan_binding(ds));
    system.push('\n');

    // Craft rules are embedded for Planner / Critic / Fixer.
    if matches!(role, Role::Planner | Role::Critic | Role::Fixer) {
        for r in &ds.constraints.craft.requires {
            if let Some(txt) = rule_text(r, None) {
                system.push_str(&format!("\n# craft: {r}\n{}\n", txt.trim()));
            }
        }
    }

    // Critic: embed the 5-dimensional rubric prompt verbatim.
    if role == Role::Critic {
        system.push_str("\n\n# 5-Dimensional Critique Rubric\n");
        system.push_str(cd_critique::RUBRIC_MD);
    }

    // Case library (P3.5): Plan / Critic see case titles as references.
    if matches!(role, Role::Planner | Role::Critic) && !ds.cases.cases.is_empty() {
        system.push_str("\n# Case library (DS-curated references)\n");
        for c in &ds.cases.cases {
            let tags = if c.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", c.tags.join(","))
            };
            let note = if c.note.is_empty() { String::new() } else { format!(" — {}", c.note) };
            system.push_str(&format!("- {} ({}){}{}\n", c.title, c.id, tags, note));
        }
        system.push_str("Use these as calibrators for what \"good\" looks like in this DS; do not copy them literally.\n");
    }

    // Interpreter: Turn-1 hard rule + VDP.
    if role == Role::Interpreter {
        system.push_str("\n# Turn-1 hard rule\n");
        system.push_str(TURN1_RULE);
        if opts.include_vdp {
            system.push_str("\n\n# Visual Direction Picker (present these five options to the user)\n");
            for (id, one_line) in direction_summaries() {
                system.push_str(&format!("- [{id}] {one_line}\n"));
            }
        }
    }

    // Pin chosen direction on downstream roles.
    if let Some(dir) = &opts.chosen_direction {
        if matches!(role, Role::Planner | Role::Critic | Role::Fixer) {
            system.push_str(&format!(
                "\n# Visual direction (locked)\nid: {dir}\nFollow this direction's posture, palette, typography, and rules.\n"
            ));
        }
    }

    let (tools, allow_ask_user) = tool_surface(role, opts);

    // Skills (P3.3/P3.4): append per-skill fragment with tool-augmentation
    // filtered to the active role's tool surface.
    for skill in &opts.skills {
        system.push_str(&skill.prompt_fragment(&tools));
    }

    RolePrompt { role, system, tools, allow_ask_user }
}

// ── role text ──────────────────────────────────────────────────────────────

fn role_identity(role: Role) -> &'static str {
    match role {
        Role::Interpreter => INTERPRETER_IDENTITY,
        Role::Planner     => PLANNER_IDENTITY,
        Role::Critic      => CRITIC_IDENTITY,
        Role::Fixer       => FIXER_IDENTITY,
        Role::Extractor   => EXTRACTOR_IDENTITY,
    }
}

const INTERPRETER_IDENTITY: &str = "\
You are the Interpreter role of CodeDesign. You translate a fuzzy PRD
into a structured Brief (users, jobs, screens, constraints, success
signals). You DO NOT produce designs. You clarify by asking one tight
multi-question form, not a back-and-forth stream.
";

const PLANNER_IDENTITY: &str = "\
You are the Planner role of CodeDesign. You emit a DesignPlan IR via
the `plan_emit` tool. You do NOT answer in prose. Refs only — never
raw hex, px, or font names. Every ref must resolve in the DS binding
block below. You produce the smallest plan that covers the Brief.
";

const CRITIC_IDENTITY: &str = "\
You are the Critic role of CodeDesign. Given a DesignPlan + Figma
readback JSON, you score the result against the DS enforcement rules
and craft requires. You emit a list of findings, each with
{severity, id, message, fix, path}. You do NOT rewrite the plan —
the Fixer does that. Deterministic rules (anti-slop) have already
run; focus on judgement calls.
";

const FIXER_IDENTITY: &str = "\
You are the Fixer role of CodeDesign. Given a DesignPlan and a list
of Critic findings, you emit a patched DesignPlan via `plan_emit`.
You change ONLY what the findings require. You do not invent new
screens or features.
";

const EXTRACTOR_IDENTITY: &str = "\
You are the Extractor role of CodeDesign. Given screenshots or a
Figma scene, you produce a DesignSystem draft (tokens + components +
constraints) that future roles can bind against. You are conservative:
unfamiliar patterns go into `patterns.json`, not components.
";

const TURN1_RULE: &str = "\
- Emit exactly ONE prose line acknowledging the user intent.
- Then call `ask_user_question` ONCE with a structured multi-question form.
- Do NOT read/write files. Do NOT call any other tool. Do NOT think out loud.
- If the PRD is already unambiguous, still ask at least one confirming question
  (target platform OR success signal) before handing off to the Planner.
";

// ── tool surface ───────────────────────────────────────────────────────────

fn tool_surface(role: Role, opts: &PromptOptions) -> (Vec<String>, bool) {
    let mut tools: Vec<&'static str> = match role {
        Role::Interpreter => vec!["ask_user_question"],
        Role::Planner     => vec!["plan_emit", "image_understand"],
        Role::Critic      => vec!["critic_emit", "image_understand"],
        Role::Fixer       => vec!["plan_emit"],
        Role::Extractor   => vec!["ds_emit", "image_understand"],
    };

    // P1.1: edit tool opt-in via whitelist
    if !opts.edit_whitelist.is_empty() && matches!(role, Role::Extractor) {
        tools.push("edit");
    }

    // P1.2: never expose shell tools regardless of role
    tools.retain(|t| !FORBIDDEN_TOOLS.contains(t));

    let allow_ask_user = tools.contains(&"ask_user_question");
    (tools.into_iter().map(str::to_owned).collect(), allow_ask_user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn default_ds() -> DesignSystem {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .join("assets/design-systems/default");
        DesignSystem::load(root).unwrap()
    }

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-prompts");
    }

    #[test]
    fn no_role_exposes_bash() {
        let ds = default_ds();
        for r in [Role::Interpreter, Role::Planner, Role::Critic, Role::Fixer, Role::Extractor] {
            let p = build(r, &ds, &PromptOptions::default());
            for forbidden in FORBIDDEN_TOOLS {
                assert!(
                    !p.tools.iter().any(|t| t == forbidden),
                    "role {r:?} exposed forbidden tool {forbidden}"
                );
            }
        }
    }

    #[test]
    fn interpreter_has_turn1_and_ask() {
        let ds = default_ds();
        let p = build(Role::Interpreter, &ds, &PromptOptions { include_vdp: true, ..Default::default() });
        assert!(p.system.contains("Turn-1 hard rule"));
        assert!(p.system.contains("Visual Direction Picker"));
        assert!(p.tools.iter().any(|t| t == "ask_user_question"));
        assert!(p.allow_ask_user);
    }

    #[test]
    fn planner_has_plan_emit_not_ask() {
        let ds = default_ds();
        let p = build(Role::Planner, &ds, &PromptOptions::default());
        assert!(p.tools.iter().any(|t| t == "plan_emit"));
        assert!(!p.allow_ask_user);
    }

    #[test]
    fn edit_tool_requires_whitelist() {
        let ds = default_ds();
        let p_off = build(Role::Extractor, &ds, &PromptOptions::default());
        assert!(p_off.tools.iter().all(|t| t != "edit"));

        let p_on = build(
            Role::Extractor,
            &ds,
            &PromptOptions {
                edit_whitelist: vec!["./out/".into()],
                ..Default::default()
            },
        );
        assert!(p_on.tools.iter().any(|t| t == "edit"));
    }

    #[test]
    fn planner_sees_ds_binding() {
        let ds = default_ds();
        let p = build(Role::Planner, &ds, &PromptOptions::default());
        assert!(p.system.contains("color.brand.primary"));
        assert!(p.system.contains("button.primary"));
    }

    #[test]
    fn chosen_direction_pins_downstream() {
        let ds = default_ds();
        let p = build(
            Role::Planner,
            &ds,
            &PromptOptions {
                chosen_direction: Some("tech-utility".into()),
                ..Default::default()
            },
        );
        assert!(p.system.contains("tech-utility"));
        assert!(p.system.contains("Visual direction (locked)"));
    }

    #[test]
    fn craft_rules_embedded_for_planner() {
        let ds = default_ds();
        let p = build(Role::Planner, &ds, &PromptOptions::default());
        // anti-ai-slop is a required craft rule in default DS
        assert!(p.system.contains("craft: anti-ai-slop"));
    }

    #[test]
    fn active_skill_augments_planner_plan_emit() {
        let ds = default_ds();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .join("assets/skills/screens");
        let skill = cd_skill::Skill::load(root).unwrap();
        let p = build(
            Role::Planner,
            &ds,
            &PromptOptions { skills: vec![skill], ..Default::default() },
        );
        assert!(p.system.contains("Active skill: screens"));
        assert!(p.system.contains("tool augmentation: plan_emit"));
    }

    #[test]
    fn skill_augmentation_hidden_when_tool_absent() {
        let ds = default_ds();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .join("assets/skills/audit");
        let skill = cd_skill::Skill::load(root).unwrap();
        // Planner does NOT see critic_emit, so audit skill's augmentation
        // (which targets critic_emit) should not appear.
        let p = build(
            Role::Planner,
            &ds,
            &PromptOptions { skills: vec![skill], ..Default::default() },
        );
        assert!(!p.system.contains("tool augmentation: critic_emit"));
    }
}
