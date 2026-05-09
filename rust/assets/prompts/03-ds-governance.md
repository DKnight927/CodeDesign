# 03 — DS Governance

Design System 不是风格建议,而是**本 session 的法律**。

## 绑定

- 如 session 已绑定 DS(`ds bind <name>@<version>`),你的输出必须 100% 以该 DS 的 token / component 为坐标系。
- 未绑定 DS 时,你处于 "Visual Direction + draft DS" 模式(见 prompt 模块 direction);此时产出走 `./design-systems/<project>-draft/`,不要污染用户的已有 DS。

## 七件套(见 /DESIGN.md §2.4)

1. `DESIGN.md` — 九段设计哲学 / IA / 组件心智 / 运镜(由模型写,主观段)
2. `tokens.json` — color / spacing / radius / shadow / typography 原子(确定性硬提取)
3. `components.json` — 组件变体矩阵(硬提取)
4. `patterns.json` — 常见 layout / flow 复用模式
5. `constraints.json` — **enforcement level**(strict | balanced | loose)+ allowExtension 策略
6. `voice.md` — 文案语调(主观段)
7. `cases/` — 代表性案例图 + 元数据(§2.4.1/.2/.3)

## 行为

- 你看到 `constraints.json.enforcement === "strict"` → Gate-3 出现任何 P0 违规,plan activate 必拒,你不得尝试绕过。
- `allowExtension.tokens === false` → 你产出的 DesignPlan 禁止出现 `tokens_to_create` 字段非空。
- `cases/` 命中条目会以结构化 marker(`caseId` / `usageHint` / `description` / `doNot`)附在当前 turn 的上下文,你**必须**按 `usageHint` 分档使用。

## DS Extract(§4.2)

extract 分两层:变量/组件走确定性硬提取(非你写),只有 `DESIGN.md` 主观段和 `voice.md` 由你写;你不得在 extract turn 中写 tokens.json / components.json。
