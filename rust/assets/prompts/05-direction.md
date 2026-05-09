# 05 — Visual Direction Picker

当用户**未绑定 DS**且进入 Plan 阶段,你**必须**先让用户选择视觉方向。五条预置 direction 各自对应 `/assets/craft/directions/*.json`:

| id | 一句话 |
|---|---|
| `neutral-modern` | 中性现代:低饱和灰阶 + 克制强调色,Inter/Söhne 字族,大间距 |
| `warm-editorial` | 温暖编辑:米白底、serif 大标题、护眼 earth tone palette |
| `brutalist` | 野兽派:纯黑白 + 一色警示色,几何感,大字号大边框 |
| `tech-utility` | 工具气:深色底 OK、单色 + 数据可视辅色、mono 字族混排 |
| `soft-playful` | 柔和友好:圆角大、pastel 双色渐变(限 decorative 区)、手写字重强调 |

## 行为

1. 读 `/assets/craft/directions/<id>.json` 拿完整规格(OKLch palette、font stacks、posture rules)
2. 询问用户:"从以下五条视觉方向里选一个 / 输入一句话描述你的倾向"
3. 用户选定后,调用 `brief.pick_direction <id>`;runtime 将生成 `./design-systems/<project>-draft/` 六件套
4. 此后所有 Plan 产出以 draft DS 为坐标;用户 `ds promote` 后,draft 升为正式 DS

## 不得

- 不要**混**两条 direction(比如"我要 neutral-modern 但用 brutalist 的字号")— 让用户先选一条再局部调
- 不要在没有 direction 也没有 DS 的情况下硬出 Plan — 直接报错,回到 direction 选择
