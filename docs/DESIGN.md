# Diary2Day 代码设计 v0.1

对应需求：[PRD.md](./PRD.md)。已定：胖 Rust 内核、Vditor IR 模式、桌面 + 移动同期。

分工：**本文件管结构与接口，[UI.md](./UI.md) 管长相。** 冲突时，交互意图以 PRD 为准，视觉以 UI.md 为准，接口以本文件为准。取舍的理由写在 PRD，本文件只写结论。

否决掉的选项集中在 §12。配套现状：`src-tauri` 是 Tauri v2 空脚手架，`panic = "abort"`、`csp: null`、只有 `tauri-plugin-opener`，§9 列出要改的配置。

---

## 1. 一句话结构

Rust 持有全部真身：schema、查询、天归属计算、字数、附件落盘、导出导入、备份恢复、回收站、应用锁。TS 负责视图、编辑器包装、去抖保存的状态机。**SQL 只存在于 `db/`，`invoke("…")` 字符串只存在于 `ipc/client.ts`。**

```
src-tauri/src/
  lib.rs            装配：建连接、跑迁移、注册协议与命令、窗口事件 → 锁
  state.rs          AppState { db: Mutex<Connection>, paths: AppPaths, lock: LockState, ops: Ops }
  error.rs          AppError(code, msg, detail) + Into<serde_json::Value>，前端按 code 分支
  db/
    conn.rs         open + PRAGMA + busy_timeout + with_db/with_db_ro
    migrate.rs      版本化迁移；迁移前自动快照；失败回滚
    schema/*.sql    0001_init.sql、0002_...（include_str!，按文件名序执行）
    journal.rs      条目 upsert / 列表 / 详情 / 软删 / 恢复 / purge / 置顶 / 敏感
    tag.rs          标签 CRUD + 合并 + 引用计数
    search.rs       FTS 与短查询兜底，策略选择集中在这一个函数里
    daymark.rs      打点增量维护 + 全量重算
    settings.rs     SettingKey 枚举声明类型/默认/校验
    stats.rs        聚合统计（连续天数读 day_mark）
  domain/           纯函数，零 IO
    day_key.rs      offset 解析、落在哪天、周条/整月区间、夏令时
    title.rs        首行派生标题、100/300 截断
    words.rs        CJK 按字 / 拉丁按词 / 代码块与表格排除
    name_key.rs     标签名折叠（casefold + 全角折叠 + trim）
    asset_name.rs   随机名生成 + 扩展名白名单 + 协议 URL ↔ 磁盘路径解析（§3.1 的校验实现）
    md_doc.rs       导出用 md 文件的 front-matter 读写 + 附件 URL 双向映射（导出与导入共用）
    trash.rs        保留期判定（时钟可注入）
  fs/
    paths.rs        AppPaths { data_dir, db_path, assets_dir, cache_dir }
    assets.rs       原子写入、按 hash 命中已有、缩略图产出、§3.1 文件协议的读实现
    export.rs       导出人可读目录（临时目录 → 原子改名 → 进度事件 → 可取消）
    backup.rs       VACUUM INTO 副本 + assets + manifest → zip
    restore.rs      校验 manifest/版本 → 快照 → 替换 → 重开连接
  commands/         #[tauri::command] 薄壳：解参 → 调上面模块 → 映射 AppError
    journal.rs tag.rs search.rs settings.rs media.rs data.rs stats.rs app.rs
  events.rs         事件名常量 + payload 类型
  lock.rs           应用锁（口令 hash、失焦计时、锁态广播）
  ops.rs            长任务登记表：op_id → AtomicBool 取消位 + 进度 emit

src/
  ipc/
    client.ts       invoke 的唯一出口：typed 函数 + 错误反序列化
    asset-url.ts    `ddasset://{kind}/{name}` 的唯一拼接处（§3.1）
    bindings/       ts-rs 生成的 DTO，进版本库，由 CI 校验与生成物一致
  views/            TodayView DiscoverView MeView DetailView
  components/       TodayCard WeekStrip MiniCalendar MonthSheet JournalCard
                    EditorPane MetaChips FindBar FilterBar AssetViewer
                    Icon StateBlock（空态/加载/错误共用一个，UI.md §3.7）
  composables/      useAutosave useBreakpoint useDayMarks useSettings useFind useCollection
  stores/           session.ts（锁态 + 设置缓存），全局仅此一个 store
  i18n/             messages/en.ts（键的唯一真相）+ messages/zh-CN.ts + t()（约 100 行）
  styles/           tokens.css（全部取值，UI.md §1）+ vditor-override.css（覆盖 Vditor 的唯一入口）
```

`domain/` 只依赖 `std`/`serde`/`chrono`，`db/` 与 `fs/` 的调用方向只朝下。

---

## 2. Schema

```sql
CREATE TABLE schema_version (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL);

