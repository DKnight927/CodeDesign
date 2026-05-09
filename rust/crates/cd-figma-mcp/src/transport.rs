//! stdio JSON-RPC transport for MCP.
//!
//! Framing: each JSON-RPC message is one UTF-8 line (LF-terminated) on
//! the child's stdout/stdin. This matches the "newline-delimited JSON"
//! variant used by Node MCP servers spawned via `npx`. We deliberately
//! do NOT implement the Streamable-HTTP transport here — the only
//! currently-reachable backend is local `npx figma-console-mcp`.
//!
//! The transport is synchronous from the caller's perspective:
//! [`StdioTransport::request`] sends a JSON-RPC request and blocks
//! until the matching response arrives (matched by `id`) or the
//! timeout fires. A background reader thread demultiplexes responses
//! and pushes server-originated notifications onto a drop-tolerant
//! inbox (we currently ignore notifications, but they must be drained
//! so the server doesn't block on a full pipe).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{json, Value};

use crate::error::{Error, Result};

/// Default per-request timeout. `figma_execute` can take a few seconds
/// when the plugin is creating many nodes, so this is generous.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Background dispatch: one entry per outstanding request id.
type Pending = Arc<Mutex<HashMap<i64, Sender<RpcOutcome>>>>;

enum RpcOutcome {
    Ok(Value),
    Err(Error),
}

pub struct StdioTransport {
    child: Child,
    /// Wrapped in `Option` so `shutdown` can drop it early (sending
    /// EOF to the child's stdin) without clashing with our `Drop`.
    stdin: Option<ChildStdin>,
    next_id: AtomicI64,
    pending: Pending,
    _reader: JoinHandle<()>,
    /// Drained by reader on stderr; kept here to prevent PIPE buffer
    /// blocking the child. We don't surface it for now — future work
    /// is to tee it to a ring buffer for `codedesign doctor` triage.
    _stderr_pump: JoinHandle<()>,
}

impl StdioTransport {
    /// Spawn a child process and start pumping its stdio.
    ///
    /// `env` entries override (not extend) any inherited values for
    /// those keys. The caller is responsible for passing in
    /// `FIGMA_ACCESS_TOKEN`.
    pub fn spawn(program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<Self> {
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|e| {
            Error::Subprocess(format!("failed to spawn `{program}`: {e}"))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Subprocess("child has no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Subprocess("child has no stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Subprocess("child has no stderr".into()))?;

        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let reader_pending = pending.clone();

        let reader = thread::spawn(move || {
            let buf = BufReader::new(stdout);
            for line in buf.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(v): std::result::Result<Value, _> = serde_json::from_str(trimmed) else {
                    // Non-JSON line on stdout (some servers warm up
                    // with banners). Ignore it; if the server is
                    // unhealthy, the request timeout will surface.
                    continue;
                };
                // JSON-RPC response has an `id`; notification has no
                // `id`. We only care about responses.
                let Some(id) = v.get("id").and_then(Value::as_i64) else {
                    continue;
                };
                let outcome = if let Some(err) = v.get("error") {
                    let code = err.get("code").and_then(Value::as_i64).unwrap_or(-32000);
                    let msg = err
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("(no message)")
                        .to_string();
                    RpcOutcome::Err(Error::Rpc { code, message: msg })
                } else if let Some(result) = v.get("result") {
                    RpcOutcome::Ok(result.clone())
                } else {
                    RpcOutcome::Err(Error::Protocol(format!(
                        "response missing both result and error: {v}"
                    )))
                };

                if let Some(tx) = reader_pending.lock().unwrap().remove(&id) {
                    // Receiver may have timed out; ignore send error.
                    let _ = tx.send(outcome);
                }
            }
            // EOF on stdout: mark all pending as failed so waiters
            // unblock instead of hanging on recv_timeout.
            let mut pmap = reader_pending.lock().unwrap();
            for (_, tx) in pmap.drain() {
                let _ = tx.send(RpcOutcome::Err(Error::Subprocess(
                    "child stdout closed before response arrived".into(),
                )));
            }
        });

        let stderr_pump = thread::spawn(move || {
            // Drain stderr to prevent the child from blocking on a
            // full pipe. We do not forward it by default; any real
            // connectivity error will surface as a request timeout or
            // stdout EOF.
            let buf = BufReader::new(stderr);
            for line in buf.lines().map_while(std::result::Result::ok) {
                // When debugging locally, set CODEDESIGN_MCP_DEBUG=1
                // to echo the child's stderr. Off by default — the
                // server emits noisy informational lines.
                if std::env::var("CODEDESIGN_MCP_DEBUG").is_ok() {
                    eprintln!("[figma-console-mcp] {line}");
                }
            }
        });

        Ok(Self {
            child,
            stdin: Some(stdin),
            next_id: AtomicI64::new(1),
            pending,
            _reader: reader,
            _stderr_pump: stderr_pump,
        })
    }

    /// Send a JSON-RPC request and wait for the matching response.
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.request_with_timeout(method, params, DEFAULT_TIMEOUT)
    }

