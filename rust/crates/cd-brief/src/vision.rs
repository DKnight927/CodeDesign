//! `image_understand` — DeepSeek-VL (or any OpenAI-compatible vision)
//! client used as a tool for Planner / Critic / Extractor roles.
//!
//! The only supported shape is the OpenAI-compatible `chat/completions`
//! endpoint with a multimodal user message. Images may be passed as
//! either a local file path (base64-encoded here) or an http(s) URL.

use std::path::Path;
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VlError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("http: {0}")]
    Http(String),
    #[error("decode: {0}")]
    Decode(String),
    #[error("no content returned by the vision model")]
    Empty,
    #[error("image `{0}` has no recognised extension (use .png/.jpg/.jpeg/.webp/.gif)")]
    UnknownMime(String),
}

#[derive(Debug, Clone)]
pub struct VlConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
}

impl VlConfig {
    /// Build a config from CODEDESIGN_DEEPSEEK_* env (reuses the same
    /// provider by default; override via CODEDESIGN_VL_*).
    pub fn from_env() -> Option<Self> {
        let api_key = first_env(&["CODEDESIGN_VL_API_KEY", "CODEDESIGN_DEEPSEEK_API_KEY"])?;
        let base_url = first_env(&["CODEDESIGN_VL_BASE_URL", "CODEDESIGN_DEEPSEEK_BASE_URL"])
            .unwrap_or_else(|| "https://api.deepseek.com".into());
        let model = first_env(&["CODEDESIGN_VL_MODEL"])
            .unwrap_or_else(|| "deepseek-vl2".into());
        Some(Self {
            api_key,
            base_url,
            model,
            timeout: Duration::from_secs(120),
        })
    }
}

fn first_env(keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

pub struct VlClient {
    cfg: VlConfig,
}

/// How to reference an image in a request.
pub enum ImageRef<'a> {
    LocalPath(&'a Path),
    Url(&'a str),
}

impl VlClient {
    #[must_use]
    pub fn new(cfg: VlConfig) -> Self {
        Self { cfg }
    }

    /// One-shot: "what is in this image, relative to this prompt?".
    ///
    /// Returns the assistant's textual reply. The caller interprets it.
    pub fn understand(&self, prompt: &str, image: ImageRef<'_>) -> Result<String, VlError> {
        let image_url = match image {
            ImageRef::Url(u) => u.to_owned(),
            ImageRef::LocalPath(p) => encode_local_image(p)?,
        };

        let body = json!({
            "model": self.cfg.model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text",      "text": prompt},
                    {"type": "image_url", "image_url": {"url": image_url}}
                ]
            }]
        });

        let url = format!("{}/v1/chat/completions", self.cfg.base_url.trim_end_matches('/'));
        let agent = ureq::AgentBuilder::new().timeout(self.cfg.timeout).build();
        let resp = agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.cfg.api_key))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| VlError::Http(e.to_string()))?;

        let json: Value = resp.into_json().map_err(|e| VlError::Decode(e.to_string()))?;

        json.pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(VlError::Empty)
    }
}

fn encode_local_image(p: &Path) -> Result<String, VlError> {
    let mime = mime_for(p).ok_or_else(|| VlError::UnknownMime(p.display().to_string()))?;
    let bytes = std::fs::read(p)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

fn mime_for(p: &Path) -> Option<&'static str> {
    match p.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mime_lookup() {
        assert_eq!(mime_for(&PathBuf::from("x.png")), Some("image/png"));
        assert_eq!(mime_for(&PathBuf::from("x.JPG")), Some("image/jpeg"));
        assert_eq!(mime_for(&PathBuf::from("x.bmp")), None);
    }

    #[test]
    fn encode_data_url() {
        let mut p = std::env::temp_dir();
        p.push("cd-brief-vl-test.png");
        std::fs::write(&p, b"\x89PNG\r\n\x1a\n").unwrap();
        let url = encode_local_image(&p).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn config_from_env_reads_deepseek_fallback() {
        let _guard_a = EnvGuard::set("CODEDESIGN_DEEPSEEK_API_KEY", "sk-test-brief");
        let _guard_b = EnvGuard::unset("CODEDESIGN_VL_API_KEY");
        let _guard_c = EnvGuard::unset("CODEDESIGN_VL_MODEL");
        let c = VlConfig::from_env().expect("config");
        assert_eq!(c.api_key, "sk-test-brief");
        assert_eq!(c.model, "deepseek-vl2");
    }

    // tiny env guard; tests that touch env run serialised via --test-threads=1
    // but each guard still restores to avoid leak.
    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }
    impl EnvGuard {
        fn set(k: &str, v: &str) -> Self {
            let prev = std::env::var(k).ok();
            std::env::set_var(k, v);
            Self { key: k.into(), prev }
        }
        fn unset(k: &str) -> Self {
            let prev = std::env::var(k).ok();
            std::env::remove_var(k);
            Self { key: k.into(), prev }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }
}
