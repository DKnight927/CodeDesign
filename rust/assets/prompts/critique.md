# critique — Five-Dimensional Scoring

> 本文件是 Critique turn 特殊 prompt,与其他模块**不拼接**(跳过 01 loop / 06 skill / 02 IR / 07 tools)。runtime 会在 CRITIQUE state 激活。

## 五维

| 维度 | 观察什么 |
|---|---|
| 1. Philosophy consistency | DS DESIGN.md 的 9 段哲学在此页面是否被一贯执行(特别是 voice、posture、density) |
| 2. Hierarchy & IA | 视觉层级与信息架构是否对齐;首屏能否 3 秒看懂主任务 |
| 3. Craft | 字阶 / 对齐 / 间距 / state 覆盖 / 排版细节 |
| 4. Functionality | 主任务能否走通;空态/错态/加载态是否闭环;可访问性(对比度、焦点可见、字号下限) |
| 5. Innovation | 在 DS 约束内的"高水准选择"(不是花哨,是更好的默认);此维打分上限 8,不得 9 或 10 |

## 打分区间

- 9–10:**禁止**,除非该维度是标志性的行业最佳水准范例
- 7–8:明确超出 DS 平均水平,有可引用原因
- 5–6:合规但平凡
- 3–4:存在可定位缺陷,影响可用
- 1–2:失误,破坏体验

**纪律**:

1. 任何 `score >= 6` 必须写 `why_not_5: "..."`,任何 `score >= 8` 必须写 `why_not_7: "..."`
2. 每条 finding 必带 `nodeId`(指向具体节点)+ `evidence`(一句可观察事实)+ `fixOps`(具体修改建议)
3. **总分 = 最差维度分**(worst-of-5,不取平均)— 短板即瓶颈
4. 禁用空话:`looks good` `feels clean` `professional` — 写 evidence 不写感受

## 输出 schema

```json
{
  "planId": "...",
  "scores": {
    "philosophy": {"score": 6, "band": "acceptable", "why_not_5": "..."},
    "hierarchy": {"score": 7, "band": "good", "why_not_5": "...", "why_not_7": null},
    "craft": {"score": 5, "band": "acceptable"},
    "functionality": {"score": 4, "band": "needs_work"},
    "innovation": {"score": 5, "band": "acceptable"}
  },
  "totalScore": 4,
  "worstDimension": "functionality",
  "findings": [
    {
      "dimension": "functionality",
      "severity": "P1",
      "nodeId": "1:234",
      "evidence": "The error state of the form has no retry affordance; users hit a dead end.",
      "fixOps": [
        {"op": "addChild", "parent": "1:234", "instanceRef": "button.secondary", "props": {"label": "Retry"}}
      ]
    }
  ]
}
```
