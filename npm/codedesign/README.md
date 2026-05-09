# codedesign

Terminal design agent — generate Figma designs from natural-language PRDs.

> v0.0.1 is an early preview. Apple Silicon Mac (darwin-arm64) only.
> Linux / Windows / Intel Mac support coming in later versions.

## Install

```bash
npm install -g @wz927/codedesign
```

## Setup

You need a DeepSeek-compatible chat-completions endpoint. Either the real
DeepSeek API or any OpenAI-compatible proxy (e.g. Kimi via oneapi).

```bash
export DEEPSEEK_API_KEY="sk-..."
export DEEPSEEK_BASE_URL="https://api.deepseek.com"     # optional, default
export DEEPSEEK_MODEL="deepseek-chat"                    # optional, default
```

## Use

### 1. Generate a Figma plugin script from a PRD

```bash
codedesign design "Mobile login page with email + password, primary CTA, forgot-password link. Restrained style."
```

This writes two files to the current directory:

- `codedesign-output.js` — a self-contained Figma Plugin API script
- `codedesign-output.plan.json` — the DesignPlan IR (for debugging)

### 2. Run the script inside Figma (v0.0.1 — manual paste mode)

Because v0.0.1 does not yet speak the Figma Remote MCP protocol, you run
the generated script as a local Figma plugin:

1. Open **Figma Desktop** → any file.
2. **Plugins → Development → New plugin → Create new plugin from scratch**.
3. Name it anything (e.g. `codedesign-dev`). Figma creates a folder with a
   `manifest.json` and a `code.js`.
4. Replace the contents of `code.js` with the contents of
   `codedesign-output.js`.
5. Back in Figma: **Plugins → Development → Open Console** (so you can see
   errors), then **Plugins → Development → codedesign-dev**.

You should see frames appear on the current page.

> If you see `__cdBindFill is not defined` etc., that's expected in v0.0.1 —
> the design-system runtime shim ships in v0.0.2. For now the script will
> create frames, layouts, and text content; variable/style/component bindings
> will error gracefully and land in the `errors[]` array inside the emitted
> result.

### 3. Compile an existing Plan JSON (debugging)

```bash
codedesign compile ./some-plan.json --out ./out.js
```

## What's in v0.0.1

- End-to-end pipeline: PRD → Plan (via DeepSeek-compatible provider) → Plugin JS.
- Tight `plan_emit` JSON Schema + 1-shot example enforce canonical DesignPlan
  shape (no more `regions[]` improvisation).
- Deterministic compiler (cd-canvas): byte-identical JS on repeated runs.
- Default DS token vocabulary baked in (grayscale + single accent, no emoji).

## What's planned

- **v0.0.2** — Figma Remote MCP passthrough (`use_figma`), so you don't have to
  paste. DS runtime shim (`__cdBindFill` etc.) ships as part of the plugin
  manifest scaffold.
- **v0.1** — Multi-platform binaries (linux-x64, linux-arm64, windows-x64,
  darwin-x64).
- **v0.1** — Interactive REPL (`codedesign`, no subcommand).
- **v0.2** — Custom DesignSystems, case library (per
  `DESIGN.md §2.4`), vision-mode Plan role.

## Troubleshooting

**`DEEPSEEK_API_KEY not set`** — export the variable (see Setup).

**`model did not return a tool_call`** — the provider's routing may have
blocked tool-call mode. Try a different `DEEPSEEK_MODEL` or pin
`DEEPSEEK_MODEL=kimi-k2.6` if you use the oneapi-comate proxy.

**`model returned a Plan that does not match DesignPlan schema`** — the
raw plan was saved; open an issue with it attached.

## License

MIT.