CREATE TABLE journal (
  id           TEXT    NOT NULL PRIMARY KEY,        -- UUID v7
  content      TEXT    NOT NULL DEFAULT '',
  format       TEXT    NOT NULL DEFAULT 'markdown'  CHECK (format IN ('markdown','plain')),
  title        TEXT,                                -- NULL = 列表用首行派生
  mood         TEXT,                                -- NULL 或 17 个 code 之一
  recorded_at  TEXT    NOT NULL,                    -- UTC ISO8601
  recorded_off INTEGER NOT NULL DEFAULT 0,          -- 记录时的本地偏移，分钟
  day_key      TEXT    NOT NULL,                    -- YYYY-MM-DD
  created_at   TEXT    NOT NULL,
  updated_at   TEXT    NOT NULL,
  deleted_at   TEXT,                                -- NULL = 存活，否则在回收站
  pinned       INTEGER NOT NULL DEFAULT 0,
  sensitive    INTEGER NOT NULL DEFAULT 0,
  words        INTEGER NOT NULL DEFAULT 0,
  asset_n      INTEGER NOT NULL DEFAULT 0,          -- 派生：save 事务内扫正文的 ddasset://
  cover_asset  TEXT,                                -- 派生：首张 image 的 name
  revision     INTEGER NOT NULL DEFAULT 1
);
-- 保留隐式 rowid：FTS5 external-content 靠它对齐主表
CREATE INDEX idx_j_day      ON journal(day_key, recorded_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_j_recorded ON journal(recorded_at DESC)          WHERE deleted_at IS NULL;
CREATE INDEX idx_j_mood     ON journal(mood, recorded_at DESC)    WHERE deleted_at IS NULL AND mood IS NOT NULL;
CREATE INDEX idx_j_pinned   ON journal(recorded_at DESC)          WHERE deleted_at IS NULL AND pinned = 1;

CREATE TABLE tag (
  id TEXT PRIMARY KEY, name TEXT NOT NULL,
  name_key TEXT NOT NULL UNIQUE,                    -- 折叠后的列，标准 collation
  created_at TEXT NOT NULL);
CREATE TABLE journal_tag (
  journal_id TEXT NOT NULL REFERENCES journal(id) ON DELETE CASCADE,
  tag_id     TEXT NOT NULL REFERENCES tag(id)     ON DELETE CASCADE,
  PRIMARY KEY (journal_id, tag_id));
CREATE INDEX idx_jt_tag ON journal_tag(tag_id);

CREATE TABLE day_mark (                             -- 打点与连续天数的唯一来源
  day_key TEXT PRIMARY KEY, n INTEGER NOT NULL DEFAULT 0, words INTEGER NOT NULL DEFAULT 0);

CREATE TABLE asset (                                -- 与 journal 之间零关联，见 §3.1
  name TEXT PRIMARY KEY,                            -- 随机名+扩展名 = 磁盘文件名
  kind TEXT NOT NULL CHECK (kind IN ('image','audio','video')),
  ext TEXT NOT NULL, size INTEGER NOT NULL,
  width INTEGER, height INTEGER, duration_ms INTEGER,
  hash BLOB, orig_name TEXT,                         -- hash 只在导入去重时用
  created_at TEXT NOT NULL);

CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);   -- janitor_last_run / install_id

CREATE VIRTUAL TABLE journal_fts USING fts5(
  title, content, content='journal', content_rowid='rowid',
  tokenize='trigram case_sensitive 0');
CREATE TRIGGER journal_ai AFTER INSERT ON journal BEGIN
  INSERT INTO journal_fts(rowid,title,content) VALUES (new.rowid, coalesce(new.title,''), new.content); END;
CREATE TRIGGER journal_ad AFTER DELETE ON journal BEGIN
  INSERT INTO journal_fts(journal_fts,rowid,title,content) VALUES ('delete',old.rowid, coalesce(old.title,''), old.content); END;
CREATE TRIGGER journal_au AFTER UPDATE OF title, content ON journal BEGIN
  INSERT INTO journal_fts(journal_fts,rowid,title,content) VALUES ('delete',old.rowid, coalesce(old.title,''), old.content);
  INSERT INTO journal_fts(rowid,title,content) VALUES (new.rowid, coalesce(new.title,''), new.content); END;
