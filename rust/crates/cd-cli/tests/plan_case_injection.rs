//! Phase 0 S0.3 — Plan case injection baseline harness.
//!
//! For each sample under `/evals/phase0/plan-case-injection/samples/`, build a
//! Plan-role prompt that embeds:
//!   - a compact DS summary (tokens + components + doNot rules)
//!   - the PRD blurb
//!   - three text-only case metadata entries (description / tags / doNot)
//!
//! Then call DeepSeek-chat with `tool_choice=plan_emit` and persist the model's
//! tool-call arguments as `output/<sample-id>.plan.json` for human scoring.
//! A lightweight auto-check also validates that the output deserializes into
//! our `cd_plan::DesignPlan` schema (schema validity is score dim 1).
//!
//! Run:
//!     DEEPSEEK_API_KEY=... cargo test -p cd-cli --test plan_case_injection \
//!         -- --ignored --nocapture
//!
//! Human scoring form: `/evals/phase0/plan-case-injection/result.md`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

const DEFAULT_BASE: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-chat";

#[derive(Debug, Deserialize)]
struct Sample {
    id: String,
    direction: String,
    #[serde(rename = "dsSummary")]
    ds_summary: Value,
    prd: String,
    cases: Vec<Value>,
}

fn samples_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../evals/phase0/plan-case-injection/samples");
    p
}

fn output_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../evals/phase0/plan-case-injection/output");
    fs::create_dir_all(&p).ok();
    p
}

fn load_sample(id: &str) -> Sample {
    let mut p = samples_dir();
    p.push(format!("{id}.json"));
    let s = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"));
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("parse {p:?}: {e}"))
}

fn plan_emit_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "plan_emit",
            "description": "Emit a DesignPlan IR for this brief.",
            "parameters": {
                "type": "object",
                "properties": {
                    "planId":   {"type": "string"},
                    "dsRef":    {"type": "string"},
                    "projectId":{"type": "string"},
                    "intent":   {"type": "string", "enum": [
                        "create_screens","refine_selection","restyle","add_state","extract_ds"
                    ]},
                    "stepKind": {"type": "string", "enum": [
                        "pages","skeleton","content","states","annotations"
                    ]},
                    "frames":   {"type": "array"},
                    "components_to_create": {"type": "array"},
                    "tokens_to_create":     {"type": "array"},
                    "craft_asserts":        {"type": "array"},
                    "quality_gates":        {"type": "array"},
                    "dependsOn":            {"type": "array"},
                    "rationale":            {"type": "string"}
                },
                "required": ["planId","dsRef","projectId","intent","stepKind","frames"]
            }
        }
    })
}

fn build_system_prompt(sample: &Sample) -> String {
    let ds = serde_json::to_string_pretty(&sample.ds_summary).unwrap();
    let cases = serde_json::to_string_pretty(&sample.cases).unwrap();
    format!(
        "You are the Plan role of CodeDesign. You MUST call the `plan_emit` \
         tool exactly once. Do NOT answer in prose. Do NOT call any other tool. \
         The entirety of your response is the tool_call.\n\
         \n\
         HARD RULES\n\
         - dsRef = \"default@1.0.0\"; projectId = \"phase0-s03-{id}\"; \
           planId = \"p_{id}_001\".\n\
         - intent = \"create_screens\", stepKind = \"skeleton\" unless strictly \
           impossible.\n\
         - ALL value-bearing fields MUST be refs (e.g. \"color.bg.base\", \
           \"typo.title.lg\", \"button.primary\"). NEVER emit raw hex / px / \
           font-name.\n\
         - Only reference tokens/components listed in the DS summary. If you \
           need something new, declare it under `tokens_to_create` / \
           `components_to_create`.\n\
         - Respect every doNot entry in the DS summary AND in every case.\n\
         - Visual direction is `{direction}` — let the reference cases steer \
           density, typography, and accent restraint; do not copy them.\n\
         \n\
         DESIGN SYSTEM SUMMARY\n\
         {ds}\n\
         \n\
         REFERENCE CASES (text-only metadata; treat as style/layout guidance)\n\
         {cases}\n",
        id = sample.id,
        direction = sample.direction,
        ds = ds,
        cases = cases,
    )
}

fn call_deepseek(sample: &Sample) -> Option<Value> {
    let key = match env::var("DEEPSEEK_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("[{}] SKIP: DEEPSEEK_API_KEY not set", sample.id);
            return None;
        }
    };
    let base = env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE.to_string());
    let model = env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));

    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": build_system_prompt(sample)},
            {"role": "user",   "content": sample.prd}
        ],
        "tools": [plan_emit_tool()],
        // Some OpenAI-compat proxies (oneapi with thinking-mode models)
        // reject forced tool_choice. Use "auto" and rely on the system prompt.
        "tool_choice": "auto"
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(300))
        .build();
    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .unwrap_or_else(|e| panic!("[{}] http: {e}", sample.id));
    Some(
        resp.into_json::<Value>()
            .unwrap_or_else(|e| panic!("[{}] decode: {e}", sample.id)),
    )
}

fn run_sample(id: &str) {
    let sample = load_sample(id);
    let Some(resp) = call_deepseek(&sample) else { return };
    let args = resp
        .pointer("/choices/0/message/tool_calls/0/function/arguments")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("[{id}] no tool_call args; resp={resp}"));

    // Save raw args for human scoring.
    let mut out = output_dir();
    out.push(format!("{id}.plan.json"));
    fs::write(&out, args).expect("write plan");

    // Auto-check dim-1: schema validity.
    // NOTE: this is a BASELINE spike, not a Gate-1 validator test. Locking
    // the frame shape down is DESIGN.md §2.5 / Gate-1's job. We record
    // schema conformance as a data point but do not fail the test — the
    // point of the spike is the human scoring that follows.
    let schema_ok = cd_plan::DesignPlan::from_json(args).is_ok();
    eprintln!(
        "[{id}] saved → {} | schema_valid={schema_ok}",
        out.display()
    );
    if !schema_ok {
        eprintln!(
            "[{id}] NOTE: plan deviated from DesignPlan schema — tighten \
             `plan_emit_tool()` JSON schema or DESIGN.md §2.5 description \
             before Phase 1. Output persisted for human scoring."
        );
    }
}

#[test]
#[ignore]
fn sample_a_saas_login_neutral_modern() {
    run_sample("sample-a");
}

#[test]
#[ignore]
fn sample_b_reading_app_warm_editorial() {
    run_sample("sample-b");
}

#[test]
#[ignore]
fn sample_c_dashboard_tech_utility() {
    run_sample("sample-c");
}
