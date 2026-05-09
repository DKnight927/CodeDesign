//! `codedesign` — terminal design agent CLI.
//!
//! v0.0.3: real MCP passthrough to Figma.
//!
//! Commands:
//!   codedesign design "<PRD>"       — Plan → compile → push to Figma via MCP
//!   codedesign compile <plan.json>  — IR → Plugin JS, written to disk (debug aid)
//!   codedesign doctor               — probe env, capture & save Figma PAT
//!   codedesign --version | --help
//!
//! End-to-end pipeline for `design`:
//!   1. Read CODEDESIGN_DEEPSEEK_{API_KEY,BASE_URL,MODEL} from env
//!      (DEEPSEEK_* accepted with a deprecation warning for one release)
//!   2. Build Plan-role prompt with tight plan_emit JSON Schema
//!   3. Call provider, parse tool_call into cd_plan::DesignPlan
//!   4. Compile with cd_canvas -> self-contained Plugin JS (no shim)
//!   5. Spawn figma-console-mcp via cd_figma_mcp, bundle shim + JS,
//!      call the `figma_execute` tool. Hard fail if the MCP layer
//!      is not healthy; no disk fallback.
//!
//! Auth model:
//!   - Figma PAT resolved from env (`CODEDESIGN_FIGMA_TOKEN` or
//!     `FIGMA_ACCESS_TOKEN`) or `~/.codedesign/auth.toml`.
//!   - `codedesign doctor` walks the user through saving one.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, ExitCode};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};

use cd_ds::DesignSystem;
use cd_figma_mcp::{
    auth, bundle_plugin_js, client::ConsoleBridgeConfig, ConsoleBridgeClient, McpClient,
    TOOL_FIGMA_EXECUTE,
};
use cd_prompts::{Role, PromptOptions};
use cd_skill::Skill;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_BASE: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-chat";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return ExitCode::SUCCESS;
    }

    match args[0].as_str() {
        "--version" | "-V" => {
            println!("codedesign {VERSION}");
            ExitCode::SUCCESS
        }
        "--help" | "-h" | "help" => {
            print_help();
            ExitCode::SUCCESS
        }
        "design" => cmd_design(&args[1..]),
        "critique" => cmd_critique(&args[1..]),
        "summarize" => cmd_summarize(&args[1..]),
        "compile" => cmd_compile(&args[1..]),
        "ds-query" => cmd_ds_query(&args[1..]),
        "ds-validate" => cmd_ds_validate(&args[1..]),
        "prd-parse" => cmd_prd_parse(&args[1..]),
        "open-node" => cmd_open_node(&args[1..]),
        "doctor" => cmd_doctor(&args[1..]),
        other => {
            eprintln!("unknown command: {other}");
            eprintln!();
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        r#"codedesign {VERSION} — terminal design agent

USAGE:
  codedesign design "<PRD>" [--dry-run [--out <file.js>]] [--skill <name> ...]
    Generate a design from the PRD and push it to Figma via MCP
    (figma-console-mcp Local Mode). Requires a Figma PAT and the
    Desktop Bridge Plugin running. See `codedesign doctor`.
    --dry-run writes the bundled plugin JS to disk instead of
    pushing. Default out: ./codedesign-output.js
    --skill activates a SKILL.md from CODEDESIGN_SKILLS_DIR or
    ~/.codedesign/skills (repeatable; e.g. `--skill screens`).

  codedesign compile <plan.json> [--out <file.js>]
    Compile an existing DesignPlan JSON into Plugin JS (Product only,
    no runtime shim). Debugging the compiler seam only.

  codedesign critique <plan.json> <readback.json> [--skill <name> ...]
    Run the 5-dimensional Critic role over a Plan + Figma readback
    and print a validated critique report (worst-of-5 scoring).

  codedesign summarize <transcript.txt> [--out <summary.json>]
    Run the isolated-session summarizer over a flat transcript and
    print / save a SessionSummary JSON. The call runs in a fresh
    chat-completion session (summarizer system prompt + transcript);
    the main session's token budget is untouched.

  codedesign ds-query <query> [--kind token|component|any] [--limit N]
    Fuzzy lookup tokens/components in the active DS.

  codedesign ds-validate <fragment.json>
    Validate every fillRef/styleRef/componentRef/padding/gap in the
    given JSON fragment against the active DS.

  codedesign prd-parse <prd.txt>
    Heuristic PRD → Brief skeleton (platform/screens/users/goals).

  codedesign open-node <fileKey> <nodeId>
    Print a Figma deeplink URL for the given node.

  codedesign doctor
    Check Node.js, Figma Desktop, and the saved PAT. Prompts for a
    PAT if none is configured and saves it to ~/.codedesign/auth.toml.

  codedesign --version | --help

ENV:
  CODEDESIGN_DEEPSEEK_API_KEY    required for `design`
  CODEDESIGN_DEEPSEEK_BASE_URL   default https://api.deepseek.com
  CODEDESIGN_DEEPSEEK_MODEL      default deepseek-chat
  CODEDESIGN_FIGMA_TOKEN         Figma PAT (overrides auth.toml)
  CODEDESIGN_MCP_DEBUG=1         echo figma-console-mcp stderr

  DEEPSEEK_* variants are accepted one more release with a warning.

Docs: https://github.com/<publisher>/codedesign
"#
    );
}