```

连接建立后一次性：`journal_mode=WAL`、`foreign_keys=ON`、`busy_timeout=4000`、`synchronous=NORMAL`。`synchronous=NORMAL` 下断电可能丢最后一个事务，README 如实写一句。

### 2.1 时间的三列用法

**排序与区间用 `recorded_at`，日历归属用 `day_key`，显示用 `recorded_at + recorded_off` 重建。** `day_key` 由 Rust 在 upsert 事务内算出（PRD §2.2）；前端上传 offset 与墙上时间，`day_key` 由 Rust 单向下发。

### 2.2 day_mark 的维护

save / delete / purge 在同一事务内增量更新（`UPDATE ... SET n = n + ?, words = words + ?`）。打点与连续天数一律读这张表。启动自检跑一次全量比对，差异 > 0 则静默全量重算并记日志。全量重算同时暴露给 `data_rebuild(what='day_mark')`（§3）。

---

## 3. IPC 面

Rust 命令全部 `snake_case`，TS 侧 `client.ts` 一一映射；分页统一 `limit/cursor`，cursor 是 base64 的 `recorded_at|id`。**命令只搬文字**——正文、标题、设置值、元数据；附件字节一律走 §3.1 的文件协议。

| 命令 | 入参 → 返回 | 备注 |
|---|---|---|
| `journal_list` | `day_key? range? tag_id? mood? include_trash limit cursor` → `JournalPage` | 摘要字段：id / 派生 title / `preview`(300 字符) / mood / day_key / words / `asset_n` / `cover_asset`；图片 URL 由卡片拼（§3.1） |
| `journal_get` | `id` → `Journal`（全量 + tag 对象） | |
| `journal_save` | `id? content format title mood tag_ids recorded_at recorded_off pinned revision` → `Journal` | 唯一写入口。同事务：`day_key`、`words`、`asset_n`/`cover_asset`、派生 title、diff `journal_tag`、增量 `day_mark`、FTS 触发器、`revision+1` |
| `journal_delete` | `id` → `day_key` | 软删；同事务更新 `day_mark` |
| `journal_restore` / `journal_purge` | `id` / 无参 | purge 由 janitor 调，UI 也提供"清空回收站" |
| `journal_trash_list` | `limit cursor` → `JournalPage` | |
| `journal_set_pinned` / `journal_set_sensitive` | `id value` → unit | 入参只有 id 与值 |
| `day_marks` | `from_key to_key` → `DayMark[]` | 周条与整月共用；切月查月区间 |
| `search_query` | `text filters range limit cursor` → `SearchPage{items, partial}` | `partial=true` 时 UI 显示"结果可能不完整"（§6.3） |
| `search_locate` | `id text` → `{total, ranges[]}` | 详情内查找（§6.3）。偏移单位是 **Unicode 码点** |
| `tag_list` / `tag_create` / `tag_rename` / `tag_delete` / `tag_merge` | 常规 | create/rename 按 `name_key` 查重，冲突返回 `code=tag_exists` |
| `settings_get_all` | → `Settings` | 一次拿全量，前端启动时缓存（§5） |
| `settings_patch` | `Partial<Settings>` → `Settings` | 服务端校验后回全量，前端覆盖缓存 |
| `stats_overview` | → `Stats` | 全走聚合 + `day_mark` |
| `media_import` | `paths[]` → `Asset[]` | 复制/移动到 `assets/`，随机名，抽尺寸或时长，产出缩略图（§3.1）。返回元数据 |
| `data_export_dir` | `target_dir` → `op_id` | 人可读 md 目录（PRD §5.2） |
| `data_backup` | `target?` → `op_id` | |
| `data_restore` | `path strategy(merge｜replace)` → `op_id` | |
| `data_import_md` | `dir zip?` → `op_id` | 与 export 对称（§6.4） |
| `data_info` | → `DataInfo{db_path, data_dir, sizes{...}, counts{...}}` | 关于页与"打开数据目录"用；移动端 `db_path` 返回 `null` |
| `data_rebuild` | `what(day_mark｜fts｜tags｜assets)` → `Report` | 诊断页入口 |
| `op_cancel` | `op_id` → `bool` | |
| `app_lock_status` / `app_lock_set` / `app_lock_clear` / `app_lock_unlock` | `code?` → `LockState` | UI 门（PRD §5.4） |
| `app_open_data_dir` | → unit | 桌面 |

事件（`events.rs` 里定义常量，前端只 `listen(EVENT.dataChanged, …)`）：

| 事件 | payload | 用途 |
|---|---|---|
| `dd://data-changed` | `{kind:'journal'｜'tag'｜'all', id?, day_key?}` | 列表与打点的局部刷新依据（§5 的失效规则） |
| `dd://op-progress` | `{op_id, phase, done, total}` | 导出/备份/恢复的进度条 |
| `dd://op-done` / `dd://op-failed` | `{op_id, result｜error}` | 收尾与错误呈现 |
| `dd://lock-state` | `{locked, reason}` | Rust 侧失焦计时驱动（§6.5） |
| `dd://settings-changed` | `{keys[]}` | 主题与语言即时生效 |

类型同步：DTO 用 `ts-rs` derive，`cargo run --bin export-types` 生成到 `src/ipc/bindings/`，生成物进版本库并挂进 CI（`cargo test` 后校验 diff 为空）。

### 3.1 附件走文件协议，字节走 WebView 直读

