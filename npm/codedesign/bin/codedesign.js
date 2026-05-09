#!/usr/bin/env node
// codedesign — Node shim that dispatches to the platform-specific native
// binary bundled under `binaries/`. See `.github/workflows/release.yml` for
// the naming contract (codedesign-{os}-{arch}[.exe]).

"use strict";

const { spawn } = require("node:child_process");
const path = require("node:path");
const fs = require("node:fs");

const SUPPORTED = {
  "linux-x64":   "codedesign-linux-x64",
  "linux-arm64": "codedesign-linux-arm64",
  "darwin-x64":  "codedesign-darwin-x64",
  "darwin-arm64": "codedesign-darwin-arm64",
  "win32-x64":   "codedesign-windows-x64.exe",
};

const key = `${process.platform}-${process.arch}`;
const binName = SUPPORTED[key];

if (!binName) {
  console.error(`codedesign: unsupported platform ${key}.`);
  console.error(`Supported: ${Object.keys(SUPPORTED).join(", ")}.`);
  console.error(`Build from source: cargo build --release -p cd-cli`);
  process.exit(2);
}

const binPath = path.join(__dirname, "..", "binaries", binName);

if (!fs.existsSync(binPath)) {
  console.error(`codedesign: bundled binary not found at ${binPath}`);
  console.error(`try: npm install -g @wz927/codedesign --force`);
  process.exit(3);
}

// Ensure executable bit survives npm extraction (no-op on Windows).
if (process.platform !== "win32") {
  try { fs.chmodSync(binPath, 0o755); } catch { /* best-effort */ }
}

const child = spawn(binPath, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 0);
  }
});

child.on("error", (err) => {
  console.error(`codedesign: failed to launch binary: ${err.message}`);
  process.exit(1);
});