// ---------- design ----------

fn cmd_design(args: &[String]) -> ExitCode {
    let mut prd: Option<String> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut dry_run = false;
    let mut no_critique = false;
    let mut skill_names: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" | "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--out requires a path");
                    return ExitCode::FAILURE;
                }
                out_path = Some(PathBuf::from(&args[i]));
            }
            "--dry-run" => dry_run = true,
            "--no-critique" => no_critique = true,
            "--skill" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--skill requires a skill name");
                    return ExitCode::FAILURE;
                }
                skill_names.push(args[i].clone());
            }
            other if !other.starts_with("--") && prd.is_none() => {
                prd = Some(other.to_string());
            }
            other => {
                eprintln!("unexpected arg: {other}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }
    let Some(prd) = prd else {
        eprintln!("usage: codedesign design \"<PRD>\" [--dry-run [--out <file.js>]]");
        return ExitCode::FAILURE;
    };

    let Some(provider) = resolve_provider() else {
        return ExitCode::FAILURE;
    };

    // Load the DS — bundled default unless CODEDESIGN_DS_DIR is set.
    let ds = match load_ds() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: could not load DesignSystem: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!("[codedesign] provider: {} @ {}", provider.model, provider.base);
    eprintln!("[codedesign] DS: {}@{}", ds.meta.name, ds.meta.version);

    let skills = load_skills(&ds, &skill_names);

    // Start the MCP server BEFORE calling the Plan so the Figma Desktop Bridge
    // plugin has time to discover and connect to the WebSocket server while the
    // (slow) LLM call is in flight.  The plugin only scans 3 times on startup
    // (0 s / 3 s / 6 s) and gives up — if we start the server after the plan
    // finishes, we'll always miss that window.
    //
    // Skip this in --dry-run mode (no Figma push needed).
    let mcp_client = if dry_run {
        None
    } else {
        eprintln!("[codedesign] spawning figma-console-mcp (Local Mode)...");
        match ConsoleBridgeClient::connect(ConsoleBridgeConfig::default()) {
            Ok(c) => {
                eprintln!("[codedesign] MCP server ready — Figma plugin has ~30s to connect");
                Some(c)
            }
            Err(e) => {
                eprintln!("error: could not start MCP client: {e}");
                eprintln!();
                eprintln!("hint: run `codedesign doctor` to diagnose. Most common causes:");
                eprintln!("  - Node.js >= 18 not installed or not on PATH");
                eprintln!("  - Figma PAT missing (CODEDESIGN_FIGMA_TOKEN or ~/.codedesign/auth.toml)");
                eprintln!("  - Desktop Bridge plugin not running in Figma");
                return ExitCode::FAILURE;
            }
        }
    };

    eprintln!("[codedesign] calling Plan role... (this can take 30-180s)");

    let plan_json = match call_plan(&provider, &ds, &prd, &skills) {
        Ok(v) => v,
        Err(e) => {
            if let Some(c) = mcp_client { Box::new(c).shutdown(); }
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let plan = match cd_plan::DesignPlan::from_json(&plan_json) {
        Ok(p) => p,
        Err(e) => {
            if let Some(c) = mcp_client { Box::new(c).shutdown(); }
            eprintln!("error: model returned a Plan that does not match DesignPlan schema");
            eprintln!("detail: {e}");
            eprintln!();
            eprintln!(
                "raw plan JSON (for debugging):\n{}",
                truncate(&plan_json, 2000)
            );
            return ExitCode::FAILURE;
        }
    };

    let product_js = match cd_canvas::compile(&plan) {
        Ok(s) => s,
        Err(e) => {
            if let Some(c) = mcp_client { Box::new(c).shutdown(); }
            eprintln!("error: compiler failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let bundle = bundle_plugin_js(&product_js);

    if dry_run {
        let out = out_path.unwrap_or_else(|| PathBuf::from("./codedesign-output.js"));
        if let Err(e) = fs::write(&out, &bundle) {
            eprintln!("error: could not write {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        eprintln!("[codedesign] --dry-run: wrote {} ({} frames)", out.display(), plan.frames.len());
        return ExitCode::SUCCESS;
    }

    let mut client = match mcp_client {
        Some(c) => c,
        None => unreachable!("mcp_client is None only in dry_run mode"),
    };

    eprintln!("[codedesign] calling {TOOL_FIGMA_EXECUTE} ({} frames)...", plan.frames.len());
    let result = match client.call_tool(
        TOOL_FIGMA_EXECUTE,
        json!({ "code": bundle }),
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: figma_execute failed: {e}");
            Box::new(client).shutdown();
            return ExitCode::FAILURE;
        }
    };

    if let Some(text) = cd_figma_mcp::client::extract_text_content(&result) {
        eprintln!("[codedesign] figma response:\n{}", truncate(&text, 1000));

        // Deterministic anti-slop lint over whatever JSON the plugin
        // returned. Findings are informational — a shipping project
        // wires the Critic role on top; for v0.0.3 we print them.
        if let Ok(readback) = serde_json::from_str::<Value>(&text) {
            if let Ok(rules) = cd_critique::AntiSlopRules::bundled() {
                let findings = cd_critique::lint(&readback, &rules);
                if findings.is_empty() {
                    eprintln!("[codedesign] anti-slop: 0 findings");
                } else {
                    eprintln!("[codedesign] anti-slop: {} findings", findings.len());
                    for f in findings.iter().take(20) {
                        eprintln!(
                            "  [{:?}] {} {} — {} (path: {})",
                            f.severity, f.id, f.message, f.fix, f.path
                        );
                    }
                }
            }

            // 5-dim critique (P3.2) — LLM call, skipped with --no-critique.
            if !no_critique {
                eprintln!("[codedesign] calling Critic role (5-dim rubric)...");
                match run_critique(&provider, &ds, &plan_json, &text, &skills) {
                    Ok(report) => {
                        eprintln!(
                            "[codedesign] critique total {} (worst: {:?})",
                            report.total_score, report.worst_dimension
                        );
                        for (dim, s) in report.scores.iter() {
                            eprintln!("  {:<14} {} ({:?})", dim.as_str(), s.score, s.band);
                        }
                        for f in report.findings.iter().take(10) {
                            eprintln!(
                                "  [{}] {} ({}) — {}",
                                f.severity,
                                f.dimension.as_str(),
                                f.node_id,
                                f.evidence
                            );
                        }
                    }
                    Err(e) => eprintln!("[codedesign] critique skipped: {e}"),
                }
            }
        }
    }
    Box::new(client).shutdown();

    eprintln!();
    eprintln!("[codedesign] done. {} frames pushed to Figma.", plan.frames.len());
    ExitCode::SUCCESS
}

// ---------- critique ----------

fn cmd_critique(args: &[String]) -> ExitCode {
    let mut plan_path: Option<PathBuf> = None;
    let mut readback_path: Option<PathBuf> = None;
    let mut skill_names: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--skill" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--skill requires a skill name");
                    return ExitCode::FAILURE;
                }
                skill_names.push(args[i].clone());
            }
            other if !other.starts_with("--") && plan_path.is_none() => {
                plan_path = Some(PathBuf::from(other));
            }
            other if !other.starts_with("--") && readback_path.is_none() => {
                readback_path = Some(PathBuf::from(other));
            }
            other => {
                eprintln!("unexpected arg: {other}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }
    let (Some(plan_path), Some(readback_path)) = (plan_path, readback_path) else {
        eprintln!("usage: codedesign critique <plan.json> <readback.json>");
        return ExitCode::FAILURE;
    };

    let Some(provider) = resolve_provider() else {
        return ExitCode::FAILURE;
    };
    let ds = match load_ds() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: could not load DesignSystem: {e}");
            return ExitCode::FAILURE;
        }
    };

    let plan_text = match fs::read_to_string(&plan_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", plan_path.display());
            return ExitCode::FAILURE;
        }
    };
    let readback_text = match fs::read_to_string(&readback_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", readback_path.display());
            return ExitCode::FAILURE;
        }
    };

    match run_critique(&provider, &ds, &plan_text, &readback_text, &load_skills(&ds, &skill_names)) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Call the Critic role and return a validated report.
fn run_critique(
    p: &Provider,
    ds: &DesignSystem,
    plan_text: &str,
    readback_text: &str,
    skills: &[Skill],
) -> Result<cd_critique::CritiqueReport, String> {
    let base = p.base.trim_end_matches('/');
    let url = if base.ends_with("/v1") || base.ends_with("/v1/") {
        format!("{}/chat/completions", base.trim_end_matches('/'))
    } else {
        format!("{}/v1/chat/completions", base)
    };

    let role = cd_prompts::build(
        Role::Critic,
        ds,
        &PromptOptions { skills: skills.to_vec(), ..Default::default() },
    );
    let system = role.system;
    let user = format!(
        "# DesignPlan (IR)\n```json\n{}\n```\n\n# Figma readback\n```json\n{}\n```\n\nEmit one `critic_emit` tool call. No prose.",
        truncate(plan_text, 24_000),
        truncate(readback_text, 24_000),
    );

    let body = json!({
        "model": p.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user",   "content": user}
        ],
        "tools": [cd_critique::critic_emit_schema()],
        "tool_choice": "auto"
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(300))
        .build();
    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {}", p.key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("http call failed: {e}"))?;

    let json: Value = resp.into_json().map_err(|e| format!("decode: {e}"))?;

    let args_str = json
        .pointer("/choices/0/message/tool_calls/0/function/arguments")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "critic did not return a tool_call. raw:\n{}",
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )
        })?;

    let report_json: Value = serde_json::from_str(args_str)
        .map_err(|e| format!("critic returned non-JSON args: {e}"))?;
    cd_critique::validate_report(&report_json)
        .map_err(|e| format!("critic report failed validation: {e}\n{args_str}"))
}