图片和音视频由 WebView 自己发请求取，Rust 只经手元数据。

**机制**：`lib.rs` 里 `register_asynchronous_uri_scheme_protocol("ddasset", …)`，实现放在 `fs/assets.rs`，职责单一——把 `ddasset://{kind}/{name}` 映射到 `assets_dir` 下的文件，回 `Content-Type`（`mime_guess`）、`Content-Length`、`Accept-Ranges: bytes`、`Cache-Control: immutable`，请求带 Range 就 `seek` 后回 206。它只依赖 `AppPaths` 与 `LockState`，与数据库连接互相独立；导出/备份占用只读连接期间图片照常加载。

**URL 直接存进正文**（`![](ddasset://image/k3f9a1.png)`），渲染路径对 URL 零介入，正文与源文本自始至终同一份字符串。前缀替换只发生在导出与导入两处（§6.4）。

**协议校验三条**：

- `name` 匹配 `[A-Za-z0-9_-]+\.[a-z0-9]{1,5}`（单扩展名），拼出的路径 resolve 后仍落在 `assets_dir` 内。
- `{kind}` 取 `image` / `audio` / `video` / `t`（指向 `.thumbs/`）四者之一，其余返回 404。
- **锁态返回 404**。§6.5 的"遮罩期间数据未发送"同时覆盖命令与协议。

**缩略图**：`media_import` 对图片产出长边 ≤512px 的 `.webp`，文件名取原 name 的 stem（`k3f9a1.png` → `.thumbs/k3f9a1.webp`），卡片请求 `ddasset://t/k3f9a1.webp`，换扩展名这件事只发生在 `asset-url.ts`。详情加载原图；音视频出图标 + 时长的卡片。

**卡片用哪张图**：`asset` 与 `journal` 保持零关联（PRD §2.4）。卡片取 `journal.asset_n` 与 `journal.cover_asset` 两个派生列，在 save 事务里和 `words`、派生 title 一起从正文扫出。两者归进同一个 `data_rebuild(what='assets')`，它同时完成全表比对的孤儿文件清理：正文出现过的 name 集合与 `asset` 表求差，多余文件删除。

---

## 4. 数据访问的形态

`rusqlite` + `Mutex<Connection>` 是全部并发原语（池、ORM、repository 抽象的取舍见 §12）。

- 写连接只服务交互；导出、备份、一致性校验走第二个只读连接（`Connection::open_with_flags(Readonly)` + `PRAGMA query_only=ON`，WAL 允许并发读）。
- **临界区内只跑同步代码。** 所有数据库操作写进 `tokio::task::spawn_blocking(move || …)`，锁在闭包里进出。`db/conn.rs` 提供 `with_db(|c| …)` 与 `with_db_ro(|c| …)`，业务代码经由这两个 helper 取连接，锁的进出集中在此。

`AppError` 是带 `code` 的枚举（`tag_exists`、`journal_not_found`、`bad_revision`、`unsupported_ext`、`asset_too_large`、`restore_version_mismatch`、`backup_manifest_invalid`、`cancelled`、`io`、`sqlite`、`internal`），序列化固定为 `{code, msg, detail?}`。前端按 `code` 分支决定文案与出路。

`panic = "abort"` 下命令里的 panic 直接终止进程。内核的错误处理规则：每个失败路径返回 `Result`；索引访问一律 `get()`；`spawn_blocking` 闭包返回 `Result`；`clippy.toml` 里 `unwrap_used.behavior = "deny"`、`expect_used.behavior = "deny"`（§9）。

---

## 5. 前端形态

**路由**：`vue-router`，4 条平铺路由 `/today`、`/discover`、`/me`、`/j/:id`。分界线一句话：**换"页"走路由，叠"层"走状态**——整月视图、附件大图、文件选择器、确认框都挂在组件局部状态上。移动端返回键接路由历史（Tauri 的 `on_back_pressed` → `router.back()`）。

**桌面三栏落在同样 4 条路由上**：`/today` 在窄屏渲染【周条 + 今天卡片 + 当日列表】，在宽屏渲染同一组件树的【迷你月历｜列表｜详情】三块 `<section>`——两个分支共用一棵树、一份列表逻辑。详情栏两边都是 `/j/:id`，宽屏套在第三列（`<router-view v-slot>` + 一份 `keep-alive` 实例，滚动位置随之保留），窄屏独占整屏。**选中的那天写在 URL 上**（`/today?d=2026-11-21`）。`/me` 是 PRD §2.7 那张单页分组滚动页（profile + 统计卡 + 五组设置），`stats_overview` 与设置缓存只服务这一屏。

**状态**：全局一个 Pinia store `session`，装两样东西——锁态、`Settings` 缓存。三条规则：

