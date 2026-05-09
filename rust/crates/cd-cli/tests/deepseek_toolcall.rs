//! Phase 0 S0.1 — DeepSeek tool-call parity harness.
//!
//! See `/evals/phase0/deepseek-toolcall/README.md` for case definitions.
//! Every test is `#[ignore]`; they hit a real DeepSeek endpoint, so they
//! only run when `DEEPSEEK_API_KEY` is set:
//!
//!     cargo test -p cd-cli --test deepseek_toolcall -- --ignored --nocapture
//!
//! Optional env:
//!   DEEPSEEK_BASE_URL   default https://api.deepseek.com
//!   DEEPSEEK_MODEL      default deepseek-chat
//!   DEEPSEEK_MODEL_JSON default deepseek-chat (tc-04 json_schema)
//!
//! Results are appended to /evals/phase0/deepseek-toolcall/result.md when the
//! `CDEVAL_WRITE_REPORT=1` env var is set (off by default so CI runs are pure).

use std::env;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};

const DEFAULT_BASE: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-chat";

struct DsClient {
    base: String,
    key: String,
    model: String,
    json_model: String,
}

impl DsClient {
    fn from_env_or_skip(case: &str) -> Option<Self> {
        let key = match env::var("DEEPSEEK_API_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                eprintln!("[{case}] SKIP: DEEPSEEK_API_KEY not set");
                return None;
            }
        };
        Some(Self {
            base: env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE.to_string()),
            key,
            model: env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
            json_model: env::var("DEEPSEEK_MODEL_JSON")
                .unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
        })
    }

    fn chat(&self, body: Value) -> Result<Value, String> {
        let url = format!("{}/v1/chat/completions", self.base.trim_end_matches('/'));
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(60))
            .build();
        // Retry transient 5xx / timeouts up to 3 times (some OpenAI-compat
        // proxies sporadically return 502/503 under load).
        let mut last_err = String::new();
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(800 * attempt as u64));
            }
            let req = agent
                .post(&url)
                .set("Authorization", &format!("Bearer {}", self.key))
                .set("Content-Type", "application/json");
            match req.send_json(body.clone()) {
                Ok(resp) => return resp.into_json::<Value>().map_err(|e| format!("decode: {e}")),
                Err(ureq::Error::Status(code, resp)) if code >= 500 => {
                    last_err = format!("status {code}: {}", resp.into_string().unwrap_or_default());
                    continue;
                }
                Err(e) => return Err(format!("http error: {e}")),
            }
        }
        Err(format!("http error after retries: {last_err}"))
    }
}

fn file_read_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "file_read",
            "description": "Read a text file from disk",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Absolute path"}
                },
                "required": ["path"]
            }
        }
    })
}

fn file_list_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "file_list",
            "description": "List files in a directory",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }
    })
}

fn plan_emit_tool() -> Value {
    // Minimal Plan IR accepting tool for tc-03 / tc-09.
    json!({
        "type": "function",
        "function": {
            "name": "plan_emit",
            "description": "Emit a DesignPlan IR for review",
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
                    "frames":   {"type": "array"}
                },
                "required": ["planId","dsRef","projectId","intent","stepKind","frames"]
            }
        }
    })
}

fn tool_calls_of<'a>(resp: &'a Value) -> &'a [Value] {
    resp.pointer("/choices/0/message/tool_calls")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

