//! Prompt-snippet formatting for a loaded [`DesignSystem`].
//!
//! These helpers produce the terse, structured system-prompt fragments
//! that the Plan / Critique / Fixer roles splice into their prompts.
//! Format is deterministic and stable — downstream role prompts may
//! assume exact shape for few-shot examples.

use std::fmt::Write as _;

use crate::DesignSystem;

/// The full DS-binding block handed to the Plan role.
///
/// Sections (in order):
///   `# DS meta` — name/version/enforcement
///   `# Tokens (refs)` — every ref grouped by section
///   `# Components` — names with variants summary
///   `# Craft requires` — list from constraints.craft.requires
///   `# Voice` — inlined voice.md (verbatim)
#[must_use]
pub fn plan_binding(ds: &DesignSystem) -> String {
    let mut s = String::new();

    let _ = writeln!(
        s,
        "# DS meta\nname: {}\nversion: {}\nenforcement: {:?}\n",
        ds.meta.name, ds.meta.version, ds.constraints.enforcement
    );

    s.push_str("# Tokens (refs — use these, do not invent)\n");
    for r in ds.tokens.all_refs() {
        let v = ds.value_for_ref(&r).unwrap_or_default();
        let _ = writeln!(s, "- {r} = {v}");
    }
    s.push('\n');

    s.push_str("# Components (use these, do not invent)\n");
    for (name, spec) in &ds.components.items {
        let variants = spec
            .variants
            .iter()
            .map(|(k, vs)| format!("{k}:[{}]", vs.join(",")))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = writeln!(s, "- {name} {{ {variants} }}");
    }
    s.push('\n');

    if !ds.constraints.craft.requires.is_empty() {
        s.push_str("# Craft requires\n");
        for r in &ds.constraints.craft.requires {
            let _ = writeln!(s, "- {r}");
        }
        s.push('\n');
    }

    s.push_str("# Voice\n");
    s.push_str(ds.voice.trim());
    s.push('\n');

    s
}

/// Compact snippet for the Critic role — focuses on enforcement rules
/// rather than the full token inventory (Critic works from readback).
#[must_use]
pub fn critic_binding(ds: &DesignSystem) -> String {
    let c = &ds.constraints;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# DS critique rules\nenforcement: {:?}\na11y.contrastMin: {}\na11y.largeContrastMin: {}\na11y.requireFocusRing: {}\nspacing.base: {}\nspacing.disallowOddPx: {}",
        c.enforcement,
        c.a11y.contrast_min,
        c.a11y.large_contrast_min,
        c.a11y.require_focus_ring,
        c.spacing.base,
        c.spacing.disallow_odd_px,
    );
    if let Some(n) = c.typography.max_sizes_per_frame {
        let _ = writeln!(s, "typography.maxSizesPerFrame: {n}");
    }
    if let Some(n) = c.typography.max_weights_per_frame {
        let _ = writeln!(s, "typography.maxWeightsPerFrame: {n}");
    }
    if let Some(n) = c.color.max_hues_per_frame {
        let _ = writeln!(s, "color.maxHuesPerFrame: {n}");
    }
    if c.color.brand_color_for_interactive_only {
        s.push_str("color.brandColorForInteractiveOnly: true\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn default_ds() -> DesignSystem {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("assets")
            .join("design-systems")
            .join("default");
        DesignSystem::load(root).unwrap()
    }

    #[test]
    fn plan_binding_contains_expected_sections() {
        let ds = default_ds();
        let p = plan_binding(&ds);
        assert!(p.contains("# DS meta"));
        assert!(p.contains("# Tokens"));
        assert!(p.contains("color.bg.base"));
        assert!(p.contains("# Components"));
        assert!(p.contains("button.primary"));
        assert!(p.contains("# Craft requires"));
        assert!(p.contains("anti-ai-slop"));
        assert!(p.contains("# Voice"));
    }

    #[test]
    fn critic_binding_reports_limits() {
        let ds = default_ds();
        let c = critic_binding(&ds);
        assert!(c.contains("contrastMin: 4.5"));
        assert!(c.contains("spacing.disallowOddPx: true"));
    }
}
