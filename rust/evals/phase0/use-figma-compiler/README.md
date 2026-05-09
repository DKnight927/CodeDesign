# S0.2 — use_figma Compiler MVP

## 目的
验证 `cd-canvas` 能把一个最小 Plan IR 样例确定性地编译成一段 Plugin API JS,且这段 JS 通过 Figma Remote MCP 的 `use_figma(code, skillNames="figma-use")` 能在真实 Figma 文件中产出:

- 3 个 top-level frame
- 每个 frame 应用一个 fill(token 绑定)
- 每个 frame 内放一个 text node(token 绑定)
- 每个 frame 内放一个 component instance

## MVP IR 样例

见 `cases/minimal-plan.json`:3 帧 onboarding 骨架。

## 步骤

1. 在 `cd-canvas` 里实现 `compile(plan: &DesignPlan) -> Result<String, CompileError>`
2. 写**单元测试**(不打 Figma):输入 `cases/minimal-plan.json`,断言输出 JS 字符串:
   - 包含 `const plan_id = "..."` 注入
   - 所有 `figma.createFrame()` 调用数等于 IR 中 frame 数
   - 所有 `setBoundVariable(...)` 调用的 variable key 来自 DS tokens.json
   - `loadFontAsync` 在使用前调用
   - try/catch 包裹每个 frame block,失败返回 `{ createdNodeIds: [...], errors: [...] }`
3. 写**集成测试**(`--ignored`):需要 `FIGMA_MCP_URL` + `FIGMA_FILE_KEY`;跑 `use_figma` 后用 `get_metadata` 读回并断言节点结构。

## 判据

- 单测必过(在 Phase 0 内必做)
- 集成测试可选(取决于用户是否已绑 Figma MCP);若跑成功记为加分
- 无 Figma 账号也能完成 S0.2 的核心"编译器正确性"验证

## 产出

`result.md` 含:
- `compile()` 对 minimal-plan.json 的输出(完整 JS 贴出)
- 单测 pass 截图/日志
- 如跑了集成测试,Figma 文件 URL + 产出节点树截图
