# S0.1 — DeepSeek Tool-Calling Parity

## 目的
验证 DeepSeek(OpenAI-compat)在以下场景能和 claw-code 的 `api::OpenAiCompatClient` 干净跑通。若达标,后续 `cd-cli` 直接设置 `OPENAI_BASE_URL=https://api.deepseek.com` 即可。

## 10 个用例

| id | 场景 | 通过判据 |
|---|---|---|
| tc-01 | 单工具调用(`file.read`) | 模型返回 `tool_calls[0].function.name="file.read"`,参数合法 JSON |
| tc-02 | 并行调 2 个读工具 | 一条响应内 2 个 `tool_calls` |
| tc-03 | 嵌套 JSON 参数(Plan IR 片段) | 模型产出的 JSON 能被 serde 反序列化到 `DesignPlan` schema |
| tc-04 | `response_format: json_schema`(turn-1 form) | 响应直接是合法 `turn1Form` JSON,无 markdown fence |
| tc-05 | tool_result 带 error=true 后的恢复 | 模型识别错误并调替代工具或请教用户 |
| tc-06 | 多轮 tool loop(5+ 轮) | 不提前终止,不陷死循环 |
| tc-07 | 大 tool result(> 4k token) | 正确处理裁剪后的 `{ ref, summary, top-k }` |
| tc-08 | `tools=[]` 下被迫文本输出 | 不幻觉调用不存在的工具 |
| tc-09 | tool_choice 强制选某个工具 | 遵循 `tool_choice={"type":"function","function":{"name":"..."}}` |
| tc-10 | UTF-8 中文参数 + 图片路径 | 中文参数正确回传,不 URL-encode 破坏原值 |

## 运行

这些用例将以 `#[ignore]` 的集成测试形式写在 `cd-cli` / `cd-tools` 的 tests 里,需要 `DEEPSEEK_API_KEY` 才能跑:

```
cargo test --package cd-cli --test deepseek_toolcall -- --ignored
```

在 `result.md` 中记录每条用例的 ✅/❌ + 失败原因。

## 判据

- 10 条至少 9 条 ✅ → S0.1 pass
- ✅ ≥ 7 且失败用例非 tc-04 / tc-05 → partial,可进 Phase 1 但需降级策略
- 其他情况 → no-go,回 DESIGN.md §13.4 调整 provider 策略