// ---------- summarize (isolated-session summarizer) ----------

fn cmd_summarize(args: &[String]) -> ExitCode {
    let mut transcript_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" | "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--out requires a path");
                    return ExitCode::FAILURE;
                }
                out_path = Some(PathBuf::from(&args[i]));
            }
            other if !other.starts_with("--") && transcript_path.is_none() => {
                transcript_path = Some(PathBuf::from(other));
            }
            other => {
                eprintln!("unexpected arg: {other}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }
    let Some(transcript_path) = transcript_path else {
        eprintln!("usage: codedesign summarize <transcript.txt> [--out <summary.json>]");
        return ExitCode::FAILURE;
    };

    let Some(provider) = resolve_provider() else {
        return ExitCode::FAILURE;
    };
    let transcript = match fs::read_to_string(&transcript_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", transcript_path.display());
            return ExitCode::FAILURE;
        }
    };

    eprintln!(
        "[codedesign] summarizing {} ({} bytes) in isolated session...",
        transcript_path.display(),
        transcript.len()
    );
    let cfg = cd_summarizer::SummarizerConfig::new(&provider.key, &provider.base, &provider.model);
    let summary = match cd_summarizer::summarize_transcript(&cfg, &transcript) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let json = match serde_json::to_string_pretty(&summary) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: encode summary: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(out) = &out_path {
        if let Err(e) = fs::write(out, &json) {
            eprintln!("error: write {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        eprintln!("[codedesign] summary written to {}", out.display());
    }
    println!("{json}");
    eprintln!("\n{}", summary.to_injection_text());
    ExitCode::SUCCESS
}

// ---------- ds-query / ds-validate / prd-parse / open-node ----------

fn cmd_ds_query(args: &[String]) -> ExitCode {
    let mut query: Option<String> = None;
    let mut limit: usize = 8;
    let mut kind = String::from("any");
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kind" => {
                i += 1;
                if i >= args.len() { eprintln!("--kind requires a value"); return ExitCode::FAILURE; }
                kind = args[i].clone();
            }
            "--limit" => {
                i += 1;
                if i >= args.len() { eprintln!("--limit requires a value"); return ExitCode::FAILURE; }
                limit = args[i].parse().unwrap_or(8);
            }
            other if !other.starts_with("--") && query.is_none() => query = Some(other.into()),
            other => { eprintln!("unexpected arg: {other}"); return ExitCode::FAILURE; }
        }
        i += 1;
    }
    let Some(query) = query else {
        eprintln!("usage: codedesign ds-query <query> [--kind token|component|any] [--limit N]");
        return ExitCode::FAILURE;
    };
    let ds = match load_ds() { Ok(d) => d, Err(e) => { eprintln!("error: {e}"); return ExitCode::FAILURE; } };
    let r = cd_tools::handle_ds_query(&ds, &query, limit, &kind);
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
    ExitCode::SUCCESS
}

