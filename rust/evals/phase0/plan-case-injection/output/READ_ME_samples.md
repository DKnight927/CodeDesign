# S0.3 样本可读版

把三份 Plan JSON 展开成大纲,方便打 D2 (style 是否贴 direction) / D3 (是否违反 doNot) 分。

## sample-a — direction: neutral-modern

**PRD 摘要:** SaaS 协作平台登录与主页。登录页:邮箱 + 密码 + SSO 入口,忘记密码链接,登录失败须明确错误。主页:左侧导航(项目、团队、个人设置),主区展示最近项目卡片、任务待办、团队动态。目标用户是企业团队管理员,强调清晰、专业、不花哨,首…

**intent/step:** create_screens / skeleton  
**frames:** 2  
**要新造的组件:** nav.item  

### Frame: 登录页
_layout: `{"type": "centered", "verticalAlign": "center", "horizontalAlign": "center"}`_

- **surface**   _fill=color.bg.base_
- **card**   _comp=card.default_
  - **text** "登录您的账户"  _style=typo.title.lg · fill=color.text.primary_
  - **input** "工作邮箱"  _comp=input.text_
  - **input** "密码"  _comp=input.text_
  - **text** "邮箱或密码错误，请检查后重试。"  _style=typo.caption.sm · fill=color.feedback.error_
  - **button** "登录"  _comp=button.primary_
  - **divider**
  - **button** "通过 Google 登录"  _comp=button.secondary_
  - **button** "通过 Microsoft 登录"  _comp=button.secondary_
  - **text** "忘记密码？"  _style=typo.caption.sm · fill=color.accent.primary_

### Frame: 主页
_layout: `{"type": "sidebar-main"}`_

- **surface**   _fill=color.bg.elevated_
  - **text** "协作平台"  _style=typo.title.md · fill=color.text.primary_
  - **nav-item** "项目"  _comp=nav.item · style=typo.body.md_
  - **nav-item** "团队"  _comp=nav.item · style=typo.body.md_
  - **nav-item** "个人设置"  _comp=nav.item · style=typo.body.md_
- **surface**   _fill=color.bg.base_
  - **text** "今日工作"  _style=typo.title.lg · fill=color.text.primary_
  - **h-stack**
    - **text** "待办 5"  _style=typo.caption.sm · fill=color.text.secondary_
    - **text** "进行中 2"  _style=typo.caption.sm · fill=color.text.secondary_
    - **text** "@提及 1"  _style=typo.caption.sm · fill=color.text.secondary_
  - **grid**
    - **v-stack**
      - **text** "今日待办"  _style=typo.title.md · fill=color.text.primary_
      - **list**
      - **text** "最近项目"  _style=typo.title.md · fill=color.text.primary_
      - **grid**
        - **card**   _comp=card.default_
        - **card**   _comp=card.default_
        - **card**   _comp=card.default_
    - **v-stack**
      - **text** "团队动态"  _style=typo.title.md · fill=color.text.primary_
      - **list**

**rationale (模型自述):**
> 登录页采用中央单卡片布局（参考 Notion 登录），垂直堆叠表单、SSO 按钮与辅助链接，大量留白降低认知负荷；主页采用左侧窄导航 + 右侧主内容区（参考 Linear Inbox），并以「今日工作」为页头，首行即展示今日待办、进行中与 @提及的 KPI 条（参考 Stripe Dashboard），随后紧跟任务列表，确保管理员在进入页面 5 秒内即可定位今日任务。整体严格使用灰阶 + 单一强调色，无装饰图、无多色标签、无饱和背景块，保持专业克制。

**craft_asserts:**
- 登录页错误提示必须使用 color.feedback.error，并位于密码输入框与登录按钮之间。
- 主页「今日待办」列表必须位于首屏可视区域，无需滚动即可查看。
- 左侧导航栏必须使用 color.bg.elevated，与主内容区形成明确层级。
- 所有文本字号不得小于 typo.caption.sm（14px）。

**quality_gates:**
- 登录失败时有明确错误状态反馈。
- 管理员进入主页后 5 秒内可定位今日待办任务。
- 未使用任何 indigo/violet/pink 作为装饰色。
- 未使用 emoji 作为 UI 图标。
- 全站最小字号为 14px。


---

## sample-b — direction: warm-editorial

**PRD 摘要:** 读书笔记应用。首页展示近期阅读的书(封面 + 书名 + 阅读进度)、当前正在读的书的最新笔记卡片、以及一个写新笔记的主 CTA。笔记页:顶部书信息 + 章节选择 + 正文长段落编辑 + 标签。强调阅读感、长内容友好、慢节奏。…

