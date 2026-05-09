# S0.3 — Plan Case Injection Baseline

## 目的
验证默认路径(用户无 vision provider)下,通过**文字化 case 元数据**注入给 DeepSeek-chat 能做出可接受的 Plan 草稿。该结果是"是否后续需要强推 vision provider"的决策基线。

## 方法

准备 3 对 (DS × PRD) 样例:

1. **样例 A** — `neutral-modern` direction + "小型 SaaS 登录 + 主页"
2. **样例 B** — `warm-editorial` direction + "读书笔记应用首页 + 笔记页"
3. **样例 C** — `tech-utility` direction + "实时指标看板 + 告警列表"

每个样例提供:
- DS 六件套(无 cases 图,但 cases/ 下有 3 条元数据 `{ description, doNot, tags, scenarios }`)
- 一段 PRD 摘要(~500 字)
- 期望 Plan IR schema(人写的"理想答案",用于比对)

## 运行

1. 把 DS + PRD + cases 元数据组装成 Plan prompt(按 `cd-prompts` 模板)
2. 调 DeepSeek-chat 出 Plan IR
3. 打分(1–5):
   - 结构正确性(IR schema 合法、引用可解)
   - 风格契合(direction 气质被捕捉的程度,人工判断)
   - 案例 `doNot` 遵循度(case 元数据里明令禁止的事是否被执行)

## 判据

- 3 样例平均 ≥ 3.5 → baseline 过线,Phase 1 默认路径够用,vision 可延后
- 3 样例平均 < 3.0 → 文字注入不足,Phase 1 开发阶段需要强制用户配 vision provider,或把 case 元数据表达做得更结构化
