# S0.4 — Summarizer Spike

## 目的
验证 `cd-summarizer` 作为独立 `ConversationRuntime` 实例可以在不污染主 session 的前提下,把一个接近 80% 预算的主对话压缩为 `SessionSummary`,且压缩后主 session 继续出 Plan 的质量退化不显著。

## 方法

1. 准备一个 mock 主 session:80 条消息(user/tool_result 混合),估算 ~100k tokens
2. 触发 compaction:启动第二个 runtime 跑 `summarize` 模式,输入上述 messages,输出
   ```
   SessionSummary {
     facts: [...],
     decisions: [...],
     openQuestions: [...],
     toolResultsDigest: [...]
   }
   ```
3. 把 summary 以 `role=system, name=summary` 注入主 session(放在 system prompts 之后、对话之前)
4. 在压缩后的主 session 上再跑一次同样的 Plan 生成 → 与压缩前的 Plan 对比

## 判据

- 压缩后主 session 总 token < 压缩前 50%
- Plan IR schema 合法性:压缩前后都通过 Gate-1
- 设计意图保留:压缩前 Brief 的关键字段(platform / tone / brand / 关键约束)在 summary 中可回查
- 主 session 账单独立:summarizer 调用的 token 不计入主 session 成本统计

## 产出
`result.md` 含:
- 压缩前后 token 统计
- `SessionSummary` 样本
- 压缩前后两份 Plan 的 diff
