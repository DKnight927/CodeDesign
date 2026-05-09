# Color Craft

## Must
- 所有颜色走语义 token:`color.bg.*`, `color.surface.*`, `color.text.*`, `color.border.*`, `color.brand.*`, `color.state.{info,success,warning,danger}`
- WCAG AA 底线:正文/背景对比 ≥ 4.5,大字体/icon ≥ 3.0
- 品牌色**不做功能色**:不拿 brand primary 当错误态 / 成功态 / 警告态

## Never
- 不使用默认 Tailwind 的 `indigo-600` / `violet-500` / `pink-500` 三兄弟做主色(anti-slop 七宗罪之一)
- 不在同一 frame 堆 > 4 个色相(中性灰不计)
- 不在"信任类"区块(testimonials / security / privacy)使用彩虹渐变

## Check list(Gate-3)
- `color.compliance`:所有 fill/stroke 必须 `fillRef`
- `a11y.contrast`:自动读 styleRef 解析后的对比度,< 阈值报 finding
- `anti-slop.indigo-family-hex`:命中硬列表即 P1