fn record(case: &str, ok: bool, note: &str) {
    eprintln!(
        "[{case}] {}: {note}",
        if ok { "PASS" } else { "FAIL" }
    );
    if env::var("CDEVAL_WRITE_REPORT").ok().as_deref() != Some("1") {
        return;
    }
    // evals/phase0/deepseek-toolcall/result.md — path relative to this test file.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../evals/phase0/deepseek-toolcall/result.md");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "- **{case}** {}: {}", if ok { "✅" } else { "❌" }, note);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// tc-01 single tool call
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_01_single_tool_call() {
    let Some(c) = DsClient::from_env_or_skip("tc-01") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content": "You must use file_read to answer."},
                {"role": "user", "content": "What is in /etc/hostname?"}
            ],
            "tools": [file_read_tool()],
            "tool_choice": "auto"
        }))
        .expect("http");
    let calls = tool_calls_of(&resp);
    let ok = calls.len() == 1
        && calls[0]
            .pointer("/function/name")
            .and_then(|v| v.as_str())
            == Some("file_read")
        && calls[0]
            .pointer("/function/arguments")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|j| j.get("path").cloned())
            .is_some();
    record("tc-01", ok, &format!("calls={}", calls.len()));
    assert!(ok, "tc-01 failed; resp={resp}");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-02 parallel tool calls
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_02_parallel_tool_calls() {
    let Some(c) = DsClient::from_env_or_skip("tc-02") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content":
                    "Call file_read for BOTH paths in parallel in a single response."},
                {"role": "user", "content":
                    "Read /etc/hostname AND /etc/os-release. Single turn, two parallel tool_calls."}
            ],
            "tools": [file_read_tool()],
            "tool_choice": "auto"
        }))
        .expect("http");
    let calls = tool_calls_of(&resp);
    let ok = calls.len() >= 2
        && calls
            .iter()
            .all(|c| c.pointer("/function/name").and_then(|v| v.as_str()) == Some("file_read"));
    record("tc-02", ok, &format!("calls={}", calls.len()));
    assert!(ok, "tc-02 expected >=2 parallel calls");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-03 nested-json args compatible with DesignPlan
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_03_nested_json_plan_ir() {
    let Some(c) = DsClient::from_env_or_skip("tc-03") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content":
                    "Emit a minimal DesignPlan via plan_emit. Use ds default@1.0.0,\
                     intent create_screens, stepKind skeleton, projectId eval-tc03,\
                     planId p_tc03. frames may be []. ALL value fields use refs not raw values."},
                {"role": "user", "content": "Emit a stub plan."}
            ],
            "tools": [plan_emit_tool()],
            "tool_choice": {"type": "function", "function": {"name": "plan_emit"}}
        }))
        .expect("http");
    let args = tool_calls_of(&resp)
        .first()
        .and_then(|c| c.pointer("/function/arguments"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Must parse into our DesignPlan schema (cd-plan serde).
    let ok = cd_plan::DesignPlan::from_json(args).is_ok();
    record("tc-03", ok, &format!("args.len={}", args.len()));
    assert!(ok, "tc-03: args did not deserialize as DesignPlan: {args}");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-04 json_schema response_format (turn-1 form)
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_04_json_schema_turn1_form() {
    let Some(c) = DsClient::from_env_or_skip("tc-04") else { return };
    let resp = c
        .chat(json!({
            "model": c.json_model,
            "messages": [
                {"role": "system", "content":
                    "You are the CodeDesign intake agent. Emit ONLY JSON of \
                     schema turn1Form. No markdown fences, no prose."},
                {"role": "user", "content":
                    "I want to design an onboarding flow for a mobile todo app."}
            ],
            "response_format": {"type": "json_object"}
        }))
        .expect("http");
    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let parsed: Option<Value> = serde_json::from_str(content).ok();
    let ok = parsed.is_some() && !content.trim_start().starts_with("```");
    record("tc-04", ok, &format!("len={}", content.len()));
    assert!(ok, "tc-04: not clean JSON: {content}");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-05 recover from tool_result error
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_05_tool_error_recovery() {
    let Some(c) = DsClient::from_env_or_skip("tc-05") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content":
                    "If a tool fails, try an alternative or ask the user — never repeat \
                     the same failing call."},
                {"role": "user", "content": "Find project files."},
                {"role": "assistant",
                 "content": null,
                 "reasoning_content": "User asked to find project files; I try reading /nope first.",
                 "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "file_read", "arguments": "{\"path\":\"/nope\"}"}
                }]},
                {"role": "tool", "tool_call_id": "call_1",
                 "content": "{\"error\": true, \"message\": \"ENOENT\"}"}
            ],
            "tools": [file_read_tool(), file_list_tool()],
            "tool_choice": "auto"
        }))
        .expect("http");
    let calls = tool_calls_of(&resp);
    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Either switches to file_list OR responds with text asking the user.
    let switched = calls
        .iter()
        .any(|c| c.pointer("/function/name").and_then(|v| v.as_str()) == Some("file_list"));
    let asks = !content.is_empty() && calls.is_empty();
    let repeats_fail = calls
        .iter()
        .any(|c| c.pointer("/function/arguments").and_then(|v| v.as_str()) == Some("{\"path\":\"/nope\"}"));
    let ok = (switched || asks) && !repeats_fail;
    record("tc-05", ok, &format!("switched={switched} asks={asks} repeat={repeats_fail}"));
    assert!(ok, "tc-05 recovery failed; resp={resp}");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-06 multi-turn tool loop (5+ turns)
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_06_multi_turn_loop() {
    let Some(c) = DsClient::from_env_or_skip("tc-06") else { return };

    // Simulate a budget-counting loop. We feed synthetic tool results and stop
    // when the model produces a final text answer OR hits MAX_TURNS.
    const MAX_TURNS: usize = 8;
    let tools = json!([file_read_tool(), file_list_tool()]);
    let mut messages = json!([
        {"role": "system", "content":
            "You are exploring a repo. Repeatedly call file_list/file_read until \
             you can answer what language the project is in. When ready, stop \
             calling tools and reply in plain text."},
        {"role": "user", "content": "What language is this project written in?"}
    ]);
    let mut turns = 0usize;
    let mut final_text: Option<String> = None;
    while turns < MAX_TURNS {
        let resp = c
            .chat(json!({
                "model": c.model,
                "messages": messages,
                "tools": tools,
                "tool_choice": "auto"
            }))
            .expect("http");
        let msg = resp.pointer("/choices/0/message").cloned().unwrap_or(json!({}));
        let calls = msg.get("tool_calls").and_then(|v| v.as_array()).cloned();
        if let Some(calls) = calls.filter(|c| !c.is_empty()) {
            messages.as_array_mut().unwrap().push(msg.clone());
            for call in calls {
                let id = call.get("id").and_then(|v| v.as_str()).unwrap_or("x").to_string();
                let name = call.pointer("/function/name")
                    .and_then(|v| v.as_str()).unwrap_or("?");
                let fake = if name == "file_list" {
                    json!({"entries": ["Cargo.toml","src","README.md"]}).to_string()
                } else {
                    json!({"content":"[package]\nname=\"demo\"\n"}).to_string()
                };
                messages.as_array_mut().unwrap().push(json!({
                    "role": "tool", "tool_call_id": id, "content": fake
                }));
            }
            turns += 1;
            continue;
        }
        final_text = msg
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        break;
    }
    let ok = final_text.as_deref().map(|s| !s.is_empty()).unwrap_or(false) && turns < MAX_TURNS;
    record("tc-06", ok, &format!("turns={turns} ended={}", final_text.is_some()));
    assert!(ok, "tc-06 loop failed");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-07 large tool result handling (pre-truncated on our side)
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_07_large_tool_result() {
    let Some(c) = DsClient::from_env_or_skip("tc-07") else { return };
    // We model our `{ ref, summary, top-k }` truncation contract: the model
    // should work from the summary, not demand the raw content.
    let big_summary = json!({
        "ref": "tmp://scan-1",
        "summary": "43 TypeScript components across src/ui; 12 of them use \
                    the deprecated <OldButton>. Top 3: Settings, Profile, Billing.",
        "top_k": [
            {"path":"src/ui/Settings.tsx","hits":9},
            {"path":"src/ui/Profile.tsx","hits":7},
            {"path":"src/ui/Billing.tsx","hits":5}
        ]
    })
    .to_string();
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content":
                    "Tool results may arrive pre-truncated as {ref, summary, top_k}. \
                     Reason from the summary; do NOT ask for the raw file."},
                {"role": "user", "content": "Which files most use <OldButton>?"},
                {"role": "assistant",
                 "content": null,
                 "reasoning_content": "I'll list src/ui to find OldButton occurrences.",
                 "tool_calls": [{
                    "id": "call_1", "type":"function",
                    "function": {"name":"file_list","arguments":"{\"path\":\"src/ui\"}"}
                }]},
                {"role": "tool", "tool_call_id":"call_1", "content": big_summary}
            ],
            "tools": [file_read_tool(), file_list_tool()],
            "tool_choice": "auto"
        }))
        .expect("http");
    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ok = content.contains("Settings") && content.contains("Profile");
    record("tc-07", ok, &format!("len={}", content.len()));
    assert!(ok, "tc-07: did not reason from summary: {content}");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-08 empty tools list → must not hallucinate
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_08_no_tools_no_hallucination() {
    let Some(c) = DsClient::from_env_or_skip("tc-08") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content":
                    "No tools are available. Answer in plain text."},
                {"role": "user", "content":
                    "Read /etc/hostname and tell me what's in it."}
            ]
            // note: no "tools" field at all
        }))
        .expect("http");
    let calls = tool_calls_of(&resp);
    let content = resp
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ok = calls.is_empty() && !content.is_empty();
    record("tc-08", ok, &format!("calls={} textlen={}", calls.len(), content.len()));
    assert!(ok, "tc-08: hallucinated tool or empty text");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-09 tool_choice forces function
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_09_forced_tool_choice() {
    let Some(c) = DsClient::from_env_or_skip("tc-09") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content": "Answer helpfully."},
                {"role": "user", "content": "Just say hi."}
            ],
            "tools": [file_read_tool(), file_list_tool()],
            "tool_choice": {"type":"function","function":{"name":"file_list"}}
        }))
        .expect("http");
    let calls = tool_calls_of(&resp);
    let ok = calls.len() == 1
        && calls[0].pointer("/function/name").and_then(|v| v.as_str()) == Some("file_list");
    record("tc-09", ok, &format!("calls={}", calls.len()));
    assert!(ok, "tc-09: forced tool_choice ignored");
}

// ────────────────────────────────────────────────────────────────────────────
// tc-10 UTF-8 Chinese args round-trip
// ────────────────────────────────────────────────────────────────────────────
#[test]
#[ignore]
fn tc_10_utf8_chinese_args() {
    let Some(c) = DsClient::from_env_or_skip("tc-10") else { return };
    let resp = c
        .chat(json!({
            "model": c.model,
            "messages": [
                {"role": "system", "content":
                    "Call file_read with the EXACT path the user provides, preserving Chinese."},
                {"role": "user", "content": "读取 /项目/需求文档.md 的内容"}
            ],
            "tools": [file_read_tool()],
            "tool_choice": "auto"
        }))
        .expect("http");
    let args_raw = tool_calls_of(&resp)
        .first()
        .and_then(|c| c.pointer("/function/arguments"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let parsed: Value = serde_json::from_str(&args_raw).unwrap_or(Value::Null);
    let path = parsed.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let ok = path.contains("项目") && path.contains("需求文档");
    record("tc-10", ok, &format!("path={path}"));
    assert!(ok, "tc-10: Chinese path was mangled: {path}");
}
