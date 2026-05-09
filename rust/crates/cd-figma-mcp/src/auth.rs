//! Figma Personal Access Token storage.
//!
//! We need a Figma PAT (`figd_*` prefix) to hand to
//! `figma-console-mcp`; the server then uses it for REST calls and the
//! WebSocket bridge handshake. Precedence, highest first:
//!
//!   1. `CODEDESIGN_FIGMA_TOKEN` env var
//!   2. `FIGMA_ACCESS_TOKEN` env var (figma-console-mcp's own name;
//!      accepted for zero-config on machines that already set it)
//!   3. `~/.codedesign/auth.toml` field `figma.access_token`
//!
//! Storage uses file mode 0600 on Unix so the token is not world-readable.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const ENV_PRIMARY: &str = "CODEDESIGN_FIGMA_TOKEN";
const ENV_FALLBACK: &str = "FIGMA_ACCESS_TOKEN";
const AUTH_FILE_REL: &str = ".codedesign/auth.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthFile {
    #[serde(default)]
    pub figma: FigmaAuth,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FigmaAuth {
    /// Figma PAT (`figd_*`). Missing means "not yet configured".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// Resolve the Figma PAT using env → auth.toml precedence.
///
/// Returns `Err(Error::Auth)` when nothing is configured. The caller
/// (cd-cli) should translate this into a clear "run `codedesign doctor`"
/// message, never a silent fallback.
pub fn load_figma_token() -> Result<String> {
    if let Ok(t) = std::env::var(ENV_PRIMARY) {
        if !t.is_empty() {
            validate_pat(&t)?;
            return Ok(t);
        }
    }
    if let Ok(t) = std::env::var(ENV_FALLBACK) {
        if !t.is_empty() {
            validate_pat(&t)?;
            return Ok(t);
        }
    }

    let path = auth_file_path()?;
    if path.exists() {
        let txt = fs::read_to_string(&path)?;
        let af: AuthFile = toml::from_str(&txt)?;
        if let Some(t) = af.figma.access_token.filter(|s| !s.is_empty()) {
            validate_pat(&t)?;
            return Ok(t);
        }
    }

    Err(Error::Auth(format!(
        "no Figma access token found. set {ENV_PRIMARY} or run `codedesign doctor` to save one to {}",
        path.display()
    )))
}

/// Persist the token to `~/.codedesign/auth.toml` (mode 0600 on Unix).
/// Merges into any existing file so unrelated fields survive.
pub fn save_figma_token(token: &str) -> Result<PathBuf> {
    validate_pat(token)?;
    let path = auth_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut af: AuthFile = if path.exists() {
        let txt = fs::read_to_string(&path)?;
        toml::from_str(&txt).unwrap_or_default()
    } else {
        AuthFile::default()
    };
    af.figma.access_token = Some(token.to_string());

    let rendered = toml::to_string_pretty(&af)?;
    fs::write(&path, rendered)?;
    set_private_mode(&path)?;
    Ok(path)
}

fn auth_file_path() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| Error::Auth("HOME env var not set; cannot locate ~/.codedesign".into()))?;
    Ok(PathBuf::from(home).join(AUTH_FILE_REL))
}

/// Surface-level PAT sanity check. The real validation is Figma's
/// REST API; this just catches obvious paste errors.
fn validate_pat(token: &str) -> Result<()> {
    let t = token.trim();
    if t.is_empty() {
        return Err(Error::Auth("empty Figma token".into()));
    }
    if !t.starts_with("figd_") && !t.starts_with("figpat_") {
        return Err(Error::Auth(
            "Figma PAT should start with `figd_` or `figpat_`".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_mode(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(path)?.permissions();
    perm.set_mode(0o600);
    fs::set_permissions(path, perm)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_mode(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_pat_accepts_figd_prefix() {
        assert!(validate_pat("figd_abc123").is_ok());
        assert!(validate_pat("figpat_v1_abc").is_ok());
    }

    #[test]
    fn validate_pat_rejects_bad_input() {
        assert!(validate_pat("").is_err());
        assert!(validate_pat("sk-proj-deadbeef").is_err());
    }

    #[test]
    fn auth_file_roundtrip() {
        let af = AuthFile {
            figma: FigmaAuth {
                access_token: Some("figd_test_only".into()),
            },
        };
        let rendered = toml::to_string_pretty(&af).unwrap();
        let back: AuthFile = toml::from_str(&rendered).unwrap();
        assert_eq!(back.figma.access_token.as_deref(), Some("figd_test_only"));
    }
}
