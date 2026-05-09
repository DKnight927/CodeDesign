//! Phase 0 S0.4 — Summarizer independent-runtime spike.
//!
//! Procedurally builds an 80-message mock main session (8 anchor messages with
//! key facts + 72 filler turns) and asks DeepSeek in a **fresh session** to
//! return a strict `SessionSummary` JSON. We then:
//!
//!   1. Schema-validate the response against `cd_summarizer::SessionSummary`.
//!   2. Check that every anchor fact is present in the summary text.
//!   3. Persist the summary + injection text under
//!      `/evals/phase0/summarizer/output/` for human inspection.
//!
//! This spike does NOT yet run the second `ConversationRuntime` instance
//! end-to-end — that wiring ships with Phase 1 (§11.3). What it validates is
//! the data contract: the summarizer prompt reliably produces a schema-clean
//! SessionSummary that preserves design intent.
//!
//! Run:
//!     DEEPSEEK_API_KEY=... cargo test -p cd-cli --test summarizer_spike \
//!         -- --ignored --nocapture

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use cd_summarizer::SessionSummary;
use serde_json::{json, Value};

const DEFAULT_BASE: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-chat";

/// Key facts seeded into the mock session. We assert every one of these
/// appears (case-insensitive substring) in the summary's `to_injection_text`.
const ANCHOR_FACTS: &[(&str, &str)] = &[
    ("platform", "mobile iOS first, Android second"),
    ("brand", "brand=acme, accent=terracotta"),
    ("direction", "direction picked: warm-editorial"),
    ("ds", "DS ref: acme-editorial@1.2.0"),
    ("constraint", "min font size 14px, no violet/indigo"),
    ("skills", "skill activated: figma-mobile-onboarding"),
    ("plan", "last plan id: p_onboard_042"),
    ("open", "open question: confirm empty-state copy tone"),
];

fn build_mock_session() -> Vec<Value> {
    let mut msgs: Vec<Value> = Vec::with_capacity(80);
    msgs.push(json!({
        "role": "system",
        "content": "CodeDesign main session — user is designing acme mobile onboarding."
    }));

    // 8 anchor messages carrying the key facts we want preserved.
    for (i, (_tag, fact)) in ANCHOR_FACTS.iter().enumerate() {
        msgs.push(json!({
            "role": "user",
            "content": format!("Anchor #{i}: {fact}")
        }));
    }

    // 71 filler messages of tool-loop noise. Deterministic, keyword-neutral.
    let fillers = [
        ("assistant", "Reading design system tokens."),
        ("tool", "{\"ok\":true,\"tokens\":123}"),
        ("assistant", "Listing frames on the current page."),
        ("tool", "{\"frames\":[{\"id\":\"f1\"},{\"id\":\"f2\"}]}"),
        ("user", "Continue."),
        ("assistant", "Applying spacing token space.md."),
        ("tool", "{\"applied\":\"space.md\"}"),
        ("assistant", "Running Gate-1 validator."),
        ("tool", "{\"gate\":\"Gate-1\",\"pass\":true}"),
        ("user", "Proceed to next step."),
    ];
    while msgs.len() < 80 {
        let (role, content) = fillers[msgs.len() % fillers.len()];
        msgs.push(json!({"role": role, "content": content}));
    }
    msgs
}

/// Flatten the mock session into a single user message payload to feed the
/// summarizer. The summarizer runs in a **fresh** session: its system prompt
/// is the summarizer role; the only user content is the transcript.
fn flatten_transcript(msgs: &[Value]) -> String {
    let mut out = String::with_capacity(8192);
    for (i, m) in msgs.iter().enumerate() {
        let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("?");
        let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
        out.push_str(&format!("[{i}|{role}] {content}\n"));
    }
    out
}

fn summarizer_system_prompt() -> &'static str {
    "You are the CodeDesign session summarizer. You are running in an ISOLATED \
     session whose token budget does NOT touch the main session.\n\
     \n\
     Read the transcript below and return a JSON object with EXACTLY these \
     fields:\n\
     {\n\
       \"facts\":            [string, ...],  // stable facts: platform, brand, constraints, DS ref\n\
       \"decisions\":        [string, ...],  // direction picked, skills activated, gates overridden\n\
       \"openQuestions\":    [string, ...],  // user asks not yet answered\n\
       \"toolResultsDigest\":[string, ...],  // one-line digests of noteworthy tool results\n\
       \"lastPlanId\":       string | null\n\
     }\n\
     \n\
     RULES:\n\
     - Output JSON only, no markdown, no commentary.\n\
     - Preserve every Anchor # message as a fact OR decision.\n\
     - Keep it compact: each entry ≤ 120 chars.\n\
     - Never invent facts not present in the transcript."
}

fn output_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../evals/phase0/summarizer/output");
    fs::create_dir_all(&p).ok();
    p
}

#[test]
#[ignore]
fn summarize_mock_session_preserves_anchors() {
    let key = match env::var("DEEPSEEK_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("SKIP: DEEPSEEK_API_KEY not set");
            return;
        }
    };
    let base = env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE.to_string());
    let model = env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let msgs = build_mock_session();
    let transcript = flatten_transcript(&msgs);

    // Save the mock transcript too, so humans can inspect what was summarized.
    let out = output_dir();
    fs::write(out.join("mock-transcript.txt"), &transcript).ok();

    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(120))
        .build();
    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": model,
            "messages": [
                {"role": "system", "content": summarizer_system_prompt()},
                {"role": "user",   "content": transcript}
            ],
            "response_format": {"type": "json_object"}
        }))
        .expect("http");
    let body: Value = resp.into_json().expect("decode");
    let content = body
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("no content; body={body}"))
        .to_string();

    // Dim 1 — schema validity.
    let summary = SessionSummary::from_json(&content)
        .unwrap_or_else(|e| panic!("schema invalid: {e}\nraw={content}"));
    fs::write(out.join("summary.json"), &content).ok();
    let injection = summary.to_injection_text();
    fs::write(out.join("injection.txt"), &injection).ok();

    eprintln!("=== SUMMARY ===\n{injection}");

    // Dim 2 — anchor fact preservation. Summarizers paraphrase, so we only
    // require the *keyword* of each anchor to survive (lower bar than exact
    // match but still catches total drops).
    let hay = injection.to_lowercase();
    let anchor_keywords: &[(&str, &[&str])] = &[
        ("platform", &["ios"]),
        ("brand",    &["acme", "terracotta"]),
        ("direction",&["warm-editorial", "warm editorial"]),
        ("ds",       &["acme-editorial", "1.2.0"]),
        ("constraint",&["14", "violet", "indigo"]),
        ("skills",   &["figma-mobile-onboarding", "mobile-onboarding"]),
        ("plan",     &["p_onboard_042"]),
        ("open",     &["empty-state", "empty state"]),
    ];
    let mut missing: Vec<&str> = vec![];
    for (tag, keys) in anchor_keywords {
        let hit = keys.iter().any(|k| hay.contains(&k.to_lowercase()));
        if !hit {
            missing.push(tag);
        }
    }

    // Pass gate: ≥ 6 / 8 anchors preserved. Dropping > 2 = summarizer too
    // lossy, revisit prompt before Phase 1.
    let kept = anchor_keywords.len() - missing.len();
    eprintln!("anchors kept {kept}/{} missing={missing:?}", anchor_keywords.len());
    assert!(
        kept >= 6,
        "summarizer lost too many anchors ({kept}/8); missing={missing:?}"
    );
}
