# Phase 0 — Spike Harness

> 目标:在 Phase 1 编码前把四项不确定打穿。每个子目录是一个独立 eval/spike,产出独立报告,最终汇总到本目录 `report.md`。

## 子任务

- `deepseek-toolcall/` — **S0.1**:DeepSeek(OpenAI-compat)的 tool calling 是否和 Anthropic 等价。10 个用例覆盖并行 / 嵌套 JSON / json_schema / 错误恢复。
- `use-figma-compiler/` — **S0.2**:`cd-canvas` 把 Plan IR 编译成 Plugin API JS 的确定性 / 最小可执行片段。
- `plan-case-injection/` — **S0.3**:无视觉模型下,用文字化 case 元数据做 Plan 注入的还原度基线(3 份样例人评)。
- `summarizer/` — **S0.4**:`cd-summarizer` 独立 runtime 压缩可行性。

## 产出

每个子目录至少包含:
- `README.md` — 目的 / 方法 / 判据
- `cases/` — 固定输入样例(JSON / PRD 文本 / 目标 DS)
- `run.md` 或 `run.rs` — 复现步骤
- `result.md` — 实际跑出的结果,含 pass/fail 判定与 nodeId/log 引用

## go/no-go 判据汇总到 `report.md`

Phase 0 完结时,`report.md` 需明确回答:
1. Phase 1 可以开工吗?
2. 如否,哪项必须先补?