- 服务端数据活在组件里：一个 20 行的 `useCollection()` 覆盖列表的 loading/分页/刷新，四个视图共用。
- 能被 UI 派生的值一律由 Rust 算好返回（连续天数、卡片标题、周条打点）。
- 跨视图失效走事件：`journal_save` 成功后 Rust 广播 `dd://data-changed`，`TodayView` 与 `DetailView` 各自判断 payload 影响自己哪一块；列表按 `day_key` 精确重查一页。

**全 App 同一时刻最多一个 Vditor 实例。** 由此定三件事：

- **今天卡片是一块自动增高的 `<textarea>`**（§6.1）。正文存的就是 markdown 源，textarea 与 Vditor 读写同一份字符串。
- 详情与整篇编辑才挂 `EditorPane`。桌面三栏里"今天卡片 + 详情编辑器"是 textarea 与 Vditor 并存，各管一个输入焦点。
- `EditorPane` 卸载时调 `vditor.destroy()`；`DetailView` 在 `onDeactivated` 里只冲刷、保留实例，滚动位置与撤销栈随实例一起留住（PRD §2.1）。

**`EditorPane.vue`** 对外暴露 `modelValue`、`dirty`、`insertText(text)`、`renderSource`（"看源码"切换）、`revealRange(range)`（把 §6.3 的码点区间滚进视野并落光标）。Vditor 的 DOM 归 Vditor：编辑器内容单向流出，外部只在初始挂载与显式"看源码"切换两个时机写入，联动一律走事件回调。

**标签与心情**（PRD §2.6）：一个弹层、两个入口。状态属于 `DetailView` 的 `MetaChips` 一份；编辑器工具栏的 `label` 按钮把同一个弹层定位到光标处唤起。

**`useAutosave`** 状态机：

```
clean ──input──▶ dirty ──1s──▶ saving ──ok──▶ clean
                                   │
                                   └─fail─▶ error(保留 dirty 内容 + 重试按钮)
触发冲刷：切换条目 / 路由离开 / 窗口失焦 / 切后台 / beforeunload
冲刷路径一律 await 到落库完成再放行导航
```

去抖 1s（PRD §7）是 `useAutosave.ts` 里的具名常量，settings 清单里没有这一项（PRD §2.7）。

`journal_save` 带 `revision`，服务端比对失败返回 `bad_revision`，处理动作一个：**丢弃本次，以最后一次冲刷为准**。

**样式**：取值全在 [UI.md §1](./UI.md)，本文件只管三条写法——色值与间距一律取 token（例外清单 UI.md §1.4）；一律 `logical properties`（`margin-inline-start`），RTL 靠 `dir="rtl"` 翻转，一套布局覆盖两个方向；Vditor 的覆盖集中在 `styles/vditor-override.css`，`!important` 只出现在这个文件里。

**文案**：`i18n/messages/en.ts` 是键的唯一真相（扁平 key + `{n}` 占位），`zh-CN.ts` 用 `satisfies typeof en` 声明，缺键直接编译失败；`t()` 加 `Intl.PluralRules` 约 100 行。语言切换改根节点一个 `locale` ref，文案由计算属性重算，视图树保持挂载。

**时间显示归前端**：Rust 只回 `recorded_at` / `recorded_off` / `day_key` 三个原始值，格式化全部在前端做，一律 `Intl.DateTimeFormat`。PRD §7 那三个预设对应 ICU 选项组合（`yyyy/MM/dd dddd` → `{dateStyle:'medium', weekday:'short'}`）存进 settings，自定义格式同理；.NET 的 `dddd` 在这里是 `weekday`。

**断点**：`useBreakpoint` 用 `matchMedia` 订阅 `<600 / 600-960 / 960-1280 / >1280`，**只在档位变化时发布**，组件按 `isWide`/`isNarrow` 翻转重渲染。

---

## 6. 关键流程

**6.1 记一条（今天卡片）**
`TodayCard` 是一块自动增高的 `<textarea>`（1 → 6 行后内部滚动），实例配额留给详情（§5）。输入 → `useAutosave` → `journal_save(id=null, …)` → 返回新 id 与 `day_key` → 广播 `data-changed` → 卡片下方"今天已存 n 条"更新，卡片随之清空。两个附加动作：粘贴/拖入图片走 `media_import` 后插一行 `![](ddasset://image/{name})`；点右下角"展开"把当前文本原样交给 `DetailView` 继续排。**卡片与当日列表是两条独立记录**：卡片是草稿式快速入口，写下第一条后它落到列表里、卡片回到空态（PRD §3 的"0 跳页"）。

**6.2 周条打点**
`day_marks(from, to)` 只读 `day_mark` 的主键区间，一次查询覆盖 7 天；切月查月区间。周条与整月共用 `useDayMarks(month)`，切月才失效。

**6.3 搜索**
策略选择集中在 `search.rs::search_query()` 一个函数里：

