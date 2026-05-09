# Anti-AI-Slop Craft(七宗罪)

机械 lint,pure regex / node scan,不依赖模型自我评估。命中即报告,P0/P1 按下表。

## 七宗罪

1. **Tailwind-default indigo/violet/pink 滥用**(P1)— 见 `anti-ai-slop.json.bannedHex`
2. **Emoji 当功能 icon 使用**(P0)— 见 `anti-ai-slop.json.bannedEmoji` 出现在 button/tab/navigation 里
3. **信任区块用彩虹渐变**(P1)— testimonials/security/privacy/legal 节点子树下含渐变 fill
4. **编造数字 metric**(P1)— `%`、`x`、`$` 前后数字,且该数字在 Brief.content 或 DS 里无出处
5. **Lorem ipsum / 中文"占位符"文本**(P0)— 见 `anti-ai-slop.json.bannedPlaceholderRegex`
6. **Display 级系统 sans-serif**(P1)— 字号 ≥ 40 且 font 在 `bannedDisplayFonts` 列表
7. **"功能 1 / 功能 2 / 功能 3" 编号前缀**(P1)— feature 卡片 title 以数字 + 分隔符开头且内容为占位

## 行为
- lint 在 Gate-3 运行,读 Plan IR + 回读后的 Figma 节点树
- 命中 P0 → `ds activate` 拒绝(strict enforcement)/ 警告(balanced / loose)
- 所有命中清单写入 `./.codedesign/audit/<planId>/anti-slop.ndjson`
- DS `constraints.json.overrides.anti-slop` 可白名单豁免(例如品牌色恰好在 indigo 家族)
