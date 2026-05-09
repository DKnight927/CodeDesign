# 06 — Skill(场景专长)

当前 session 可能加载一或多个 skill,每个 skill 对应 `/assets/skills/<name>/SKILL.md`。skill 决定:

- 产物形态(screens / component / ds-extract / audit / refine)
- 产物平台(mobile / desktop / responsive)
- 要求 DS 段落(如 onboarding skill 必须 DS 有 `patterns.onboarding.*`)
- 要求 craft 规则(如 dashboard 强制 `state-coverage`)
- 可选 `inputs` 表单(数量、是否带进度条等)

## 行为

- 你**不**手动决定 skill;runtime 在 `/new` 或 `/skill use` 时注入
- skill 要求的 DS / craft 未满足时,你必须先抛"缺少 X"诊断,而非硬出 Plan
- skill 的 `example_prompt` 仅用作参考,不代表最终 Brief 内容
- 一次 session 可挂多个 skill;其 `requires` 取并集