```
归一化后字符数 >= 3 且 LIKE 元字符已清理 → FTS5 MATCH（bm25 排序 + snippet 取高亮片段）
其余                                       → 有界 LIKE：按 filters 收窄 day_key 区间，LIMIT 200，partial = true
```

`partial` 一路传到 UI 文案（UI.md §3.6）。查询永远 `JOIN journal USING(rowid) WHERE deleted_at IS NULL AND (sensitive=0 OR :show_sensitive)`。返回的 `SearchPage.items` 带 `ranges`，点条目进 `DetailView` 后 `revealRange(first)` 定位。**命中位置一律由 Rust 算**，偏移单位是 Unicode 码点（`&str[..pos].chars().count()` 把 FTS 的字节偏移转过来），`ranges` 直接喂给 `Range.setStart(node, offset)`。

**文内查找与全局搜索共用 `FindBar.vue`**（PRD §2.5），两者差别只在回车之后做什么。详情内的高亮用 CSS Custom Highlight API（`CSS.highlights.set(new Highlight(...ranges))` + `::highlight(dd-hit)`）：高亮只存在于 `CSS.highlights` 集合里，编辑器源文本与 DOM 保持原样。上下键由 `useFind` 维护游标，每步调 `revealRange`；计数取 `search_locate` 回的 `total`。该 API 的可用性由 V-5 验，退化画法在 UI.md §3.6。

**6.4 导出人可读目录**
只读连接流式取全部存活条目 → 写临时目录 `cache/export-<ts>/`：每条 `<day_key>-<id>.md`（front-matter：`id / recorded_at / offset / format / mood / pinned / sensitive / tags / words`；正文只做一件事——把 `ddasset://{kind}/{name}` 换成相对路径 `assets/{name}`，其余字节照抄）+ 附件复制到 `assets/` 子目录（`.thumbs/` 留在原地，导出的是原图）→ 每 20 条 emit 一次进度 → 完成后原子改名到目标。Android 侧走 SAF 返回的 URI 流，改名交还给平台：先写临时文件、flush、关闭，失败时清理，并在 `DataInfo` 里标注这条路径的能力。
**文件名带 id** 保证导入对称：`md_doc.rs` 同一份 front-matter 读写 + 路径双向映射被 export 和 import 共用，重复导入同一目录按 `id` upsert。测试判据："导出 → 导入 → 导出，目录内容逐字节相等（除 `updated_at`）"。

**6.5 备份 / 恢复 / 锁**
备份 = 写连接 `VACUUM INTO 'cache/backup/live.db'` + assets + `manifest.json{app_version, schema_version, counts, created_at, media_included}` → zip。恢复 = 校验 manifest（`schema_version` 高于当前 → `restore_version_mismatch`，提示先升级应用）→ 快照当前库 → 替换 → 关闭并重开连接 → 广播 `all` 失效。
锁 = Rust 监听 `WindowEvent::Focused(false)` 起计时（`lock_leave_delay_s`），到点置锁态并 emit；移动端对应 Tauri 的 lifecycle 事件；`dd://lock-state` 决定前端遮罩。**遮罩的实质是数据停止发送**：判定在 Rust，锁态期间命令与 §3.1 协议同时返回 404，前端手上持有的正文字节数保持为零。

---

## 7. Vditor：四条补丁 + 一笔样式预算

参考项目 140 行 JS 里绝大部分是在给 Vditor 擦屁股，四条直接搬：

1. **工具栏抢焦点**：`toolbar.onmousedown = e => e.preventDefault()`，需要真正触发 click 的按钮加白名单 class。效果是移动端点工具栏时键盘保持展开。
2. **上传劫持**：Vditor 自带 upload 走 HTTP，本项目改用隐藏 `<input type=file>` + `files` 拷贝 + `dispatchEvent('change')`，把选择动作接到应用侧，再喂给 `media_import`，拿回来的 `Asset.name` 拼成 `ddasset://image/{name}` 插进正文（§3.1）。`upload.enable` 保持 `false`，图片插入走 IR 模式自己的路径。
3. **光标恢复**：编辑态与只读态来回切时，从 `vditor[vditor.currentMode].range` 取回选区再 `addRange`，点击后光标停在用户点的位置。
4. **大纲（TOC）**：`.vditor-outline__content` 在窄屏不可用，克隆到自己的抽屉里。大纲排期在 M3。

5. **样式覆盖是一天独立工期，排在编辑器能打字之后、首页接线之前。** 要盖的是 `.vditor-reset`、`.vditor-reset h1…h6`、`.vditor-ir__marker`、`.vditor-toolbar`、`pre` 五处，全部写进 `styles/vditor-override.css`，用 `:where()` 保持零特异度，让后续覆盖始终可用普通选择器完成。取值对照 UI.md §4。

