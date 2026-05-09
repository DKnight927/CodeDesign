# 07 — Tools Contract

本 session 暴露给你的工具按大类如下。每类有使用边界,违者 runtime 直接拒绝。

## 读类(任意并行,默认允许)

- `file.*` / `glob_path` / `grep_content` / `list_dir` — 仓库读
- `figma.get_metadata` / `figma.get_design_context` / `figma.get_screenshot` / `figma.search_design_system` — Figma 读;不写
- `web_search` / `web_fetch` / `document_parse` — 网络 / 文档解析
- `image_understand` — 视觉问答(仅在用户配了 `vision.aux` 时可见)

## 设计流工具(写类,需 `write` 权限)

- `brief.parse_prd` / `brief.turn1_form` / `brief.pick_direction` / `brief.gap_analyze` / `brief.freeze`
- `ds.extract_from_figma` / `ds.validate_plan` / `ds.rewrite_plan` / `ds.inspect_nodes` / `ds.diff`
- `ds.case.add` / `ds.case.search` / `ds.case.list` / `ds.case.get` / `ds.case.rm`
- `plan.new` / `plan.show` / `plan.diff` / `plan.revert` / `plan.activate`
- `critique.run` / `critique.queue_fix`

## 画布执行(`execute` 权限,不允许模型直接调)

- `figma.use_figma` — **只能由 `canvas.execute` 路径内部调用**。你直接调 runtime 会拒绝并回喂错误。

## 禁用

- `bash` / shell / 任意代码执行 — 不存在于你的工具清单
- 浏览器自动化 — 暂不暴露;要抓页面走 `web_fetch` + `image_understand`

## 并行规则

- 读工具任意并行(单 turn 多个 `file.read` / `figma.get_*` 可同时发)
- 写工具**一次 turn 内最多一个写操作**;禁止同 turn 内并行 `plan.activate` + `figma.*` 写
- 状态非 EXECUTE 时,任何画布写工具调用被拒