    pub fn request_with_timeout(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel::<RpcOutcome>();
        self.pending.lock().unwrap().insert(id, tx);

        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = format!("{}\n", serde_json::to_string(&req)?);
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Subprocess("transport already shut down".into()))?;
        if let Err(e) = stdin.write_all(line.as_bytes()) {
            self.pending.lock().unwrap().remove(&id);
            return Err(Error::Io(e));
        }
        stdin.flush().ok();

        match rx.recv_timeout(timeout) {
            Ok(RpcOutcome::Ok(v)) => Ok(v),
            Ok(RpcOutcome::Err(e)) => Err(e),
            Err(RecvTimeoutError::Timeout) => {
                self.pending.lock().unwrap().remove(&id);
                Err(Error::Timeout(timeout))
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.pending.lock().unwrap().remove(&id);
                Err(Error::Subprocess("transport reader died".into()))
            }
        }
    }

    /// Send a JSON-RPC notification (no id, no response expected).
    pub fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let line = format!("{}\n", serde_json::to_string(&msg)?);
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Subprocess("transport already shut down".into()))?;
        stdin.write_all(line.as_bytes())?;
        stdin.flush().ok();
        Ok(())
    }

    /// Terminate the child. Idempotent; ignores "already gone" errors.
    pub fn shutdown(mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.flush();
            // dropping stdin here signals EOF to the child
            drop(stdin);
        }
        // Give the child a moment to exit cleanly, then kill.
        let grace = Duration::from_millis(500);
        let deadline = std::time::Instant::now() + grace;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if std::time::Instant::now() >= deadline => break,
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(_) => break,
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Best-effort child cleanup if caller forgot to shutdown.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    //! These tests use a tiny hand-rolled JSON-RPC echo server written
    //! as a shell one-liner. They prove the framing / id-matching /
    //! timeout paths without depending on node/npx being available.

    use super::*;

    fn sh(script: &str) -> (String, Vec<String>) {
        ("sh".into(), vec!["-c".into(), script.into()])
    }

    #[test]
    fn request_response_roundtrip() {
        // Echo server: for every line in, emit {"jsonrpc":"2.0","id":<id>,"result":{"echo":<params>}}.
        let script = r#"
            while IFS= read -r line; do
                id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
                printf '{"jsonrpc":"2.0","id":%s,"result":{"ok":true}}\n' "$id"
            done
        "#;
        let (prog, args) = sh(script);
        let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut t = StdioTransport::spawn(&prog, &args_ref, &[]).expect("spawn");
        let v = t.request("ping", json!({"hello": "world"})).expect("rpc");
        assert_eq!(v["ok"], json!(true));
        t.shutdown();
    }

    #[test]
    fn rpc_error_surfaces() {
        let script = r#"
            while IFS= read -r line; do
                id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
                printf '{"jsonrpc":"2.0","id":%s,"error":{"code":-32601,"message":"nope"}}\n' "$id"
            done
        "#;
        let (prog, args) = sh(script);
        let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut t = StdioTransport::spawn(&prog, &args_ref, &[]).expect("spawn");
        let err = t.request("whatever", json!({})).unwrap_err();
        match err {
            Error::Rpc { code, message } => {
                assert_eq!(code, -32601);
                assert_eq!(message, "nope");
            }
            other => panic!("expected Rpc error, got {other:?}"),
        }
        t.shutdown();
    }

    #[test]
    fn timeout_on_silent_server() {
        // Consumes stdin but never writes to stdout.
        let script = r#"while IFS= read -r _; do :; done"#;
        let (prog, args) = sh(script);
        let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut t = StdioTransport::spawn(&prog, &args_ref, &[]).expect("spawn");
        let err = t
            .request_with_timeout("hang", json!({}), Duration::from_millis(100))
            .unwrap_err();
        assert!(matches!(err, Error::Timeout(_)), "got {err:?}");
        t.shutdown();
    }
}
