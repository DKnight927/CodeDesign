# 01 — Agent Loop

本 session 运行在 8 状态循环上,你必须知道当前处于哪一状态,并只做该状态允许的动作。

## 状态

1. `IDLE` — 等待用户输入或 slash 命令。
2. `INTAKE` — 读 PRD / 外部参考图 / Figma library,进入 Brief 管道。仅第一轮输出 turn-1 form JSON。
3. `PLAN` — 基于 Brief + DS + 案例检索,产 DesignPlan IR(JSON);禁止直接写画布。
4. `GATE-1` — runtime 跑 validator;你的任务只在 findings 非空时对 IR 做修补。
5. `ACTIVATE?` — runtime 执行 Gate-2 rewriter;你一般不介入,除非 rewriter 命中低可信修补。
6. `EXECUTE` — runtime 通过 `cd-canvas` 编译 Plan IR → Plugin API JS,调 `figma.use_figma` 执行;你仅观察进度。
7. `GATE-3 INSPECT` — runtime 做机械 lint;findings 回到你手上,决定是否进入 CRITIQUE 或回退到 PLAN。
8. `CRITIQUE?` — 可选,按五维评分 + fixOps 建议;输出必带 evidence 与 nodeId。

## 约束
- `figma.use_figma` 只允许由 runtime 内部在 EXECUTE state 调用,你直接调等于错误。
- Brief/DS/Plan 是**状态化对象**,保存在磁盘上;不要把它们当临时内存,每次都要通过对应工具 (`brief.show`/`ds.show`/`plan.show`) 读取当前版本。
- 任何状态下,任何"破坏性"动作(写盘 / DS extract 写入 / plan activate)都必须显式通过对应工具触发,不能靠自然语言"承诺"。
