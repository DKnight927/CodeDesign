# S0.3 — Scoring (human)

Executed 2026-05-09 on Kimi-K2.6 via `https://oneapi-comate.baidu-int.com/v1`
(see report.md §3 for provider notes).

Outputs in `./output/<sample-id>.plan.json`. Score each sample 1–5 on:

- **D1 Schema validity** — deserializes as `cd_plan::DesignPlan`, refs resolve
  against the DS summary (tokens + components)
- **D2 Style match** — direction gestalt (density, accent restraint, typography
  posture) visible in frame sizing, layout gap, typo refs
- **D3 doNot adherence** — no violation of DS `doNotRules` or any case-level
  `doNot` (e.g. emoji-as-status, multi-accent, raw decorative colors)

## Auto (D1) — all 3 samples schema_valid=false

All three samples produced a semantic DSL that does **not** deserialize into
the canonical `cd_plan::DesignPlan`. Deviation pattern (identical across a/b/c):

| Canonical DesignPlan                       | What Kimi emitted                        |
|---                                         |---                                       |
| `frames[].localId`                         | `frames[].id`                            |
| `frames[].page`, `.size`, `.layout.mode`   | missing; `.layout.type=centered/sidebar-main` |
| `frames[].children[Node enum]`             | `frames[].regions[]` with free-form `type` |
| `Node::Text{styleRef, fillRef, content}`   | `{type:text, style, color, content}`     |
| `Node::Instance{componentRef, props}`      | `{type:button/card/input, component, label}` |
| `fillRef: "color.bg.base"`                 | `fill: "color.bg.base"`                  |

**Root cause:** `plan_emit_tool()` JSON Schema declares `"frames":{"type":"array"}`
with no item shape. Model infers its own (more intuitive) vocabulary.

**Phase 1 M0 action:** rewrite `plan_emit_tool()` with full nested JSON Schema
matching `cd_plan::DesignPlan` + `Node` enum, plus a 1-shot minimal-plan
example in the Plan role system prompt. Tracked in report.md §9.

**D1 auto-score:** 1/5 for all samples (schema fails; refs superficially use
correct names but in wrong field positions).

## Human scoring (D2 / D3) — to be filled

| sample | D1 | D2 | D3 | avg | notes |
|---|---|---|---|---|---|
| sample-a | 1 |   |   |   | neutral-modern SaaS login/dashboard |
| sample-b | 1 |   |   |   | warm-editorial reading app |
| sample-c | 1 |   |   |   | tech-utility SRE dashboard |

**Pass gate (original):** overall avg ≥ 3.5 → text-only cases as default; < 3.0
→ promote `cases.images` (vision) to required.

**Gate interpretation:** Because D1 uniformly fails due to *our* schema
looseness (not the model's grasp of cases), D1 is not a valid signal here.
Re-run D1 after Phase 1 M0 tightens `plan_emit_tool()`. D2 / D3 scored by
human against the persisted JSON remain informative for the text-vs-vision
decision and will drive whether Phase 1 ships with text-only cases.