另两件事：公式/mermaid/hljs 资源本地化到 `dist/`，`cdn` 选项指向 `_assets/` 下的本地路径（PRD §7）；§9 的 CSP 里 `style-src 'unsafe-inline'` 与 `data:` 是 Vditor 需要的。

---

## 8. 依赖

**Rust**：`tauri@2`、`rusqlite{bundled}`、`serde`+`serde_json`、`uuid`(v7)、`chrono`(固定 offset) + `iana-time-zone`(启动时读一次本地偏移)、`tokio`(随 tauri)、`thiserror`、`zip`、`pbkdf2`+`sha2`(锁口令)、`mime_guess`(§3.1 的 Content-Type)、`image`(feature 只开 `jpeg`/`png`/`webp`，出缩略图用)、`ts-rs`(dev)。数据目录取 `tauri::path`；FTS 偏移转码点用 `&str[..pos].chars().count()`。

**前端**：`vue@3.5`、`vue-router`、`pinia`、`vditor`、`@tauri-apps/api`、`@tauri-apps/plugin-dialog`、`@tauri-apps/plugin-opener`、`@mdi/js`(取 SVG path 常量；PRD §7 那 17 个心情图标名就是它的 `face-*`)、`@panzoom/panzoom`(附件缩放)、`vite`、`typescript`、`vitest`(composable)。月历与格子规格在 UI.md §3.2，自己实现；日期显示走 `Intl`（§5）。

CI：`cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`、`cargo run --bin export-types && git diff --exit-code src/ipc/bindings`、`vue-tsc --noEmit`、`vitest run`、`node scripts/check-style.mjs`。三平台 matrix 跑构建，测试全在 ubuntu runner 跑一遍。

`check-style.mjs` 约 30 行正则扫 `.vue` 与 `.css` 源文件，拦 UI.md §7 的三条禁忌：组件里出现十六进制色值、出现五档之外的 `font-size`、`!important` 出现在 `vditor-override.css` 之外。

---

## 9. 要改的脚手架配置

1. `tauri.conf.json`：`app.security.csp` 显式写出，并把自定义协议列进去——`default-src 'self'`、`img-src 'self' ddasset: data:`、`style-src 'self' 'unsafe-inline'`、`media-src 'self' ddasset:`。Android/iOS 上 Tauri 把自定义协议映射成 `http://ddasset.localhost`，V-2 确认两种写法是否需要并列。
2. `capabilities/default.json`：权限清单只列 `dialog:allow-open`、`dialog:allow-save`、`opener:default`。文件能力由 Rust 命令承担，WebView 保持零文件系统权限。
3. 窗口配置：移动端安全区、桌面最小尺寸；现有 800×600 固定尺寸在手机上会退化。
4. `clippy.toml`：`unwrap_used`/`expect_used` 设为 `deny`，对应 §4 的 `panic = "abort"` 约束。
5. `tauri-plugin-dialog`（另存为/打开目录）要动三处：`Cargo.toml`、`tauri.conf.json`、`capabilities`。

---

## 10. 测试面

`domain/` 是主战场，优先级最高的是 `md_doc` 往返等价（它承担 PRD §1 "数据可达"）。

- `day_key`：固定 offset 的日期归属、offset 为 +8/+5:30/-5 的边界时刻、夏令时切换前后同一 `day_key` 的稳定性、`recorded_at` 排序与 `day_key` 分组一致性。
- `words`：纯中文、中英混排、代码块、表格、超长文本；`title`/`preview` 的 100/300 截断与换行折叠。
- `md_doc`：随机生成 100 条含全字段与 `ddasset://` 引用的日记 → export → import → export → 比较。
- `trash`：注入假时钟，检查过期集合与保留期边界。
- `name_key`：全角/半角、大小写、尾空格折叠后视为同名。
- `asset_name`：协议 URL → 路径解析，`..`、绝对路径、`%2e%2e`、`t` 之外的 kind、双扩展名五类输入各返回拒绝（§3.1）。

`db/` 用真实临时库（`Connection::open_in_memory` + 跑迁移）：

- 迁移：逐版本 up，中途注入失败后 rollback 的完整性。
- 派生数据：随机 1000 次 save/delete/purge 后，`day_mark` 与 `asset_n`/`cover_asset` 必须与 `data_rebuild` 的全量结果逐行相等（PRD §6 第 3 条的防线）。
- `search`：三字符走 FTS、单/双字符走兜底且 `partial=true`、结果集只含存活条目、`sensitive` 仅在解锁态参与、码点偏移与 `chars()` 计数一致。
- `journal_save` 并发：去抖到点与显式冲刷同时提交 → 一次成功写入，另一次得到 `bad_revision` 并被上层丢弃。

