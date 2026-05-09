# 04 — Craft(工艺公理)

Craft 轴是跨品牌通用的工艺约束,与 DS(品牌特定)和 Skill(场景特定)正交。每个 DesignPlan 的 `craft_asserts` 字段必须至少列出下列规则之一,且当前文件 `/assets/craft/` 下的每条规则你都必须遵守。

## 五条规则文件

1. `typography.md` — 字阶、字重、行高配比;禁止超系统字重;Display 级禁用系统 sans
2. `color.md` — 语义化颜色 token 分层;品牌色不做功能色用;WCAG AA 为底线
3. `spacing.md` — 4 或 8 的倍数基准;不允许奇异 padding;垂直节奏优先
4. `state-coverage.md` — default/hover/active/focus/disabled/loading/empty/error 八态必备;空态禁用"暂无数据"这种空话,必须有引导动作
5. `anti-ai-slop.md` — 见 `/assets/craft/anti-ai-slop.md` + `anti-ai-slop.json` 的 7 宗罪硬列表

## 行为

- 你产出的每个 frame 必须显式声明它符合上述哪几条(写进 DesignPlan `craft_asserts`)
- 你不得"发明"新 craft 规则;craft 文件是可审计资产,只能通过 PR 增删
- 若 DS `constraints.json.craft.override` 明确豁免某条规则,你方可在该文件内使用该豁免(但仍需声明 `usesOverride: true`)
