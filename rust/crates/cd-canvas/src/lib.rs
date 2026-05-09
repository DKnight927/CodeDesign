//! cd-canvas — DesignPlan IR → Figma Plugin API JavaScript compiler.
//!
//! See /DESIGN.md §5 (use_figma passthrough) and §6 (compile pipeline).
//! The compiler is fully deterministic: given the same DesignPlan input,
//! `compile` returns byte-identical JS output. The produced script is meant
//! to be passed directly to the official Figma Remote MCP `use_figma` tool.
//!
//! Determinism rules:
//!   * No timestamps, no RNG, no HashMap iteration.
//!   * Ref strings (`color.bg.base`, `typo.title.lg`, `button.primary`) are
//!     emitted verbatim; resolution to Figma variable/style IDs is the
//!     responsibility of the in-plugin runtime loader (see §6.2).
//!   * Each frame block is wrapped in `try/catch`; partial failures record
//!     an entry in `errors[]` but do not abort the remaining frames.
//!
//! Safety: string literals embedded into JS go through `js_string` which
//! escapes `\`, `'`, newlines, `<` (for script-context safety), and U+2028/9.

use cd_plan::{DesignPlan, Frame, Layout, LayoutMode, Node};
use std::fmt::Write;

pub const CRATE_NAME: &str = "cd-canvas";

#[derive(Debug, thiserror::Error)]
pub enum CanvasError {
    #[error("format error: {0}")]
    Fmt(#[from] std::fmt::Error),
}

/// Compile a DesignPlan into a self-contained Plugin API JS script.
///
/// The script, when executed inside Figma, will:
///   1. Pre-load every font referenced by text styles and every component
///      referenced by instance nodes (via `__cdPreloadFonts` provided by
///      the plugin shell — see §6.2 runtime contract).
///   2. Create each frame with AutoLayout settings from `layout`.
///   3. Append text + instance children, binding variables by ref.
///   4. Emit a final `{ planId, createdNodeIds, errors }` result via
///      `__cdEmitResult` (the plugin shell owns the transport).
pub fn compile(plan: &DesignPlan) -> Result<String, CanvasError> {
    let mut out = String::with_capacity(2048);
    writeln!(out, "// cd-canvas generated — do not edit by hand")?;
    writeln!(out, "// planId: {}", plan.plan_id)?;
    writeln!(out, "// dsRef:  {}", plan.ds_ref)?;
    writeln!(out, "(async () => {{")?;
    writeln!(out, "  const __cdPlanId = {};", js_string(&plan.plan_id))?;
    writeln!(out, "  const __cdDsRef  = {};", js_string(&plan.ds_ref))?;
    writeln!(out, "  const createdNodeIds = [];")?;
    writeln!(out, "  const errors = [];")?;
    writeln!(out)?;
    writeln!(out, "  await __cdPreloadFonts(__cdDsRef);")?;

    for frame in &plan.frames {
        compile_frame(&mut out, frame)?;
    }

    writeln!(out)?;
    writeln!(
        out,
        "  __cdEmitResult({{ planId: __cdPlanId, createdNodeIds, errors }});"
    )?;
    writeln!(out, "}})();")?;
    Ok(out)
}

fn compile_frame(out: &mut String, frame: &Frame) -> Result<(), CanvasError> {
    let fid = sanitize_id(&frame.local_id);
    writeln!(out)?;
    writeln!(out, "  // frame: {} ({})", frame.local_id, frame.name)?;
    writeln!(out, "  try {{")?;
    writeln!(out, "    const f_{fid} = figma.createFrame();")?;
    writeln!(out, "    f_{fid}.name = {};", js_string(&frame.name))?;
    writeln!(
        out,
        "    f_{fid}.resizeWithoutConstraints({}, {});",
        frame.size.w, frame.size.h
    )?;
    emit_layout(out, &fid, &frame.layout)?;
    if let Some(fill) = &frame.fill_ref {
        writeln!(out, "    await __cdBindFill(f_{fid}, {});", js_string(fill))?;
    }
    writeln!(
        out,
        "    await __cdPlacePage(f_{fid}, {});",
        js_string(&frame.page)
    )?;
    writeln!(out, "    createdNodeIds.push(f_{fid}.id);")?;

    for (idx, child) in frame.children.iter().enumerate() {
        compile_child(out, &fid, idx, child)?;
    }

    writeln!(out, "  }} catch (e) {{")?;
    writeln!(
        out,
        "    errors.push({{ frame: {}, message: String(e && e.message || e) }});",
        js_string(&frame.local_id)
    )?;
    writeln!(out, "  }}")?;
    Ok(())
}

fn emit_layout(out: &mut String, fid: &str, layout: &Layout) -> Result<(), CanvasError> {
    let mode = match layout.mode {
        LayoutMode::Vertical => "VERTICAL",
        LayoutMode::Horizontal => "HORIZONTAL",
        LayoutMode::None => "NONE",
    };
    writeln!(out, "    f_{fid}.layoutMode = {};", js_string(mode))?;
    writeln!(
        out,
        "    await __cdBindPadding(f_{fid}, {});",
        js_string(&layout.padding)
    )?;
    if let Some(gap) = &layout.gap {
        writeln!(out, "    await __cdBindGap(f_{fid}, {});", js_string(gap))?;
    }
    Ok(())
}

fn compile_child(
    out: &mut String,
    parent_fid: &str,
    idx: usize,
    node: &Node,
) -> Result<(), CanvasError> {
    let cid = format!("{parent_fid}_{idx}");
    match node {
        Node::Text {
            style_ref,
            fill_ref,
            content,
        } => {
            writeln!(out, "    const t_{cid} = figma.createText();")?;
            writeln!(
                out,
                "    await __cdBindTextStyle(t_{cid}, {});",
                js_string(style_ref)
            )?;
            writeln!(out, "    t_{cid}.characters = {};", js_string(content))?;
            writeln!(
                out,
                "    await __cdBindFill(t_{cid}, {});",
                js_string(fill_ref)
            )?;
            writeln!(out, "    f_{parent_fid}.appendChild(t_{cid});")?;
            writeln!(out, "    createdNodeIds.push(t_{cid}.id);")?;
        }
        Node::Instance {
            component_ref,
            props,
        } => {
            writeln!(
                out,
                "    const i_{cid} = await __cdCreateInstance({});",
                js_string(component_ref)
            )?;
            let props_json =
                serde_json::to_string(props).unwrap_or_else(|_| "{}".to_string());
            writeln!(out, "    await __cdApplyProps(i_{cid}, {props_json});")?;
            writeln!(out, "    f_{parent_fid}.appendChild(i_{cid});")?;
            writeln!(out, "    createdNodeIds.push(i_{cid}.id);")?;
        }
        Node::Frame(inner) => {
            let inner_fid = sanitize_id(&inner.local_id);
            writeln!(out, "    const f_{inner_fid} = figma.createFrame();")?;
            writeln!(
                out,
                "    f_{inner_fid}.name = {};",
                js_string(&inner.name)
            )?;
            writeln!(
                out,
                "    f_{inner_fid}.resizeWithoutConstraints({}, {});",
                inner.size.w, inner.size.h
            )?;
            emit_layout(out, &inner_fid, &inner.layout)?;
            if let Some(fill) = &inner.fill_ref {
                writeln!(
                    out,
                    "    await __cdBindFill(f_{inner_fid}, {});",
                    js_string(fill)
                )?;
            }
            for (j, sub) in inner.children.iter().enumerate() {
                compile_child(out, &inner_fid, j, sub)?;
            }
            writeln!(out, "    f_{parent_fid}.appendChild(f_{inner_fid});")?;
            writeln!(out, "    createdNodeIds.push(f_{inner_fid}.id);")?;
        }
    }
    Ok(())
}

/// Sanitize a DesignPlan localId for use as a JS identifier fragment.
/// Non-alphanumeric characters become `_`. A leading digit gets a `_` prefix.
fn sanitize_id(raw: &str) -> String {
    let mut s = String::with_capacity(raw.len() + 1);
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            s.push(ch);
        } else {
            s.push('_');
        }
    }
    if s.is_empty() {
        return "_".to_string();
    }
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s.insert(0, '_');
    }
    s
}