**intent/step:** create_screens / skeleton  
**frames:** 2  
**要新造的组件:** {"id": "book.thumb", "name": "书籍缩略图", "desc": "封面图+书名+阅读进度，用于首页网格"}, {"id": "editor.surface", "name": "长文本编辑区", "desc": "多行富文本/ markdown 编辑表面，舒适行高与暖色背景"}, {"id": "progress.bar", "name": "阅读进度条", "desc": "细线性进度，用于书籍缩略图"}  
**要新造的 token:** {"id": "layout.reading.max", "name": "阅读最大宽度", "desc": "正文栏最大宽度，保证长内容可读性"}  

### Frame: 首页
_layout: `{}`_

- **frame** "顶部导航"
  - **text** "应用标题"  _fill=color.text.primary_
  - **instance** "写新笔记"  _comp=button.primary_
- **frame** "正在阅读"
  - **text** "区块标题"  _fill=color.text.primary_
  - **instance** "最新笔记卡片"  _comp=card.article_
- **frame** "最近阅读"
  - **text** "区块标题"  _fill=color.text.primary_
  - **frame** "书籍网格"
    - **instance** "书籍1"  _comp=book.thumb_
    - **instance** "书籍2"  _comp=book.thumb_
    - **instance** "书籍3"  _comp=book.thumb_
    - **instance** "书籍4"  _comp=book.thumb_

### Frame: 笔记页
_layout: `{}`_

- **frame** "顶部书信息"
  - **instance** "返回"  _comp=button.quiet_
  - **text** "书名"  _fill=color.text.primary_
  - **text** "作者/进度"  _fill=color.text.secondary_
- **frame** "章节选择"
  - **instance** "章节1"  _comp=button.quiet_
  - **instance** "章节2"  _comp=button.quiet_
  - **instance** "章节3"  _comp=button.quiet_
- **frame** "编辑区"
  - **instance** "笔记标题"  _comp=input.text_
  - **instance** "正文编辑"  _comp=editor.surface_
- **frame** "标签区"
  - **instance** "标签1"  _comp=tag.inline_
  - **instance** "标签2"  _comp=tag.inline_
- **frame** "底部操作"
  - **instance** "保存"  _comp=button.primary_

**rationale (模型自述):**
> 构建双屏骨架：首页采用书封网格+笔记卡片+主 CTA 的暖色阅读信息流；笔记页采用顶部书信息+章节选择+中央长文本编辑+标签的沉浸编辑器布局。遵循 warm-editorial 方向，以 serif 标题配 sans 正文，保持克制用色、无侧边栏装饰、无高对比色块，优先长内容可读性。

**craft_asserts:**
- 编辑器采用中央单列布局，无侧边栏装饰，工具条保持单色极简
- 首页书封卡片使用柔和视觉处理，避免硬锐角
- 正文层级严格区分：标题使用 serif，正文使用 sans 16px+
- 每次仅出现一种强调色装饰，避免多色并置

**quality_gates:**
- 所有正文文字不小于 16px，行高舒适
- 编辑器页面无多层级侧边栏、无高对比色块
- 标签使用 inline 形式呈现，与正文保持呼吸感
- 书封网格信息密度适中，避免拥挤小字


---

## sample-c — direction: tech-utility

**PRD 摘要:** SRE 实时指标看板。顶部 KPI 行(QPS、P99、错误率、在线实例数);中部 4 宫格图表(时间序列折线);下部告警列表(时间、等级、服务、摘要、操作)。需要高信息密度,深色优先,关键告警突出但不喧宾夺主。…

**intent/step:** create_screens / skeleton  
**frames:** 1  
**要新造的组件:** {"componentId": "chart.timeseries", "purpose": "时间序列折线面板，承载单条或多条折线，仅供数据可视化区域使用"}  

### Frame: None
_layout: `{}`_

- **?**
- **?**
- **?**

**rationale (模型自述):**
> SRE 实时看板骨架：采用高信息密度的深色布局，顶部横向 KPI 行使用 metric.card + mono 数值；中部 2×2 宫格使用折线面板承载数据序列，严格隔离 data colors；下部告警列表采用 table.row 结构，等级通过 tag.status 小色点与反馈色表达，避免喧宾夺主。整体遵循 tech-utility 方向，融合 Grafana 紧凑排版、Datadog 表格告警与 Linear 克制用色的参考特征。

**craft_asserts:**
- {"assert": "information-density", "level": "high", "scope": "global"}
- {"assert": "no-decorative-gradients", "level": "strict"}
- {"assert": "data-color-isolation", "level": "strict", "scope": ["r-mid-charts"]}
- {"assert": "alert-restraint", "level": "strict", "note": "关键告警使用小色点+灰阶文字传达，禁止大面积色块或 emoji"}

**quality_gates:**
- {"gate": "typography-contrast", "mustPass": true}
- {"gate": "mono-usage-on-metrics", "rule": "KPI 数值与图表轴标签须使用 typo.mono.sm"}
- {"gate": "data-color-hygiene", "rule": "data colors 不得出现在按钮、背景、文本等非数据可视区域"}


---
