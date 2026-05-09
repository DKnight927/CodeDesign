//! `prd_parse` — regex-based PRD → Brief skeleton extractor.
//!
//! Pure. The LLM Interpreter is still the source of truth; this tool
//! exists so downstream roles can seed their structured view quickly
//! and so smoke tests don't require an LLM.

use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, Serialize)]
pub struct PrdParseResult {
    pub platform: Option<String>,
    pub screens: Vec<String>,
    pub users: Vec<String>,
    pub success_signals: Vec<String>,
    pub raw_length: usize,
}

#[must_use]
pub fn prd_parse_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "prd_parse",
            "description": "Heuristic PRD → Brief skeleton. Returns best-guess platform/screens/users/success_signals. Use as a seed; confirm with the user before handing to the Planner.",
            "parameters": {
                "type": "object",
                "required": ["prd"],
                "properties": {
                    "prd": {"type": "string"}
                },
                "additionalProperties": false
            }
        }
    })
}

#[must_use]
pub fn handle_prd_parse(prd: &str) -> PrdParseResult {
    PrdParseResult {
        platform: detect_platform(prd),
        screens: detect_screens(prd),
        users: detect_users(prd),
        success_signals: detect_success(prd),
        raw_length: prd.len(),
    }
}

fn detect_platform(s: &str) -> Option<String> {
    let low = s.to_ascii_lowercase();
    // order matters: check most specific first
    for (kw, platform) in [
        ("ios", "mobile-ios"),
        ("android", "mobile-android"),
        ("mobile", "mobile"),
        ("desktop", "desktop"),
        ("web", "web"),
        ("小程序", "miniapp"),
        ("手机", "mobile"),
        ("移动端", "mobile"),
        ("桌面", "desktop"),
        ("网页", "web"),
    ] {
        if low.contains(kw) {
            return Some(platform.to_owned());
        }
    }
    None
}

fn detect_screens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let keywords = [
        ("登录", "login"),
        ("注册", "signup"),
        ("首页", "home"),
        ("设置", "settings"),
        ("个人中心", "profile"),
        ("详情", "detail"),
        ("列表", "list"),
        ("搜索", "search"),
        ("结算", "checkout"),
        ("支付", "payment"),
        ("login", "login"),
        ("signup", "signup"),
        ("home", "home"),
        ("dashboard", "dashboard"),
        ("settings", "settings"),
        ("profile", "profile"),
        ("checkout", "checkout"),
    ];
    let low = s.to_ascii_lowercase();
    for (kw, name) in keywords {
        if low.contains(kw) && !out.contains(&name.to_owned()) {
            out.push(name.to_owned());
        }
    }
    out
}

fn detect_users(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    // "用户: X / Y" or "users: a, b"
    let re = Regex::new(r"(?i)(users?|用户)\s*[:：]\s*([^\n]{1,200})").unwrap();
    for cap in re.captures_iter(s) {
        for u in cap[2].split(|c: char| c == ',' || c == '、' || c == '/' || c == ';') {
            let u = u.trim();
            if !u.is_empty() {
                out.push(u.to_owned());
            }
        }
    }
    out
}

fn detect_success(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let re = Regex::new(
        r"(?i)(success|goal|目标|成功指标|kpi)\s*[:：]\s*([^\n]{1,300})",
    )
    .unwrap();
    for cap in re.captures_iter(s) {
        for g in cap[2].split(|c: char| c == ',' || c == '、' || c == ';') {
            let g = g.trim();
            if !g.is_empty() {
                out.push(g.to_owned());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_platform_chinese() {
        let r = handle_prd_parse("这是一个移动端应用");
        assert_eq!(r.platform.as_deref(), Some("mobile"));
    }

    #[test]
    fn detects_login_screen() {
        let r = handle_prd_parse("需要一个登录页和首页");
        assert!(r.screens.contains(&"login".into()));
        assert!(r.screens.contains(&"home".into()));
    }

    #[test]
    fn parses_users_list() {
        let r = handle_prd_parse("用户: 张三, 李四\n其他内容");
        assert_eq!(r.users, vec!["张三".to_string(), "李四".to_string()]);
    }

    #[test]
    fn parses_success_signals() {
        let r = handle_prd_parse("goal: DAU > 1000, retention > 40%");
        assert_eq!(r.success_signals.len(), 2);
    }

    #[test]
    fn raw_length_tracked() {
        let r = handle_prd_parse("abc");
        assert_eq!(r.raw_length, 3);
    }
}
