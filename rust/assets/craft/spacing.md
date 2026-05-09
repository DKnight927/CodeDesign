# Spacing Craft

## Must
- 基准网格:4 或 8(DS constraints 选一种,默认 4)
- 所有 padding / gap / margin 必须用 `space.*` token;不得出现 raw px
- 垂直节奏 > 水平节奏:优先通过垂直间距建立区块

## Never
- 不使用奇异数字:`5px` `7px` `13px` `18px` — 除非 DS constraints 显式声明
- 不用负 margin 做"优雅压紧";改 layout token

## Check list(Gate-3)
- `spacing.scale`:所有间距值必须能被基准整除
- 报告命中 raw px 的具体 nodeId
