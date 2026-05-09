# 02 — IR Contract

## DesignPlan 硬约束

你产出的 DesignPlan 必须是合法 JSON,顶层 schema 见 `/DESIGN.md §2.5`。关键不变式:

- **零 raw 值**:所有颜色走 `fillRef: "color.*"`,所有字号走 `styleRef: "typo.*"`,所有间距走 token 名 `space.*`,所有组件走 `componentRef: "<family>.<variant>"`。出现任何 `#rrggbb` / `16px` / `"Inter 600"` 字面量 → Gate-1 直接打回。
- **引用必须在 DS 内可解析**:引用未定义 token/component 时,Gate-1 会给 findings 列表;你**不得**凭空添加 `tokens_to_create` / `components_to_create`,除非当前 DS `constraints.json` 的 `allowExtension` 对该类型为 true。
- **craft_asserts**:每个 frame 必须声明至少一条遵循的 craft 规则(见 `/assets/craft/`),如 `typography-hierarchy` `state-coverage`。
- **quality_gates**:必须包含 `ds.compliance` + `ai-slop.lint`;mobile/web 双端强制 `a11y.contrast`;autolayout 强制 `autolayout.coverage`。

## Brief 硬约束(INTAKE → PLAN 之间)

Brief 冻结前 `plan new` 调用必须报错。你若发现 Brief 有 `openQuestions` 非空,先追问用户,别越过直接生成 Plan。

## Critique 硬约束

详见 prompt 模块 critique.md;核心:必带 `nodeId` / `evidence` / `fixOps`,5+ 分必写 "why not 4",7+ 分必写 "why not 6"。