fn cmd_ds_validate(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("usage: codedesign ds-validate <fragment.json>");
        return ExitCode::FAILURE;
    }
    let path = PathBuf::from(&args[0]);
    let txt = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => { eprintln!("error: read {}: {e}", path.display()); return ExitCode::FAILURE; }
    };
    let v: Value = match serde_json::from_str(&txt) {
        Ok(v) => v,
        Err(e) => { eprintln!("error: invalid JSON: {e}"); return ExitCode::FAILURE; }
    };
    let ds = match load_ds() { Ok(d) => d, Err(e) => { eprintln!("error: {e}"); return ExitCode::FAILURE; } };
    let r = cd_tools::handle_ds_validate(&ds, &v);
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
    if r.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn cmd_prd_parse(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("usage: codedesign prd-parse <prd.txt>");
        return ExitCode::FAILURE;
    }
    let path = PathBuf::from(&args[0]);
    let txt = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => { eprintln!("error: read {}: {e}", path.display()); return ExitCode::FAILURE; }
    };
    let r = cd_tools::handle_prd_parse(&txt);
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
    ExitCode::SUCCESS
}

fn cmd_open_node(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("usage: codedesign open-node <fileKey> <nodeId>");
        return ExitCode::FAILURE;
    }
    println!("{}", cd_tools::format_figma_deeplink(&args[0], &args[1]));
    ExitCode::SUCCESS
}

