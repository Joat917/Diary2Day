# Diary2Day 视觉与交互规格 v0.1

[PRD.md](./PRD.md) 定意图与理由，[DESIGN.md](./DESIGN.md) 定结构与接口，**本文件定取值**。写界面时这里查不到的项，先补本文件再回组件写那一行。功能层面的取舍集中在 [DESIGN §12](./DESIGN.md)，本文件只管界面。

---

## 1. Token

全部在 `styles/tokens.css`，浅/深两套挂在 `html[data-theme]` 上。组件里取值的写法只有一种：`var(--*)`。

### 1.1 颜色

| token | 浅 | 深 | 用在 |
|---|---|---|---|
| `--c-surface` | `#F6F7F6` | `#101211` | 页面底 |
| `--c-surface-1` | `#FFFFFF` | `#191C1B` | 卡片、今天卡片、详情底 |
| `--c-surface-2` | `#EDEFEE` | `#232725` | 凹陷处：周条底、chip 底、输入框底 |
| `--c-on-surface` | `#17181A` | `#ECEEED` | 正文 |
| `--c-on-surface-muted` | `#5F6467` | `#9BA29F` | 元信息、次级文字 |
| `--c-on-surface-faint` | `#9AA0A3` | `#6D7472` | 禁用态与装饰图形 |
| `--c-primary` | `#17695C` | `#5FC9B4` | 选中态、今日、链接、主按钮 |
| `--c-on-primary` | `#FFFFFF` | `#052B24` | 主按钮文字 |
| `--c-primary-soft` | `#E7F0ED` | `#1B3A34` | 选中项的行底 |
| `--c-accent` | `#17695C` | `#5FC9B4` | 今日环、置顶图钉（与 primary 同色） |
| `--c-danger` | `#B42318` | `#E57366` | 删除、清空回收站 |
| `--c-hairline` | `#E4E7E6` | `#2A2F2D` | 分隔线与卡片描边，全 App 描边色唯一 |
| `--c-hit` | `#CDE7DF` | `#1F4A41` | 搜索命中 |
| `--c-hit-current` | `#8FCDBB` | `#2E6B5E` | 当前命中（n/N 指向的那条） |
| `--c-scrim` | `rgba(17,19,18,.45)` | `rgba(0,0,0,.65)` | 浮层遮罩、应用锁遮罩 |

三条约定：

- 对比度门槛：正文与小字 ≥4.5:1，标题与主正文 ≥7:1。改色板后跑一遍计算，结论写进 PR。
- 暗色的层级用 `--c-surface` 与 `--c-surface-1` 的亮度差 + 1px `--c-hairline` 表达；投影留给浅色模式。
- 表面只有三档 + scrim，区块分隔优先用留白（§1.3）。

### 1.2 字体与字阶

```css
--font-body: system-ui, -apple-system, "Segoe UI Variable Text", "Segoe UI",
             "PingFang SC", "Hiragino Sans GB", "Noto Sans CJK SC",
             "Microsoft YaHei UI", "Microsoft YaHei", sans-serif;
--font-mono: "SF Mono", "Cascadia Mono", "JetBrains Mono", Consolas,
             "Noto Sans Mono CJK SC", monospace;
```

**拉丁字体排在 CJK 之前**，`Microsoft YaHei` 保持在列表末段。

字阶 5 档，组件的 `font-size` 一律从中取：

| token | size / line-height | 用在哪 |
|---|---|---|
| `--t-display` | 22px / 30px，500 | 页面大标题、"我的"页的统计数字 |
| `--t-title` | 17px / 26px，500 | 卡片首行、详情首屏、对话框标题 |
| `--t-body` | 15.5px / **1.85** | 日记正文（Vditor 也用它）、列表预览 |
| `--t-small` | 13px / 1.5 | 元信息、chip、按钮文字 |
| `--t-caption` | 11.5px / 1.35，500 | 周条星期、月历日号下的打点数、n/N 计数 |

- 正文行长上限 `--measure: 34em`（约 34 个汉字），宽容器里居中，左右留白承担呼吸感。
- 字重取 400 / 500 / 600（600 只给数字与今日）。
- 中文段落左对齐；`letter-spacing` 取默认值。
- PRD §7 的"首行缩进"开关实现为 `text-indent: 2em`，作用范围是 `p`；标题、列表项、代码块齐头。

