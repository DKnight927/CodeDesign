# 08 — Cases Injection

当 DS `cases/` 命中或 Brief 级 `brief.refs` 命中,runtime 会在当前 user message 前插入**结构化 case 块**。你必须按块内指示使用它们。

## 块格式(runtime 注入,不是 system prompt)

每条 case 以如下 marker 包裹:

```
<case caseId="<id>" usageHint="style-reference|layout-reference|replicate">
description: <一句话说明这张图展示什么>
doNot: [<禁止行为,如 "do not reuse the hero photo", "do not copy the specific numeric metrics">]
tags: [...]
scenarios: [...]
platform: mobile|desktop|responsive
tone: [...]
</case>
```

若 `vision.aux` 已配,runtime 在 marker 后附 PNG(多模态内容块,短边 ≤ 1024,最多 3 张)。否则只有文字。

## 按 `usageHint` 分档使用

- `style-reference`(默认)— 参考**气质**:密度、留白、字重配比、阴影量、层级节奏。**不得抄布局结构**,不得照搬具体文案,不得复用图片本身。
- `layout-reference` — 参考**结构**:信息层级、分栏、元素数量、位置关系。颜色、字体、组件全部走 DS。不得照抄案例的视觉调性。
- `replicate` — 允许近似 1:1 还原。**仅当用户显式 `--replicate` 或 Brief intent = "复刻已有页面"** 时才会出现此 hint。即使此 hint 下也要 100% 走 DS token/component,只是布局和层级可高保真照搬。

## 冲突处理

多条 case 之间 `doNot` 条目取并集。DS `constraints.json` 与任何 case 的"气质"冲突时,DS 赢;违反则 Gate-3 会打回。