前端测三个 composable：`useAutosave`（`vi.useFakeTimers`，覆盖"未落库就切走必须 await"）、`useBreakpoint`（只在档位变化时 emit）、`useFind`（ranges 变化时游标夹取、空结果、单命中）。视觉一致性由 UI.md 的 token 与 §8 的 `check-style.mjs` 保证，四个视图的观感靠 UI.md §7 的手工清单回归。

---

## 11. 开工前要验的五件事（M0 准入，一天内能出结论）

| # | 验什么 | 受阻时的退路 |
|---|---|---|
| V-1 | 打包的 SQLite 是否开 FTS5 且 ≥3.34（`trigram`） | 走有界 `LIKE`（`partial` 恒 true），后续加索引即可。策略选择已在 Rust 内部（§6.3），前端零改动 |
| V-2 | §3.1 的 `ddasset:` 协议在 Android/iOS WebView 下能否出图、视频 Range 拖动是否可用、CSP 该写 `ddasset:` 还是 `http://ddasset.localhost` | 图片改走内置 `asset:` 协议 + `convertFileSrc`（配一份 scope）；音视频走 `blob:` 或本地回环服务。三种方案共用同一个 IPC 面 |
| V-3 | Vditor 在 WKWebView / Android WebView 上的四条补丁（§7）能否照搬生效，以及 `:where()` 零特异度盖得住它自带的选择器。最早验 | 换 CodeMirror 源码态，"详情即编辑"降级到 M2；样式盖得勉强就加特异度，仍然集中在同一个 override 文件里 |
| V-4 | `Mutex<Connection>` + `spawn_blocking` 下，导出 2GB 附件目录时 UI 读写是否仍顺 | 已按第二只读连接设计（§4）；若仍卡顿，导出改成分片让出连接（每 50 条重新取锁） |
| V-5 | CSS Custom Highlight API（`CSS.highlights` / `::highlight()`）在两个 WebView 上可用（§6.3） | 走 UI.md §3.6 的退化画法（覆盖块只标当前命中）。列表侧高亮走自有模板 |

---

## 12. 集中记录：本项目否决的选项

只有这一节写"不做"，其余章节写"怎么做"。

| 否决 | 现在的替代 | 什么时候重新评估 |
|---|---|---|
| 连接池、`sqlx`、repository trait + mock + DI 容器 | `Mutex<Connection>` + `with_db` helper，`db/journal.rs` 本身就是仓储 | 出现第二个存储后端（多日记本 / 外部 vault）时 |
| ORM（diesel / sea-orm / sqlx 编译期查询） | 手写 SQL + `include_str!` 的 schema 文件，8 张表 | 表数超过 10，或出现需要类型安全 join 的报表需求 |
| Clean Architecture 分层、领域事件总线 | 上限三层 `commands → db/domain → fs` | 无 |
| 编辑器抽象接口（可换 Vditor/CodeMirror） | 直接面向 Vditor API 写 `EditorPane` | V-3 判定 Vditor 不可用时，整体替换 `EditorPane` 内部实现 |
| `@tauri-apps/plugin-sql` | 查询在 Rust 内核，schema/迁移/FTS 策略都在同一层 | 想让第三方直接复用前端时 |
| `plugin-store`、`plugin-fs`、`plugin-upload`、`plugin-stronghold` | settings 进 SQLite；文件能力走自己的 `fs/` 命令；上传场景在桌面/移动里就是系统选择器；加密按 PRD §5 判定 | 出现前端需要直接读写任意路径的需求 |
| TanStack Query | `useCollection()`（20 行）+ `dd://data-changed` 事件驱动失效 | 视图数超过 8 个或出现跨页缓存共享 |
| Tailwind/UnoCSS、VueUse、`vue-i18n`、`dayjs`/`date-fns`、日历与图标字体、连接池以外的 UI 库 | UI.md 的 token + 手写月历 + `@mdi/js` 常量 + `Intl` + 100 行 `t()` | token 体系撑不住多主题时；语言数超过 4 种时 |
| 图片压缩、转码、EXIF 清理、视频首帧图 | §3.1 那张缩略图是唯一加工 | 用户真实反馈原图过大时，且届时要给导出留"存原图"开关 |
| 附件与日记的关联表、引用计数、媒体库、"清理未使用文件"按钮 | 正文里的 URL + 两个派生列 + `data_rebuild(assets)` 顺带清孤儿 | 附件量级大到必须整理时（PRD §2.4 的只读画廊视图） |
| 组件快照测试、覆盖率指标 | §10 那三类：天归属、往返等价、派生一致性；视觉走 UI.md §7 手工清单 | 有第二个 contributor 接手时 |
| 冲突三选框（本地/远端/合并） | `bad_revision` 一律"丢弃本次，以最后一次冲刷为准" | 出现多设备同步（PRD §4 已排除） |
| 双 DOM 并存 + CSS `hidden-*` 断点方案（参考项目） | `useBreakpoint` 档位变化 + 组件按 `isWide` 分支渲染 | 无 |