### 1.3 间距、圆角、层级

- 间距标度取 4 的倍数：`--s-1…--s-8 = 4 / 8 / 12 / 16 / 20 / 24 / 32 / 48`。
- 固定节奏：卡片内边距 16（窄）/ 20（宽）；列表项纵向 12；区块之间 24；同一行 chip 之间 8；图标与文字之间 6（16px 图标专用，属 §1.4 例外清单之外的唯一半步）。
- 圆角：`--r-sm 6`（chip、小按钮）/ `--r-md 10`（卡片、输入框、缩略图）/ `--r-lg 14`（浮层、对话框）/ `--r-pill 999`（今日环、计数徽标）。同一层元素用同一档。
- 投影两档，只在浅色模式下生效：
  `--e-1: 0 1px 2px rgba(17,19,18,.04), 0 1px 3px rgba(17,19,18,.06)`（轻微）
  `--e-2: 0 10px 30px rgba(17,19,18,.16)`（月历浮层、对话框、浮动按钮）
- 触控目标最小 **44×44**。视觉尺寸可以更小（月历格子 36×36，用 `::before` 把热区撑到 44）。

### 1.4 允许写死的东西（封闭清单）

1px 发丝线、`env(safe-area-inset-*)`、Vditor 内部尺寸、SVG `stroke-width`、`1em`/`2em` 这类与字号联动的相对值。这份清单之外的字面值由 `check-style.mjs` 拦截（§7）。

---

## 2. 骨架尺寸

**窄屏（<600）** 自上而下：安全区 → 顶栏 52 → 周条 76 → 今天卡片 → 当日列表 → 底部导航 56 + 安全区。
**宽屏（≥960）三栏**：`240px | minmax(320px, 1fr) | minmax(420px, 1fr)`，栏间 1px `--c-hairline`；左栏可折叠成 56px 图标条，动画 180ms。600–960 之间用两栏（列表 + 详情覆盖层）。

- 键盘弹起时今天卡片贴住键盘：监听 `visualViewport` 的 `resize`/`scroll`，把偏移写进一个 CSS 变量，**一次 rAF 内完成**。视口高度一律取 `100dvh`。
- 宽屏下详情栏与列表各自是独立滚动容器。换条目时详情栏 `scrollTop` 归零，`keep-alive` 返回列表时恢复旧值（PRD §2.1 的"不丢上下文"落在这两个动作上）。

---

## 3. 七个高风险界面

### 3.1 WeekStrip（首页常驻周条）

7 格等宽（`(100% - 2×12) / 7`），每格三行：`--t-caption` 星期缩写 → `--t-title` 日号 → 打点行（高 6）。

- 打点编码：1 条内容 = 1 个 4px 圆点（`--c-primary`）；2–3 条 = 3 个点；>3 条 = 2 个点 + 一个数字。点的大小恒定。
- 状态叠加靠形状，颜色只是加强：今日 = 日号外 `1.5px --c-accent` 环；选中 = `--c-primary` 实心 pill 底 + `--c-on-primary` 字；今日且选中 = 实心 pill + 环。这是本项目唯一的无障碍硬要求。
- 横滑换周，跟手；松手按位移过半吸附，180ms。后半周照常可选（PRD §2.2 的补记）。
- 左右两端各留 12px 渐变遮罩暗示可滑。周跨月时右上角浮一个 `--t-caption` 的月份标签，只在该周跨月时出现。

### 3.2 MiniCalendar / MonthSheet

同一份 `MiniCalendar.vue`，桌面嵌左栏（格子 32），移动在浮层里放大（格子 44）。日历自己实现（[DESIGN §8](./DESIGN.md)）。

- 网格固定 7×6，切月时高度恒定。
- 邻月补白：字色 30% 透明度、无打点、热区只用于显示。切月手段是顶部 ←/→ 与横滑。
- 周起始日读设置（PRD §2.7），表头随之整体平移。表头与格子同宽，容器宽度取 `fr` 或百分比。
- 键盘（桌面）：↑↓←→ 走天，PgUp/PgDn 走周，Home/End 走周首周末，Enter 选中，Esc 关闭浮层并把焦点还给触发它的按钮。
- 浮层：从顶部下拉展开，`--r-lg` + `--e-2`，240ms、位移 24px、配合淡入，背后 `--c-scrim`。

### 3.3 TodayCard

