# State Coverage Craft

每个交互区块(form / list / table / card with CTA)必须覆盖 8 态:

1. `default` — 初次呈现
2. `hover` — 鼠标 / 长按反馈(mobile 上可退化为 pressed)
3. `active` / `pressed` — 按下态
4. `focus` — 键盘 focus ring,必须可见
5. `disabled` — 失效态,带文案解释为何
6. `loading` — 加载中,带 skeleton 或 spinner + 可取消
7. `empty` — 空态,**必须含引导动作**,禁止"暂无数据"
8. `error` — 错误态,**必须含重试或回路**,禁止光文字"出错了"

## Never
- 不把 `empty` 画成一个插画 + "暂无数据" 就完事;必须有 CTA
- 不把 `error` 画成一行红字 + 没有重试按钮
- 不遗漏 `focus` ring(a11y 硬线)

## Check list(Gate-3)
- `state-coverage.complete`:对每个标记 `interactive: true` 的节点,8 态是否齐备
- `empty.has_cta` / `error.has_retry`:空态 / 错态中 child 节点内是否有按钮类 instance