/// Emit a JS single-quoted string literal with full escaping.
fn js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\x3c"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_PLAN: &str =
        include_str!("../../../evals/phase0/use-figma-compiler/cases/minimal-plan.json");

    fn minimal_plan() -> DesignPlan {
        DesignPlan::from_json(MINIMAL_PLAN).expect("minimal plan parses")
    }

    #[test]
    fn compile_is_deterministic() {
        let plan = minimal_plan();
        let a = compile(&plan).expect("compile ok");
        let b = compile(&plan).expect("compile ok");
        assert_eq!(a, b, "compiler must be byte-deterministic");
    }

    #[test]
    fn compile_emits_plan_header_and_frames() {
        let plan = minimal_plan();
        let js = compile(&plan).expect("compile ok");

        assert!(js.contains("planId: plan_s02_minimal"));
        assert!(js.contains("dsRef:  default@1.0.0"));
        assert!(js.contains("__cdPreloadFonts"));
        assert!(js.contains("__cdEmitResult"));

        let frame_count = js.matches("figma.createFrame()").count();
        assert_eq!(frame_count, 3, "expected 3 top-level frames");

        assert_eq!(js.matches("try {").count(), 3);
        assert_eq!(js.matches("} catch (e) {").count(), 3);
    }

    #[test]
    fn compile_binds_refs_not_raw_values() {
        let plan = minimal_plan();
        let js = compile(&plan).expect("compile ok");

        assert!(js.contains("__cdBindFill"));
        assert!(js.contains("'color.bg.base'"));
        assert!(js.contains("'typo.title.lg'"));
        assert!(js.contains("'button.primary'"));

        assert!(!js.contains('#'), "no raw hex allowed in compiled JS");
    }

    #[test]
    fn compile_creates_children_in_order() {
        let plan = minimal_plan();
        let js = compile(&plan).expect("compile ok");

        // Text children are emitted as JS single-quoted strings; instance
        // props are emitted as a JSON blob (double-quoted strings).
        let pos_title = js.find("'欢迎'").expect("title text");
        let pos_sub = js.find("'开始配置你的第一个项目'").expect("sub text");
        let pos_btn = js.find("\"label\":\"开始\"").expect("button label");
        assert!(pos_title < pos_sub && pos_sub < pos_btn);
    }

    #[test]
    fn js_string_escapes_quotes_and_newlines() {
        assert_eq!(js_string("it's\n<ok>"), "'it\\'s\\n\\x3cok>'");
    }

    #[test]
    fn sanitize_id_handles_dots_and_leading_digit() {
        assert_eq!(sanitize_id("f1"), "f1");
        assert_eq!(sanitize_id("frame.a-b"), "frame_a_b");
        assert_eq!(sanitize_id("1x"), "_1x");
    }

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-canvas");
    }
}