// ---------- compile (debug aid) ----------

fn cmd_compile(args: &[String]) -> ExitCode {
    let mut plan_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" | "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--out requires a path");
                    return ExitCode::FAILURE;
                }
                out_path = Some(PathBuf::from(&args[i]));
            }
            other if !other.starts_with("--") && plan_path.is_none() => {
                plan_path = Some(PathBuf::from(other));
            }
            other => {
                eprintln!("unexpected arg: {other}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }
    let Some(plan_path) = plan_path else {
        eprintln!("usage: codedesign compile <plan.json> [--out <file.js>]");
        return ExitCode::FAILURE;
    };
    let out_path = out_path.unwrap_or_else(|| PathBuf::from("./codedesign-output.js"));

    let txt = match fs::read_to_string(&plan_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", plan_path.display());
            return ExitCode::FAILURE;
        }
    };
    let plan = match cd_plan::DesignPlan::from_json(&txt) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: invalid DesignPlan JSON: {e}");
            return ExitCode::FAILURE;
        }
    };
    let product_js = match cd_canvas::compile(&plan) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: compile failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Bundle with the shim so the output is still runnable by hand,
    // but do not claim this is a supported channel — `design` is.
    let bundle = bundle_plugin_js(&product_js);
    if let Err(e) = fs::write(&out_path, &bundle) {
        eprintln!("error: could not write {}: {e}", out_path.display());
        return ExitCode::FAILURE;
    }
    eprintln!("[codedesign] compiled -> {} (debug aid; production path is `design`)", out_path.display());
    ExitCode::SUCCESS
}

// ---------- doctor ----------

fn cmd_doctor(_args: &[String]) -> ExitCode {
    let mut failures = 0;

    // Node.js check.
    match SysCommand::new("node").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(major) = parse_node_major(&v) {
                if major >= 18 {
                    println!("  ok   node {v}");
                } else {
                    println!("  FAIL node {v} (need >= 18)");
                    failures += 1;
                }
            } else {
                println!("  warn node present but unparsable: {v}");
            }
        }
        _ => {
            println!("  FAIL node not found on PATH (need >= 18)");
            failures += 1;
        }
    }

    // PAT check (env first, then auth.toml).
    match auth::load_figma_token() {
        Ok(_) => {
            println!("  ok   figma PAT resolved");
        }
        Err(e) => {
            println!("  warn figma PAT missing: {e}");
            if prompt_yes_no("Save a Figma PAT now?") {
                match prompt_line("Paste your Figma PAT (figd_...): ") {
                    Ok(t) if !t.trim().is_empty() => match auth::save_figma_token(t.trim()) {
                        Ok(path) => println!("  ok   saved to {}", path.display()),
                        Err(err) => {
                            println!("  FAIL could not save PAT: {err}");
                            failures += 1;
                        }
                    },
                    _ => {
                        println!("  FAIL no token entered");
                        failures += 1;
                    }
                }
            } else {
                failures += 1;
            }
        }
    }

    // DeepSeek key is only needed for `design`, but call it out.
    if resolve_provider_key().is_none() {
        println!("  warn CODEDESIGN_DEEPSEEK_API_KEY not set — `codedesign design` will fail");
    } else {
        println!("  ok   CODEDESIGN_DEEPSEEK_API_KEY present");
    }

    println!();
    println!("next: open Figma Desktop and install the figma-console-mcp Desktop Bridge Plugin");
    println!("      (see https://github.com/southleft/figma-console-mcp). Then run:");
    println!("        codedesign design \"<PRD>\"");

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        println!();
        println!("{failures} check(s) failed.");
        ExitCode::FAILURE
    }
}