- 空态：高 44，`--c-surface-1` 底 + 1px `--c-hairline`，placeholder 写"此刻在想什么…"这类具体的话。
- 有内容：1→6 行自动增高，超过 6 行内部滚动；右下角浮出两个 24px 图标（图片、展开）；左下角 `--t-caption` 显示字数与保存态。
- 保存态用点的形状与颜色表达：`dirty` = 4px `--c-on-surface-faint` 空心点，1.6s 呼吸；`saving` = 同尺寸实心点；`clean` = "已保存" 停 2s 后淡出；`error` = `--c-danger` 的"未保存 · 重试"。
- 卡片自带标题栏与日期，靠所在页面的上下文说明。输入区常驻，一屏之内除打字外零必需决策。

### 3.4 JournalCard

两种密度共用一套元素：**叠放态**（首页当日，`--c-surface-1` 卡）与 **结果态**（搜索/发现，靠 hairline 分隔）。

- 布局：首行 `--t-title` 两行截断 → 预览 `--t-body` 两行截断（`preview` 字段，[DESIGN §3](./DESIGN.md) 的列表摘要）→ 底行 meta。右侧缩略图 56×56、`--r-md`、`object-fit: cover`，取 `ddasset://t/{cover_asset}`（DESIGN §3.1）。
- meta 行只渲染有值的字段：时间（`--t-caption`）、mood 图标 16px、`n` 图角标、置顶图钉。全空时整行撤走，12px 还给内容（PRD §2.3）。
- 分隔方式二选一：叠放态用 `--e-1` + 纯白底；结果态用 hairline + 页面底。描边与阴影同一张卡只出现一种。
- 封面缺席时文字区铺满整行。
- 图片加载失败或已被清理：缩略图位置显示 `--c-surface-2` + 一个 20px 图标，替代浏览器默认的裂图标记。

### 3.5 DetailView：读与写一套 UI

触发方式按 PRD §2.3 定：窄屏进详情 = 只读，宽屏进详情 = 编辑态。

- **窄屏**：三个入口共用一个状态机——右下角浮动按钮"编辑"、双击正文、`E` 键。
- **宽屏**：常驻第三栏直接进编辑态，读写切换这个动作位留给"看源码"（DESIGN §5 的 `renderSource`）。
- 切换动画：容器 `opacity` 120ms 淡出 → 卸载当前渲染器、挂载另一个 → 120ms 淡入，同时按第一行可见文字的 `getBoundingClientRect` 还原滚动位置。同一时刻容器里只有一份渲染结果（Vditor 实例配额见 DESIGN §5）。
- 只读态字号 = `--t-body` + `--measure`；标题层级由 §4 的覆盖压到 `--t-title` / `--t-body` 两档。

### 3.6 FindBar

一条 40px 的条，全局搜索与文内查找共用（PRD §2.5）：输入框 + `n/N` 计数 + ↑↓ + ✕。窄屏贴在顶栏下方，宽屏浮在详情栏右上角。

- `partial=true`（短查询走有界 LIKE，DESIGN §6.3）时在条内右侧显示 `--t-caption` 的"仅前 200 条"。
- 命中数 ≥2 时显示 `n/N`；单命中只显示 ↑↓ 的当前态；空结果显示"无匹配"。
- 详情内高亮用 `::highlight(dd-hit)`（DESIGN §6.3），背景 `--c-hit`、当前 `--c-hit-current`。高亮只改背景，行高与字距保持原样。
- **Highlight API 缺失时的退化态**（V-5）：用码点区间构一个 DOM `Range`，取它的 `getBoundingClientRect()`，在详情容器上盖一个 `pointer-events: none` 的绝对定位小块（高 = 行高，背景 `--c-hit-current`，120ms 淡入、1.2s 后淡出，每跳一次重新定位）。退化态只标当前命中，全量高亮留给支持的环境。
- 键盘：`/` 或 `Ctrl/⌘+F` 聚焦，`Enter`/`Shift+Enter` 跳，`Esc` 关闭并清空高亮集合。

### 3.7 StateBlock（空 / 加载 / 错误）

一个组件三种形态，全 App 只此一份。

