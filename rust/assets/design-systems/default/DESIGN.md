# Default Design System

> 示例 DS,首次安装附带。用户 `ds extract` 成品应该长这样。

## 1. Philosophy
默认 DS 遵循"克制优于花哨"。所有元素默认为功能服务,装饰性元素必须有功能理由。

## 2. Information Architecture
- 一级导航:≤ 5 项
- 二级导航:≤ 8 项
- 面包屑在三级及以下出现

## 3. Component mental model
- `button` 有 `primary / secondary / tertiary / danger` 四种意图;尺寸三档 `sm / md / lg`
- `input` 必含 `label / helper / error` 三态
- `card` 默认无阴影,仅通过 `raised` 变体引入层级

## 4. Voice(见 voice.md)

## 5. Motion
- 进场 200ms ease-out;退场 150ms ease-in
- 禁止超过 300ms 的非关键动画

## 6. Density
airy。`comfortable` 作为高密度替代(table、dashboard 页)

## 7. Accessibility
AA 为底线;所有 interactive 节点必须有 focus ring

## 8. Do & Don't
- Do: 优先垂直节奏;空间承担层级责任
- Don't: 不用阴影做层级,用 surface + spacing

## 9. Case curation(见 cases/)