// ---------- provider ----------

struct Provider {
    key: String,
    base: String,
    model: String,
}

fn resolve_provider() -> Option<Provider> {
    let key = resolve_provider_key()?;
    let base = env::var("CODEDESIGN_DEEPSEEK_BASE_URL")
        .or_else(|_| env::var("DEEPSEEK_BASE_URL"))
        .unwrap_or_else(|_| DEFAULT_BASE.to_string());
    let model = env::var("CODEDESIGN_DEEPSEEK_MODEL")
        .or_else(|_| env::var("DEEPSEEK_MODEL"))
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    Some(Provider { key, base, model })
}

fn resolve_provider_key() -> Option<String> {
    if let Ok(k) = env::var("CODEDESIGN_DEEPSEEK_API_KEY") {
        if !k.is_empty() {
            return Some(k);
        }
    }
    if let Ok(k) = env::var("DEEPSEEK_API_KEY") {
        if !k.is_empty() {
            eprintln!("warning: DEEPSEEK_API_KEY is deprecated, rename to CODEDESIGN_DEEPSEEK_API_KEY");
            return Some(k);
        }
    }
    None
}

fn call_plan(p: &Provider, ds: &DesignSystem, prd: &str, skills: &[Skill]) -> Result<String, String> {
    let base = p.base.trim_end_matches('/');
    let url = if base.ends_with("/v1") || base.ends_with("/v1/") {
        format!("{}/chat/completions", base.trim_end_matches('/'))
    } else {
        format!("{}/v1/chat/completions", base)
    };

    // Build the Planner system prompt from the DS + craft rules.
    let role = cd_prompts::build(
        Role::Planner,
        ds,
        &PromptOptions { skills: skills.to_vec(), ..Default::default() },
    );
    let system = format!(
        "{}\n\n# Planner output contract\n{}",
        role.system,
        PLAN_CONTRACT_FOOTER.trim()
    );

    // If the DS ships curated case images, attach the first 3 as
    // multimodal `image_url` parts alongside the PRD. Requires a
    // vision-capable model at the gateway — when the provider rejects
    // multimodal, fall back to text-only on the retry.
    let cases = if env::var("CODEDESIGN_NO_CASES").ok().as_deref() == Some("1") {
        Vec::new()
    } else {
        ds.resolved_case_images().into_iter().take(3).collect::<Vec<_>>()
    };

    let user_content = if cases.is_empty() {
        Value::String(prd.to_owned())
    } else {
        let mut parts = vec![json!({
            "type": "text",
            "text": format!("{prd}\n\nReference cases follow — use as calibration only, do not copy."),
        })];
        for (entry, path) in &cases {
            match encode_image_data_url(path) {
                Ok(data_url) => {
                    parts.push(json!({
                        "type": "text",
                        "text": format!("Case: {} ({}) — {}", entry.title, entry.id, entry.note),
                    }));
                    parts.push(json!({"type": "image_url", "image_url": {"url": data_url}}));
                }
                Err(e) => eprintln!("[codedesign] case `{}` skipped: {}", entry.id, e),
            }
        }
        eprintln!("[codedesign] attached {} DS case image(s) to Plan prompt", cases.len());
        Value::Array(parts)
    };

    let body = json!({
        "model": p.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user",   "content": user_content}
        ],
        "tools": [plan_emit_tool(ds)],
        "tool_choice": "auto"
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(300))
        .build();

    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {}", p.key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("http call failed: {e}"))?;

    let json: Value = resp
        .into_json()
        .map_err(|e| format!("decode response failed: {e}"))?;

    let args = json
        .pointer("/choices/0/message/tool_calls/0/function/arguments")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "model did not return a tool_call. raw response:\n{}",
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )
        })?;

    Ok(args.to_string())
}

