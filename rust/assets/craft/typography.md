# Typography Craft

## Must
- 正文字号 ≥ 14px(mobile)/ 13px(dense desktop tool,如 table row)
- 同一 frame 最多 3 种字号 + 2 种字重
- 行高:正文 1.45–1.6;标题 1.15–1.3
- Display 级(≥ 40px)**禁用系统 sans-serif**(Helvetica / Arial / PingFang 等),必须用有设计意图的字族(e.g., Inter Display, Söhne, Playfair, Pretendard)

## Never
- 不用 `italic` 做强调(中文场景几乎总错);用字重 / 颜色 / 尺寸做强调
- 不使用 `letter-spacing` 夸张正文(> 0.05em 或 < -0.02em);标题可酌用
- 不在同屏混用衬线 + 非衬线**多于一处**;有意对比除外

## Check list(Gate-3)
- 每个 frame 的字号集合 ≤ 3
- 每个 frame 的字重集合 ≤ 2
- 所有文本样式必须通过 `styleRef` 绑定,禁止 inline font