- **加载**：首屏列表 = 3 张骨架卡（`--c-surface-2` 块，shimmer 1.2s）；分页追加 = 底部一条 2px `--c-primary-soft` 进度线。打点与统计这类毫秒级查询直接渲染结果。
- **空**：图标 32（`--c-on-surface-faint`）+ 一句话 + 一条出路按钮（PRD §2.5）。文案区分两种真状态："今天还没有记录 · 写一条"与"没有结果 · 放宽筛选"。
- **错误**：inline 卡片，`--c-danger` 标题 + 一句人话 + 重试按钮。错误呈现一律走这里。

---

## 4. Vditor 覆盖规格

改动集中在 5 个选择器，全部写在 `styles/vditor-override.css`，用 `:where()` 保持零特异度（DESIGN §7 第 5 条）：

| 选择器 | 改成 |
|---|---|
| `:where(.vditor-reset)` | `--font-body`、`--t-body`、`--measure` 居中、段距 `--s-4`；`color: var(--c-on-surface)` |
| `:where(.vditor-reset h1 … h6)` | 折叠到 `--t-title` / `--t-body`+500 两档，上间距 `--s-6` |
| `:where(.vditor-ir__marker)` | `color: var(--c-on-surface-faint)`；当前行的 marker 提到 muted |
| `:where(.vditor-toolbar)` | 高 40、底 `--c-surface-2`、下边框去掉、图标 20px `--c-on-surface-muted` |
| `:where(.vditor-reset pre > code)` | `--font-mono` 13px、底 `--c-surface-2`、`--r-md`；开行号时（PRD §7）行号列 `--t-caption` |

图片圆角 `--r-md`、`max-width: 100%`；任务列表勾选框 18px。Vditor 自带的深色一律换成 §1.1 的 token，暗色保持一套值。

---

## 5. 交互态与动效

- **hover**：底色换 `--c-surface-2`，或叠 2% 黑遮罩，仅此两项。触屏端走 `@media (hover: hover)`，窄屏无 hover 态。
- **pressed**：浮动按钮缩放到 98%，其余元素靠底色加深。
- **focus**：`:focus-visible` 一律 `outline: 2px solid var(--c-primary); outline-offset: 2px`。鼠标点击走 `:focus-visible` 的默认行为，无环。
- **禁用**：38% 透明度 + `cursor: default`，字色保持原样。
- 动效三档时长：`--m-1 120ms`（淡入淡出、状态切换）、`--m-2 180ms`（吸附、折叠、左栏宽度）、`--m-3 240ms`（浮层进出）。缓动一条：`--ease: cubic-bezier(.2,0,0,1)`。位移量取 `--s-4` 或 `--s-6`。三档之外只留给周条吸附与浮层进出这类有信息量的动作。
- `@media (prefers-reduced-motion: reduce)` 下三档时长归 0，打点呼吸动画停在静态态。

---

## 6. 主题、语言与 RTL

- 主题三态（系统/浅/深）由 `session` 里一个设置驱动，切换发生在 `html[data-theme]` 一处，组件只消费 token。深色与否的判断集中在这一个 composable。
- 选"跟随系统"时，`matchMedia('(prefers-color-scheme)')` 的变化直接改 `data-theme`。
- 需要反色的图片走 CSS filter。
- 文案长度上限：英文按钮 ≤ 3 词、中文 ≤ 6 字；更长的动作改用图标 + tooltip。i18n 的实现见 DESIGN §5。
- RTL：`dir="rtl"` 一次翻转 + 全部 logical properties（DESIGN §5）。镜像清单 —— ←/→ 图标、周条与月历的排列方向、`text-align` 起点。保持原样 —— 表情图标、数字、日期、`ddasset://` URL、代码块。月历在 RTL 下的周起始与补白位置在 V-2 那轮一起验。

---

## 7. 自查与机器检查

每完成一屏做四件事（10 分钟内）：浅色/深色各截一张图并排看；4 种断点各看一次；切纯英文与纯中文各看一次；灌 1000 条真数据滚一遍首页。

`check-style.mjs`（DESIGN §8）在 CI 里拦三类写法：

1. 组件里出现十六进制色值 —— 色值只来自 §1.1 的 token。
2. `font-size` 落在 §1.2 的五档之外。
3. `!important` 出现在 `vditor-override.css` 之外 —— Vditor 的覆盖只集中在 §4 那一个文件。

一条判断题：某一屏同时出现三种以上分区手段（阴影、描边、底色、间距、标题）时，留一种，其余换成留白。