/// Resolve DS per priority: CODEDESIGN_DS_DIR env → ~/.codedesign/design-systems/default/ → bundled.
fn load_ds() -> Result<DesignSystem, String> {
    if let Ok(dir) = env::var("CODEDESIGN_DS_DIR") {
        if !dir.is_empty() {
            return DesignSystem::load(&dir).map_err(|e| format!("{dir}: {e}"));
        }
    }
    if let Some(home) = dirs_home() {
        let p = home.join(".codedesign").join("design-systems").join("default");
        if p.exists() {
            return DesignSystem::load(&p).map_err(|e| format!("{}: {e}", p.display()));
        }
    }
    DesignSystem::bundled_default().map_err(|e| format!("bundled default: {e}"))
}

fn dirs_home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

/// Resolve a skill name to a directory on disk. Search order:
///   1. `CODEDESIGN_SKILLS_DIR/<name>` (env override)
///   2. `~/.codedesign/skills/<name>` (user)
///   3. bundled `assets/skills/<name>` (workspace, dev builds only)
fn resolve_skill_dir(name: &str) -> Option<PathBuf> {
    if let Ok(base) = env::var("CODEDESIGN_SKILLS_DIR") {
        let p = PathBuf::from(&base).join(name);
        if p.join("SKILL.md").exists() { return Some(p); }
    }
    if let Some(home) = dirs_home() {
        let p = home.join(".codedesign").join("skills").join(name);
        if p.join("SKILL.md").exists() { return Some(p); }
    }
    // Dev-mode fallback: resolve relative to the workspace assets dir.
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().and_then(Path::parent)
        .map(|p| p.join("assets/skills").join(name));
    if let Some(p) = bundled {
        if p.join("SKILL.md").exists() { return Some(p); }
    }
    None
}

/// Load and validate the named skills against the DS. Skills whose
/// requirements are not met are reported to stderr and skipped.
fn load_skills(ds: &DesignSystem, names: &[String]) -> Vec<Skill> {
    let mut out = Vec::new();
    for name in names {
        let Some(dir) = resolve_skill_dir(name) else {
            eprintln!("[codedesign] skill `{name}` not found (checked CODEDESIGN_SKILLS_DIR, ~/.codedesign/skills, bundled)");
            continue;
        };
        match Skill::load(&dir) {
            Ok(s) => {
                if let Err(e) = s.check_requirements(ds) {
                    eprintln!("[codedesign] skill `{name}` skipped: {e}");
                    continue;
                }
                eprintln!("[codedesign] skill active: {} ({})", s.front.cd.name, s.front.cd.product_kind);
                out.push(s);
            }
            Err(e) => eprintln!("[codedesign] skill `{name}` failed to parse: {e}"),
        }
    }
    out
}

// ---------- helpers ----------

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\n... [truncated, {} bytes total]", &s[..max], s.len())
    }
}

fn parse_node_major(s: &str) -> Option<u32> {
    // Accepts "v20.11.0" or "20.11.0".
    let s = s.trim_start_matches('v');
    s.split('.').next()?.parse().ok()
}

fn encode_image_data_url(path: &Path) -> Result<String, String> {
    let mime = match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => return Err(format!("unknown image extension: {}", path.display())),
    };
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

fn prompt_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    Ok(s)
}

fn prompt_yes_no(prompt: &str) -> bool {
    match prompt_line(&format!("{prompt} [y/N] ")) {
        Ok(s) => {
            let t = s.trim().to_ascii_lowercase();
            t == "y" || t == "yes"
        }
        Err(_) => false,
    }
}

// ---------- prompts & schema ----------

const PLAN_CONTRACT_FOOTER: &str = r#"
Your sole output channel is the `plan_emit` tool. Call it exactly once per turn. Do NOT answer in prose.

SCHEMA CONTRACT (ENFORCED)
- Top-level keys: planId, dsRef, projectId, intent, stepKind, frames, rationale.
- Frames MUST have: localId, name, page, size {w,h}, layout {mode, padding, gap?}, children[].
- Children are one of:
    * {"type":"text", "styleRef":"typo.xxx", "fillRef":"color.xxx", "content":"..."}
    * {"type":"instance", "componentRef":"<DS component>", "props":{...}}
    * {"type":"frame", ...}  (nested)
- fillRef / styleRef / componentRef MUST be dot-path refs that RESOLVE in the DS binding block above.
- padding / gap MUST be dot-path refs like space.md.
- NEVER emit raw hex, px, or font names.

