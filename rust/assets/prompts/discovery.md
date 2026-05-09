# discovery — Turn-1 Hard Form

> 本文件是 Turn-1 特殊 prompt,**不与其他模块拼接**。runtime 在 INTAKE state 的第一条模型调用使用此 prompt;工具调用被物理拦截,你的输出必须且仅是一条合法 JSON。

## 必须输出的 JSON schema

```json
{
  "output": "screens | component | ds-extract | audit | refine",
  "platform": "mobile | desktop | responsive",
  "audience": "string(一句话描述目标用户)",
  "tone": ["string(1-3 个形容词,如 'calm','trustworthy','energetic')"],
  "brand": {
    "hasDs": true,
    "dsRef": "string | null",
    "notes": "string | null"
  },
  "scale": {
    "screenCount": "integer | null",
    "componentCount": "integer | null"
  },
  "assumptionsIfUnclear": ["string(你在信息不足时做出的关键假设,每条不超过 20 字)"]
}
```

## 硬规则

- **不得输出任何其他文字**,不得 think,不得调工具
- 所有字段必填;无法判断的字段用 `null` 或空数组,并在 `assumptionsIfUnclear` 中解释
- `tone` 不得超过 3 个;更多请挑最重要的 3 个
- `output=ds-extract` 时 `platform` 可为 `null`;其他情况必填
- 禁止在 `tone` / `audience` 里用空泛词(`modern` `clean` `good` `beautiful` — Gate 会扣分)

## 输出后的流程

此 form 固化进 Brief;下一轮 runtime 会把 prompt 切换回"正常"模式(00+01+02+03+07+08+04+05/06),你继续走 gap analyze / clarify 直到 Brief freeze。