HARD RULES
- dsRef  = "default@1.0.0"
- projectId = "codedesign-session"
- intent = "create_screens" unless explicitly impossible.
- stepKind = "skeleton" for initial plans.
- 1-6 frames per plan. Typical sizes: 375x812 (mobile) or 1440x900 (desktop).
"#;

fn plan_emit_tool(ds: &DesignSystem) -> Value {
    let component_enum: Vec<Value> = ds
        .component_names()
        .into_iter()
        .map(Value::String)
        .collect();

    let node_schema = json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["type", "styleRef", "fillRef", "content"],
                "properties": {
                    "type":     {"const": "text"},
                    "styleRef": {"type": "string", "pattern": "^typo\\."},
                    "fillRef":  {"type": "string", "pattern": "^color\\."},
                    "content":  {"type": "string"}
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "required": ["type", "componentRef"],
                "properties": {
                    "type":         {"const": "instance"},
                    "componentRef": {"type": "string", "enum": component_enum},
                    "props":        {"type": "object"}
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "required": ["type", "localId", "name", "page", "size", "layout", "children"],
                "properties": {
                    "type":    {"const": "frame"},
                    "localId": {"type": "string"},
                    "name":    {"type": "string"},
                    "page":    {"type": "string"},
                    "size":    {"type": "object", "required": ["w","h"],
                                "properties": {"w": {"type":"number"}, "h":{"type":"number"}}},
                    "layout":  {
                        "type": "object",
                        "required": ["mode", "padding"],
                        "properties": {
                            "mode":    {"type": "string", "enum": ["VERTICAL", "HORIZONTAL", "NONE"]},
                            "padding": {"type": "string", "pattern": "^space\\."},
                            "gap":     {"type": "string", "pattern": "^space\\."}
                        }
                    },
                    "fillRef":  {"type": "string", "pattern": "^color\\."},
                    "children": {"type": "array"}
                },
                "additionalProperties": false
            }
        ]
    });

    let frame_schema = json!({
        "type": "object",
        "required": ["localId", "name", "page", "size", "layout", "children"],
        "properties": {
            "localId": {"type": "string"},
            "name":    {"type": "string"},
            "page":    {"type": "string"},
            "size": {
                "type": "object",
                "required": ["w", "h"],
                "properties": {"w": {"type": "number"}, "h": {"type": "number"}}
            },
            "layout": {
                "type": "object",
                "required": ["mode", "padding"],
                "properties": {
                    "mode":    {"type": "string", "enum": ["VERTICAL", "HORIZONTAL", "NONE"]},
                    "padding": {"type": "string", "pattern": "^space\\."},
                    "gap":     {"type": "string", "pattern": "^space\\."}
                }
            },
            "fillRef":  {"type": "string", "pattern": "^color\\."},
            "children": {"type": "array", "items": node_schema}
        },
        "additionalProperties": false
    });

    json!({
        "type": "function",
        "function": {
            "name": "plan_emit",
            "description": "Emit a DesignPlan IR. Refs only, no raw values.",
            "parameters": {
                "type": "object",
                "required": ["planId", "dsRef", "projectId", "intent", "stepKind", "frames"],
                "properties": {
                    "planId":    {"type": "string"},
                    "dsRef":     {"type": "string"},
                    "projectId": {"type": "string"},
                    "intent":    {"type": "string", "enum": ["create_screens","refine_selection","restyle","add_state","extract_ds"]},
                    "stepKind":  {"type": "string", "enum": ["pages","skeleton","content","states","annotations"]},
                    "frames":    {"type": "array", "minItems": 1, "items": frame_schema},
                    "components_to_create": {"type": "array"},
                    "tokens_to_create":     {"type": "array"},
                    "craft_asserts":        {"type": "array", "items": {"type": "string"}},
                    "quality_gates":        {"type": "array", "items": {"type": "string"}},
                    "dependsOn":            {"type": "array", "items": {"type": "string"}},
                    "rationale":            {"type": "string"}
                },
                "additionalProperties": false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_node_major_handles_v_prefix() {
        assert_eq!(parse_node_major("v20.11.0"), Some(20));
        assert_eq!(parse_node_major("18.0.0"), Some(18));
        assert_eq!(parse_node_major("not a version"), None);
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 100), "hello");
    }

    #[test]
    fn truncate_long_string_cut() {
        let big = "a".repeat(3000);
        let t = truncate(&big, 100);
        assert!(t.starts_with(&"a".repeat(100)));
        assert!(t.contains("truncated"));
    }
}
