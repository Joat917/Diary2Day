# Diary2Day 代码设计 v0.3

对应需求：[PRD.md](./PRD.md)。已定：胖 Rust 内核、Vditor IR 模式、桌面 + 移动同期。

分工：**本文件管结构与接口，[UI.md](./UI.md) 管长相。** 冲突时，交互意图以 PRD 为准，视觉以 UI.md 为准，接口以本文件为准。取舍的理由写在 PRD，本文件只写结论。

否决掉的选项集中在 §12。配套现状：`src-tauri` 是 Tauri v2 空脚手架，`panic = "abort"`、`csp: null`、只有 `tauri-plugin-opener`，§9 列出要改的配置。

---

## 1. 一句话结构

Rust 持有全部真身：schema、查询、天归属计算、字数、附件落盘、导出导入、备份恢复、回收站、应用锁。TS 负责视图、编辑器包装、去抖保存的状态机。**SQL 只存在于 `db/`，`invoke("…")` 字符串只存在于 `ipc/client.ts`。**

```
src-tauri/src/
  lib.rs            装配：建连接、跑迁移、授权 asset 协议目录、注册命令、窗口事件 → 锁
  state.rs          AppState { db: Mutex<Connection>, paths: AppPaths, app, lock: Mutex<LockRuntime> }
  error.rs          AppError(code, msg, detail) + Into<serde_json::Value>，前端按 code 分支；
                    client.ts 的 describeError 把三者拼成可读文本，所有错误态直接展示
  db/
    conn.rs         open + PRAGMA + busy_timeout + with_db
    migrate.rs      版本化迁移（schema/0001_init.sql、0002_m2.sql、0003_media_counts.sql、0004_m3.sql，include_str! 按序执行）
    journal.rs      条目 upsert / 列表 / 详情 / 软删 / 恢复 / purge / 置顶 / 敏感
    tag.rs          标签 CRUD + 合并 + 引用计数
    search.rs       FTS 与短查询兜底，策略选择集中在这一个函数里
    export.rs       导出/导入共用查询：存活条目、附件 upsert、schema_version
    lock.rs         应用锁口令（SHA-256(salt:code)）与锁态快照
    settings.rs     SettingKey 枚举声明类型/默认/校验
  domain/           纯函数，零 IO
    day_key.rs      offset 解析、落在哪天、周条/整月区间、夏令时
    title.rs        首行派生标题、100/300 截断
    words.rs        CJK 按字 / 拉丁按词 / 代码块与表格排除
    name_key.rs     标签名折叠（casefold + 全角折叠 + trim）
  fs/
    assets.rs       附件流式写入、缩略图产出与类型嗅探
  export.rs         导出人可读目录 / 备份 zip / 恢复整库（§6.4–§6.5，含进度事件）
  backup_crypt.rs   备份包可选口令加密：`.ddbak` 头与分块帧、Argon2id 派生、解密校验（§6.5）
  share.rs          移动端分享收件箱 → 附件导入 + 新条目（§6.6）
  commands/         #[tauri::command] 薄壳：解参 → 调上面模块 → 映射 AppError
    journal.rs tag.rs search.rs settings.rs media.rs data.rs lock.rs app.rs share.rs
  events.rs         事件名常量 + payload 类型

src/
  ipc/
    client.ts       invoke 的唯一出口：typed 函数 + 错误反序列化
    types.ts        DTO 手写声明，与 Rust 的 serde 输出逐字段对齐
  utils/assetUrl.ts 逻辑 URL `ddasset://{kind}/{name}` 与物理 asset URL 的唯一映射处（§3.1）
  views/            TodayView DiscoverView MeView DetailView
  components/       TodayCard WeekStrip MiniCalendar JournalCard
                    EditorPane MetaChips AssetViewer
                    CodeDialog ConfirmDialog LockOverlay
                    Icon StateBlock（空态/加载/错误共用一个，UI.md §3.6）
  composables/      useBreakpoint（自动保存、列表加载当前写在各自视图里）
  utils/date.ts     前端日记日：diaryToday / diaryDayKey / 24+ 时刻（与 Rust 同式，只用于显示与写入闸门）
  stores/           session.ts（设置缓存）+ lock.ts（锁态），Pinia 仅此两处
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
  weather      TEXT,                                -- NULL 或 17 个 code 之一（0004_m3）
  recorded_at  TEXT    NOT NULL,                    -- UTC ISO8601
  recorded_off INTEGER NOT NULL DEFAULT 0,          -- 记录时的本地偏移，分钟
  day_key      TEXT    NOT NULL,                    -- YYYY-MM-DD
  created_at   TEXT    NOT NULL,
  updated_at   TEXT    NOT NULL,
  deleted_at   TEXT,                                -- NULL = 存活，否则在回收站
  pinned       INTEGER NOT NULL DEFAULT 0,
  sensitive    INTEGER NOT NULL DEFAULT 0,
  words        INTEGER NOT NULL DEFAULT 0,
  asset_n      INTEGER NOT NULL DEFAULT 0,          -- 派生：save 事务内扫正文的 ddasset://image/
  audio_n      INTEGER NOT NULL DEFAULT 0,          -- 派生：同上，ddasset://audio/
  video_n      INTEGER NOT NULL DEFAULT 0,          -- 派生：同上，ddasset://video/
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

CREATE TABLE app_lock (                             -- M2：应用锁只有一个口令，见 §6.5
  id      INTEGER PRIMARY KEY CHECK (id = 1),
  salt    TEXT    NOT NULL,
  hash    TEXT    NOT NULL,                         -- SHA-256(salt:code)
  enabled INTEGER NOT NULL DEFAULT 0);

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

**排序与区间用 `recorded_at`，日历归属用 `day_key`，显示用 `recorded_at + recorded_off` 重建。** `day_key` 由 Rust 在 upsert 事务内算出：

```
day_key = date((recorded_at → recorded_off 得本地墙钟) - day_boundary_minutes)
```

`recorded_at` / `recorded_off` 是事实，永不因为分界点变化而改写（PRD §2.8）。分界点是 0–479 分钟的整数；设置校验拒绝 ≥480，前端 `parseBoundary` 与 Rust 侧都把越界值夹回区间。前端上传 offset 与墙上时间，`day_key` 由 Rust 单向下发；前端只为 UI 判定实现同一公式（`utils/date.ts::diaryToday`），不写库。分界点改值时按 §2.2 重排。

### 2.2 day_mark 的维护

save / delete / purge 在同一事务内增量更新（`UPDATE ... SET n = n + ?, words = words + ?`）。打点与连续天数一律读这张表。启动自检跑一次全量比对，差异 > 0 则静默全量重算并记日志。全量重算同时暴露给 `data_rebuild(what='day_mark')`（§3）。

**改分界点是一次全局重排**：`settings_patch` 检测到 `dayBoundaryMinutes` 变化时，在同一个事务里调 `journal::recompute_day_keys(conn, boundary)`——遍历全部 `journal`（含回收站）重算 `day_key`，`DELETE FROM day_mark` 后按存活条目 `GROUP BY day_key` 重建；随后写设置并提交。返回全量设置并 emit `dd://settings-changed`，前端以新日界重算今天与写入闸门，重新进入首页即按新 `day_key` 查询。`recorded_at` 不变，所以把分界点改回去就能复原归属。

---

## 3. IPC 面

Rust 命令全部 `snake_case`，TS 侧 `client.ts` 一一映射；分页统一 `limit/cursor`，cursor 是 base64 的 `pinned|recorded_at|id`（置顶优先级也在游标里）。**命令只搬文字**——正文、标题、设置值、元数据与文件路径；既可能是大文件的字节（媒体、导出、备份、恢复）一律由 Rust 直接读写，或由 WebView 走 §3.1 的内置 `asset` 协议直读，不过 IPC。

| 命令 | 入参 → 返回 | 备注 |
|---|---|---|
| `journal_list` | `day_key? range? tag_id? mood? include_trash limit cursor` → `JournalPage` | 排序固定 `pinned DESC, recorded_at DESC, id DESC`，游标按同一顺序推进。摘要字段：id / 派生 title / `preview`(300 字符) / mood / weather / day_key / updated_at / words / `asset_n` / `audio_n` / `video_n` / `cover_asset`；图片 URL 由卡片拼（§3.1） |
| `journal_get` | `id` → `Journal`（全量 + tag 对象） | |
| `journal_save` | `id? content format title mood weather tag_ids recorded_at recorded_off pinned revision` → `Journal` | 唯一写入口。`weather` 只接受 17 个固定 code 或 null，其余返回 `bad_weather`。`day_key` 用当前 `day_boundary_minutes` 现算（§2.1）。同事务：`day_key`、`words`、`asset_n`/`audio_n`/`video_n`/`cover_asset`、派生 title、diff `journal_tag`、增量 `day_mark`、FTS 触发器、`revision+1`。空内容的新建由前端拦截（§5） |
| `journal_delete` | `id` → `day_key` | 软删；同事务更新 `day_mark` |
| `journal_restore` / `journal_purge` | `id` / 无参 | 软删除只改 `deleted_at`。彻底删除与清空回收站都先取正文，删行后清理不再被任何条目引用的附件（§3.1） |
| `journal_trash_list` | `limit cursor` → `JournalPage` | |
| `journal_set_pinned` / `journal_set_sensitive` | `id value` → unit | 入参只有 id 与值 |
| `day_marks` | `from_key to_key` → `DayMark[]` | 周条与整月共用；切月查月区间 |
| `search_query` | `text from? to? tag_id? mood? limit` → `SearchPage{items, partial}` | 空 `text` 走同一 `journal_list` 路径（§6.3）。UI 目前只传 `text` 与 `tag_id`；`from` / `mood` 留给将来的"高级搜索"（PRD §2.5），不预埋界面。`partial=true` 时 UI 显示"结果可能不完整" |
| `tag_list` / `tag_create` / `tag_rename` / `tag_delete` / `tag_merge` | 常规 | create/rename 按 `name_key` 查重，冲突返回 `code=tag_exists` |
| `settings_get_all` | → `Settings` | 一次拿全量，前端启动时缓存（§5） |
| `settings_patch` | `Partial<Settings>` → `Settings` | 服务端校验后回全量，前端覆盖缓存。改 `dayBoundaryMinutes` 时同事务重排全部 `day_key`（含回收站）并重建 `day_mark`（§2.2），随后 emit `dd://settings-changed` |
| `stats_overview` | → `Stats` | 全走聚合 + `day_mark` |
| `media_pick_import` | → `Asset[]` | 打开系统选择器后由 Rust 直接读取选中文件：桌面用 `dialog` 插件，Android 用 SAF `Picker::pick_files`。取消返回空数组；只回元数据，字节不过 IPC。按 64 KiB 分块边读边写，内存不随文件增长；图片抽尺寸并产出缩略图（§3.1） |
| `media_paste_import` | → `Asset[]` | 桌面由 Rust 读系统剪贴板（`arboard`）：文件列表按路径导入，图片编码成 PNG 再导入；Android 返回 `paste_media_unsupported`，提示改用附件按钮 |
| `media_import_paths` | `paths[]` → `Asset[]` | 桌面原生拖放（`onDragDropEvent`）与文件对话框给出的路径；非媒体扩展名跳过 |
| `asset_clean_unused` | → `CleanupReport{removed, freed}` | 手动诊断项：读全部正文得到引用集合（含回收站），遍历 `assets_dir` 删除未被引用的媒体文件与 `asset` 行（§3.1）。锁态返回 `locked` |
| `asset_export_views` | `names[]` → `ExportView{name, view}[]` | 导出前给超大图生成 `.export/{stem}.webp`（限宽 1280px、限总像素 800 万，高按比例，§6.7）；已是小图的直接跳过，前端继续用原图 |
| `share_drain` | → `ShareReport{id, dayKey, assets}` 或 `null` | 移动端分享收件箱：Kotlin `MainActivity` 把系统分享的文本/文件写进 `cache/share-inbox`，命令校验路径、导入资产并建条目（§6.6）。锁态返回 `locked`，收件箱保留待解锁重试 |
| `data_export_dir` | `target_dir` → `ExportReport{dir, journals, assets}` | 桌面。同步等待完成；在 `target_dir` 下新建 `Diary2Day-<ts>/`，人可读 md + `assets/`（PRD §5.2、§6.4）。Android 走 `data_export_android`（§3.2） |
| `data_backup` | `target, password?` → `BackupReport{path, size}` | 桌面。`VACUUM INTO` 快照 + assets + `manifest.json` 压成 zip；带口令时再加密成 `.ddbak`（§6.5）。Android 走 `data_backup_android` |
| `data_restore` | `path, password?` → `ImportReport{journals, assets}` | 桌面。只支持整库替换，不做 merge；加密包先解密再校验 schema、换连接并重载锁运行时（§6.5）。Android 走 `data_pick_restore_android` + `data_restore_android` |
| `backup_password_get` | → `password` 或 `null` | 读取本机记住的备份口令（`data_dir/backup-password.bin`，固定密钥加密，§6.5）；锁态返回 `locked` |
| `backup_password_set` | `password?` → `void` | 写入或删除（`null`）记住的口令；该文件不进入备份包（§6.5） |
| `data_import_md` | `dir` → `ImportReport{journals, assets}` | 桌面。与 export 对称（§6.4）；同一 `id` 按 upsert 处理。Android 走 `data_import_android` |
| `image_export` | `target, data` → `u64` | 桌面。把笔记图片导出器产出的 base64 PNG 写到用户选定路径（`plugin-dialog` 的 `save` 先给 `target`）；校验 base64 与 PNG 魔数，失败 `bad_image`。字节仅在此跨 IPC，分段时逐张调用（§6.7） |
| `image_export_android` | `name, data` → `name` 或 `null` | Android。`save_file` 定位置后 `open_file_writable` 写入同一份 PNG；取消 `null`（§3.2、§6.7） |
| `images_export_android` | `names, datas` → 目录显示名 或 `null` | Android。分段导出（§6.7）：`pick_dir` 选一次目录，先整体解码再逐张 `create_new_file` + `open_file_writable` 写入；取消 `null`（§3.2） |
| `data_export_android` | → `ExportReport` 或 `null` | Android。`Picker::pick_dir` 选目标树，`cache/export` 暂存后逐文件写入 SAF；取消返回 `null`（§3.2） |
| `data_import_android` | → `ImportReport` 或 `null` | Android。`pick_dir` → `cache/import` → 复用 `import_dir`；取消 `null` |
| `data_backup_android` | `password?` → `BackupReport{path, size}` 或 `null` | Android。`save_file` 先定位置（无口令 `.zip` + `application/zip`，有口令 `.ddbak` + `application/octet-stream`），`cache/backup-out` 打包后写入 URI；`path` 是显示名而非绝对路径 |
| `data_pick_restore_android` | → `PickedBackup{path}` 或 `null` | Android。`pick_file` 选 zip / ddbak 并复制到 `cache/restore-pick`；前端确认后再调 `data_restore_android` |
| `data_restore_android` | `path, password?` → `ImportReport{journals, assets}` | Android。复用 `export::restore`，成功后清空 `restore-pick` |
| `data_info` | → `DataInfo{db_path, data_dir, sizes{...}, counts{...}, mobile}` | 关于页与"打开数据目录"用；移动端 `db_path` 返回 `null`，`mobile=true` 让前端隐藏目录行并切到 SAF 动作 |
| `data_rebuild` | `what(day_mark｜fts｜tags｜assets)` → `Report` | 诊断页入口 |
| `app_lock_status` / `app_lock_set` / `app_lock_clear` / `app_lock_unlock` / `app_lock_engage` | `code?` → `LockState{enabled, locked}` | UI 门（PRD §5.4）。`set` 只收新口令，`clear` / `unlock` 收当前口令；`engage` 由前端失焦计时触发 |
| `app_open_data_dir` | → unit | 桌面 |

事件（`events.rs` 里定义常量，前端只 `listen(EVENT.dataChanged, …)`）：

| 事件 | payload | 用途 |
|---|---|---|
| `dd://data-changed` | `{kind:'journal'｜'tag'｜'all', id?, day_key?}` | 列表与打点的局部刷新依据（§5 的失效规则） |
| `dd://op-progress` | `{op_id, phase, done, total}` | 导出/导入每 20 条广播一次（§6.4） |
| `dd://lock-state` | `{enabled, locked}` | 前端失焦计时与锁页共用（§6.5） |
| `dd://settings-changed` | `{keys[]}` | 主题与语言即时生效 |

类型同步：DTO 当前手写在 `src/ipc/types.ts`，字段名与 Rust 的 `serde(rename_all = "camelCase")` 输出逐项对齐（Rust 是真相）。原先计划的 `ts-rs` 生成与 `export-types` 校验尚未接入；接入后生成物放 `src/ipc/bindings/` 并入 CI。

### 3.1 附件走内置 asset 协议，字节走 WebView 直读

图片和音视频由 WebView 自己请求，Rust 只经手元数据，字节不过 IPC。**插入方向同理**：选择、拖放、粘贴三种入口只把路径/URI 或平台能力交给 Rust，文件由 Rust 打开、校验、落盘，WebView 只收到 `Asset[]`；`asset_gc` 也只传文件名（§3）。

**机制**：启用 Tauri 内置的 `asset` 协议（`tauri` 的 `protocol-asset` feature + `app.security.assetProtocol.enable`），启动时 `lib.rs` 用 `app.asset_protocol_scope().allow_directory(assets_dir, true)` 把 scope 收窄到附件目录（含 `.thumbs/`、`.export/`）。仓库里不再有自定义 scheme 的转发代码。物理 URL 由 `@tauri-apps/api/core` 的 `convertFileSrc` 生成：Windows/Android 是 `http://asset.localhost/<编码后的绝对路径>`，其余平台是 `asset://localhost/<编码后的绝对路径>`。文件不存在回 404，越出 scope 回 403，MIME 由 Tauri 嗅探，Range（视频拖动）由 Tauri 处理。

**正文里只存逻辑 URL**（`![](ddasset://image/k3f9a1.png)`），数据库、导出格式与分享文本都是这一份字符串。物理 URL 只在两处出现：把内容交给 Vditor 之前（编辑器初始化、只读预览）用 `expandAssetUrls` 展开，取回编辑器内容或落库之前用 `collapseAssetUrls` 折回。`src/utils/assetUrl.ts` 是唯一映射处；启动时 `main.ts` 通过 `asset_paths` 命令拿到 `assets_dir`（该命令不走锁门禁，否则锁定状态下无法拼 URL）。

**安全**：scope 只覆盖 `assets_dir`，请求路径先经 `SafePathBuf` 再查 scope；`..`、绝对路径跳转与目录外文件一律 403/404。扩展名 → kind 的校验在前端收敛（`collapseAssetUrls` 只把已知媒体扩展名折回逻辑 URL），Rust 侧的 `scan_asset_refs`、导出与分享逻辑不变。

**锁态**：`asset` 协议不经过 `run_blocking` 的锁门禁。启动即锁定时 `App.vue` 的 `viewsReady` 不挂载路由视图，解锁后才挂载并取数（`viewEpoch` 同时重建），因此不再依赖"协议回 404"这条约束；换来的是三端 Range、MIME 与协议行为都由 Tauri 维护。

**缩略图**：导入命令（§3）对图片产出长边 ≤512px 的 `.webp`，文件名取原 name 的 stem（`k3f9a1.png` → `.thumbs/k3f9a1.webp`），卡片请求 `assetUrl("t", …)`；缩略图加载失败回落到原图，再失败显示占位图标。详情加载原图；音视频出图标 + 时长的卡片。

**未使用附件**：是否被使用只由文档内容判定——退出编辑器时撤销栈随之重置，删掉引用的文件已无法恢复，因此就在这一刻清理。`DetailView` 记两份名字：载入正文时的 `referencedAssetNames` 与本次导入新增的 name；退出编辑（点正文编辑器与元信息行以外）、切换条目前与组件卸载前先冲刷保存，**确认这次保存成功**后才用「载入 ∪ 新增 − 最终正文引用」得到待删名单，`asset_gc` 以 fire-and-forget 调用，删除失败只进控制台，不阻塞导航。保存失败（`saveState === "error"`）时跳过这一轮——库里仍是旧正文，按新正文删会删掉仍在引用的文件；下一次冲刷成功后自然补收，失败原因照常显示在保存状态行。`TodayCard` 的待发附件在点删除或卡片卸载时走同一条命令；已提交的附件不参与。日常流程不做全库引用扫描、不维护引用计数：回收站里的条目不会进入编辑会话，其附件自然不会被列入候选。**永久删除另算**：`journal_purge` / `journal_empty_trash` 先读出待删条目的正文，删行后对正文里出现的每个附件名做一次 `instr` 存在性查询，确认没有剩余条目引用后再删原图、缩略图与 `asset` 行；软删除只改 `deleted_at`，文件原样保留，恢复后引用仍有效。**手动清理**：`asset_clean_unused` 只在用户点"清理未使用附件"时执行——读一遍全部正文（含回收站）得到引用集合，再遍历 `assets_dir` 删掉未被引用的媒体文件与原图/缩略图/`asset` 行，回报删除个数与释放字节；不自动化，避免删除动作顺手清掉与当前操作无关的文件。Rust 侧只校验文件名合法（防路径穿越）并删除原图、缩略图与 `asset` 行；缩略图被同 stem 的其它附件共用时保留。

**卡片用哪张图**：`asset` 与 `journal` 保持零关联（PRD §2.4）。卡片取 `journal.asset_n` / `audio_n` / `video_n` 三个分类计数与 `journal.cover_asset`，在 save 事务里和 `words`、派生 title 一起从正文按 `ddasset://{kind}/` 前缀扫出；正文变了它们就跟着变，没有额外的全表重建命令。

### 3.2 Android 文件 IO 走 SAF 插件，字节只在 Rust 侧搬运

Android 的目录/文件选择器只回 `content://` URI，`std::fs` 打不开，`plugin-dialog` 也拿不到目标目录。因此移动端的导出 / 导入 / 备份 / 恢复由 `tauri-plugin-android-fs`（`=30.2.0`，只在 `[target.'cfg(target_os = "android")'.dependencies]` 下引入）承担；不为文件 IO 写自定义 Kotlin 插件。

- **选择**：`Picker::pick_dir` / `pick_file` / `save_file`。取消统一映射成 `Option::None`，IPC 返回 `null`，前端当作"原样返回"。
- **暂存**：命令里先按桌面同一条管线在 `cache_dir/<op>`（`export` / `import` / `backup-out` / `restore-pick` / `restore-decrypt`）生成目录、备份包或解密后的 zip，再用插件返回的 std `File`（`open_file_writable` / `open_file_readable`）与 `std::io::copy` 搬运；字节不过 IPC。成功与失败都清理暂存目录。
- **目录树**：复制递归走 `read_dir` + `create_dir_all_and_return_relative_path` + `create_new_file`；文件名先经 `safe_name` 去掉路径分隔符。
- **命令装配**：全部 `#[cfg(target_os = "android")]`（`commands/android_data.rs`），桌面构建不编译；`generate_handler!` 里的对应条目同样带 `cfg`。前端按 `data_info.mobile` 选命令集，进度行、错误行、恢复确认与成功后重载的流程不变。
- **分享出**：插件自带的 `Opener::share_file` / `share_files` 可把暂存文件交给系统分享面板。暂不单独做"分享条目"入口——后续导出加密时会弹出选择窗口，分享与保存两个按钮就放在那个窗口里，共用同一份暂存文件。
- **单图保存不暂存**：笔记图片导出（§6.7）由 `image_export_android` 直接 `save_file` 拿 URI，解码 base64 后 `open_file_writable` 写入，不落 `cache/`；取消同样映射为 `null`。
- **多图保存不暂存**：分段导出（§6.7）由 `images_export_android` 一次 `pick_dir` 拿目标目录，先整体解码全部 PNG（任一失败整组不写，避免留下半套图片），再逐个 `create_new_file` + `open_file_writable` 写入；取消返回 `null`。

---

## 4. 数据访问的形态

`rusqlite` + `Mutex<Connection>` 是全部并发原语（池、ORM、repository 抽象的取舍见 §12）。

- 写连接只服务交互；导出、备份、一致性校验走第二个只读连接（`Connection::open_with_flags(Readonly)` + `PRAGMA query_only=ON`，WAL 允许并发读）。
- **临界区内只跑同步代码。** 所有数据库操作写进 `tokio::task::spawn_blocking(move || …)`，锁在闭包里进出。`db/conn.rs` 提供 `with_db(|c| …)` 与 `with_db_ro(|c| …)`，业务代码经由这两个 helper 取连接，锁的进出集中在此。

`AppError` 是带 `code` 的枚举（`tag_exists`、`journal_not_found`、`bad_revision`、`bad_weather`、`unsupported_ext`、`restore_version_mismatch`、`backup_manifest_invalid`、`cancelled`、`io`、`sqlite`、`internal`），序列化固定为 `{code, msg, detail?}`。前端按 `code` 分支决定文案与出路。

`panic = "abort"` 下命令里的 panic 直接终止进程。内核的错误处理规则：每个失败路径返回 `Result`；索引访问一律 `get()`；`spawn_blocking` 闭包返回 `Result`；`clippy.toml` 里 `unwrap_used.behavior = "deny"`、`expect_used.behavior = "deny"`（§9）。

---

## 5. 前端形态

**路由**：`vue-router`，5 条平铺路由 `/today`、`/discover`、`/me`、`/theme`、`/j/:id`。分界线一句话：**换"页"走路由，叠"层"走状态**——整月视图、附件大图、文件选择器、确认框都挂在组件局部状态上。Android 返回键走 `window.ddBack()`：先按后进先出消费 `useBackLayer` 注册的叠层（确认/口令/备份框、附件查看器、日历抽屉、三栏详情、标签面板），再在 `history.state.back` 存在时 `router.back()`，都没有才退出应用；锁定状态下直接退出，不动路由。

**桌面三栏落在同样 5 条路由上**：`/today` 在窄屏渲染【周条 + 今天卡片 + 当日列表】，在宽屏渲染同一组件树的【迷你月历｜列表】两栏；屏幕到 1200 才加第三列，详情以 `DetailView` 的**内嵌态**（`entry-id` prop）挂在列表右侧，不切路由。选中日记日写在 `?d=`，选中的条目写在 `?e=`，刷新与前进后退都能还原；直接访问 `/j/:id`（搜索、深链）仍走整页路由。内嵌态复用同一份读写判定：日记今天直接编辑，历史日期只读，关闭时冲刷保存并从 query 里移除 `e`。`/me` 是 PRD §2.7 那张单页分组滚动页（profile + 统计卡 + 五组设置），主题行只显示当前主题名并跳 `/theme`；`/theme` 是浅色/深色调色板卡片页，`stats_overview` 与设置缓存只服务 `/me` 这一屏。

**状态**：全局一个 Pinia store `session`，装两样东西——锁态、`Settings` 缓存。三条规则：

- 服务端数据活在组件里：一个 20 行的 `useCollection()` 覆盖列表的 loading/分页/刷新，四个视图共用。
- 能被 UI 派生的值一律由 Rust 算好返回（连续天数、卡片标题、周条打点）；唯一例外是"日记今天"与 24+ 时刻——它们只依赖时钟与设置，不进库，由 `utils/date.ts` 算（§2.1）。
- 跨视图失效走事件：`journal_save` 成功后 Rust 广播 `dd://data-changed`，`TodayView` 与 `DetailView` 各自判断 payload 影响自己哪一块；列表按 `day_key` 精确重查一页。

**全 App 同一时刻最多一个 Vditor 实例。** 由此定三件事：

- **今天卡片是一块自动增高的 `<textarea>`**（§6.1），只在选中的是日记今天时挂载。正文存的就是 markdown 源，textarea 与 Vditor 读写同一份字符串；它不接 `useAutosave`，失焦或显式提交时整段写入。
- 详情与整篇编辑才挂 `EditorPane`。桌面三栏里"今天卡片 + 详情编辑器"是 textarea 与 Vditor 并存，各管一个输入焦点。
- `EditorPane` 卸载时调 `vditor.destroy()`；`DetailView` 以条目 id 为 key，切条目前先 `flushSave()`，再卸载旧实例、挂新实例（`?e=` 变化走同一条路径）。撤销栈不跨条目保留，滚动位置由每次加载时的 `scrollTop = 0` 重置。

**当前日记与只读过去**：`session.settings.dayBoundaryMinutes` 是唯一输入，`utils/date.ts::diaryToday(boundary, now)` 与 Rust 的 `compute_day_key` 同式。`/today` 的 `selected` 缺省取它；`TodayCard` 只在 `selected === today` 时挂载；`DetailView` 加载时把 `day_key < diaryToday` 置为 `readOnly`，`persist()` 第一行短路，编辑入口（FAB / 顶栏铅笔 / 双击 / `E` / `?edit=1`）全部经过这一个布尔；初始形态由 `?edit=1`（显式编辑意图）或 `settings.autoEdit`（设置「打开即进入编辑」，默认开）决定，带 `?q=` 进入一律先预览。新建路由 `/j/new` 不带日期，保存时用 `recordedAtNow()`，`day_key` 仍由 Rust 单向决定；空白草稿由 `persist()` 的 trim 检查挡在落库之前。

**`EditorPane.vue`** 对外暴露 `modelValue`、`dirty`、`insertText(text)`、`renderSource`（"看源码"切换）、`revealRange(range)`（把 §6.3 的码点区间滚进视野并落光标）。Vditor 的 DOM 归 Vditor：编辑器内容单向流出，外部只在初始挂载与显式"看源码"切换两个时机写入，联动一律走事件回调。Vditor 初始化会清空宿主元素的 `innerHTML`，因此它挂在内层专属容器 `.editor-body` 上，拖入遮罩与插入提示留在外层 `.editor`，由 Vue 独占；原生拖放的命中检测就注册在这个外层元素上（§7 第 2 条）。

**标签与心情**（PRD §2.6）：一个弹层、两个入口。状态属于 `DetailView` 的 `MetaChips` 一份；编辑器工具栏的 `label` 按钮把同一个弹层定位到光标处唤起。

**`useAutosave`** 状态机：

```
clean ──input──▶ dirty ──1s──▶ saving ──ok──▶ clean
                                   │
                                   └─fail─▶ error(保留 dirty 内容 + 重试按钮)
触发冲刷：退出编辑（点正文编辑器与元信息行以外）/ 切换条目 / 路由离开 / 窗口失焦 / 切后台 / beforeunload
冲刷路径一律 await 到落库完成再放行导航
```

失败不重抛、不静默：`saveState` 落 `error`，`saveError` 存完整原因，状态行显示"保存失败 / 重试"与详情；导航照常放行，下一次冲刷重试。`error` 状态下不做附件差集回收——库里仍是旧正文，按新正文删会误删仍被引用的文件（§3.1）。

去抖 1s（PRD §7）是 `useAutosave.ts` 里的具名常量，settings 清单里没有这一项（PRD §2.7）。`useAutosave` 只服务 `DetailView`；今天卡片是一次性提交，不在此列（§6.1）。

`journal_save` 带 `revision`，服务端比对失败返回 `bad_revision`，处理动作一个：**丢弃本次，以最后一次冲刷为准**。

**样式**：取值全在 [UI.md §1](./UI.md)，本文件只管三条写法——色值与间距一律取 token（例外清单 UI.md §1.4）；一律 `logical properties`（`margin-inline-start`），RTL 靠 `dir="rtl"` 翻转，一套布局覆盖两个方向；Vditor 的覆盖集中在 `styles/vditor-override.css`，`!important` 只出现在这个文件里。

**文案**：`i18n/messages/en.ts` 是键的唯一真相（扁平 key + `{n}` 占位），`zh-CN.ts` 用 `satisfies typeof en` 声明，缺键直接编译失败；`t()` 加 `Intl.PluralRules` 约 100 行。语言切换改根节点一个 `locale` ref，文案由计算属性重算，视图树保持挂载。

**时间显示归前端**：Rust 只回 `recorded_at` / `recorded_off` / `updated_at` / `day_key` 原始值，格式化全部在前端做，一律 `Intl.DateTimeFormat`。PRD §7 那三个预设对应 ICU 选项组合（`yyyy/MM/dd dddd` → `{dateStyle:'medium', weekday:'short'}`）存进 settings，自定义格式同理；.NET 的 `dddd` 在这里是 `weekday`。修仙模式的 24+ 时刻也在前端算：本地墙钟早于分界点时，日期取前一天、小时 +24（`displayParts`），再交给格式串与 `Intl`。卡片显示哪个时间由设置 `cardTime` 决定（`recorded` 默认 / `updated`）：`updated` 用该时刻的本地 offset 与同一个分界点格式化；排序、游标与 `day_key` 始终基于 `recorded_at`。

**断点**：`useBreakpoint` 用三组 `matchMedia` 订阅 `<600 / (≥960 或 高 <600 且宽 ≥720) / ≥1200`，**只在布尔翻转时发布**，组件按 `isNarrow`、`isWide`、`isThreePane` 切换。`isWide` 是一条逗号媒体查询：宽度到 960，或横屏手机这种宽度够、高度不足 600 的视口也进宽屏（720 下限保证侧轨 184 + 日历 300 之后列表仍可用）。960 先出两栏（184 侧轨 + 日历 300 + 列表），1200 才出三栏（240 + 300 + 420）；理由见 [UI.md §2](./UI.md)。样式的宽屏分支同样挂在 `isWide` 绑出的 `wide` 类上（`.view.wide`、`.app.wide`），不再单独写 `@media`，避免 JS 与 CSS 两套断点漂移。

---

## 6. 关键流程

**6.1 记一条（今天卡片）**
`TodayCard` 是一块自动增高的 `<textarea>`（1 → 6 行后内部滚动），实例配额留给详情（§5），并且只在选中的是日记今天时挂载。提交走两个时机——失焦 / "保存"按钮——`trim` 为空且没有附件时直接返回，不产生空条目。否则 `journal_save(id=null, recordedAt=now, recordedOff=local)` → Rust 按当前分界点算 `day_key` → 广播 `data-changed` → 卡片清空、高度复位 → 列表重查并停在今天。**它不走去抖自动保存**：这是"写一条就走"的快速入口，不是详情编辑器（PRD §2.8）。粘贴、拖入与文件选择都接到同一条「Rust 直接读文件」的导入管线（§7 第 2 条），附件先排成待发条，提交时把 `ddasset://` 引用追加到正文末尾；只有附件也能保存（UI.md §3.9）。待发附件被移除或卡片卸载时，删除引用已不存在的那部分文件（§3.1）。**卡片与当日列表是两条独立记录**：卡片是草稿式快速入口，写下第一条后它落到列表里、卡片回到空态（PRD §3 的"0 跳页"）。

**6.2 周条打点**
`day_marks(from, to)` 只读 `day_mark` 的主键区间，一次查询覆盖 7 天；切月查月区间。周条与整月共用 `useDayMarks(month)`，切月才失效。

**6.3 搜索**
策略选择集中在 `search.rs::search_query()` 一个函数里：

```
归一化后字符数 >= 3 且 LIKE 元字符已清理 → FTS5 MATCH（bm25 排序 + snippet 取高亮片段）
其余                                       → 有界 LIKE：按 filters 收窄 day_key 区间，LIMIT 200，partial = true
```

`partial` 一路传到 UI 文案（UI.md §3.11）。查询永远 `JOIN journal USING(rowid) WHERE deleted_at IS NULL AND (sensitive=0 OR :show_sensitive)`。返回的 `SearchPage.items` 带 `ranges`（命中码点区间，供将来使用，当前前端未消费）；点条目进 `DetailView` 时带 `?q=`，详情渲染完成后由前端自行定位并高亮。

**搜索跳转高亮**（纯前端，M2 起替代文内查找）：`DetailView` 在预览渲染后按 `?q=` 用 `TreeWalker` 把命中文字包进 `<mark class="dd-hit">`（上限 200 处），再把第一处 `scrollIntoView({block:'center'})`。因此不依赖 `search_locate`，也不改编辑器源文本；不提供查找条、`n/N` 计数或快捷键，编辑态不出现任何标记。代价是超长文只标前 200 处，与有界 LIKE 是同一个取舍。全局搜索仍走 `search_query`。

**6.4 导出人可读目录**
取全部存活条目 → 在用户选择的目录下新建 `Diary2Day-<ts>/`：每条 `<day_key>-<id>.md`（front-matter：`id / recorded_at / offset / format / mood / weather / pinned / sensitive / tags / words`；正文只做一件事——把 `ddasset://{kind}/{name}` 换成相对路径 `assets/{name}`，其余字节照抄）+ 附件按需复制到 `assets/` 子目录（`.thumbs/` 留在原地，导出的是原图）→ 每 20 条 emit 一次进度。桌面入口用 `plugin-dialog`；移动端经 `data_export_android` 的 `pick_dir` 接入 SAF（§3.2）。
**文件名带 id** 保证导入对称：`md_doc.rs` 同一份 front-matter 读写 + 路径双向映射被 export 和 import 共用，重复导入同一目录按 `id` upsert。测试判据："导出 → 导入 → 导出，目录内容逐字节相等（除 `updated_at`）"。

**6.5 备份 / 恢复 / 锁**
备份 = `VACUUM INTO 'cache/backup/live.db'` + assets + `manifest.json{app_version, schema_version, counts, created_at, media_included}` → zip。恢复 = 校验 manifest（`schema_version` 高于当前 → `restore_version_mismatch`，提示先升级应用）→ 解包到 `cache/restore/` → checkpoint 后用占位连接换出旧库、复制新库并重开迁移 → 从新库重载 `app_lock` 运行时并 emit `dd://lock-state` → 广播 `all` 失效。恢复是整库替换，没有 merge 策略。Android 上备份位置由 `data_backup_android` 的 `save_file` 决定；恢复前 `data_pick_restore_android` 先把选的备份包（zip / ddbak）复制进 `cache/restore-pick`，确认后再走同一条恢复管线（§3.2）。

**记住口令**：`backup-password.bin` 放在 `data_dir` 根部，内容 = `[24B 随机 nonce][XChaCha20-Poly1305 密文]`，密钥是源码常量，与 `.ddbak` 的口令派生无关。它只避免明文落盘，拿到安装包即可解密，不当作安全边界。备份 zip 只写 `manifest.json` / `diary.db` / `assets/`，恢复只替换库与附件，所以这个文件既不进备份也不会被恢复覆盖。前端在打开创建/恢复对话框前调 `backup_password_get` 预填，操作成功后按复选框调 `backup_password_set`（`null` 即删除）；失败的操作不改变记录。

**加密备份**：`.ddbak` = 68 字节明文头 + 帧流。头：magic `DD2BAK01`、版本、KDF（Argon2id）、算法（XChaCha20-Poly1305）、m/t/p、块大小、16B salt、24B nonce 基。帧：`[u32 明文长度][密文+16B tag]`，零长尾帧收尾；帧 nonce = nonce 基后 8 字节 + 帧号（BE），AAD = 头部，因此重排、截断、改头都会认证失败。`backup()` 带口令时先生成普通 zip 到 `cache/backup/payload.zip`，再流式加密到目标并删除中间包；`restore()` 读头判定，加密包缺口令返回 `password_required`，解密失败统一 `bad_password`，解密到 `cache/restore-decrypt/`，完整解密后才进入整库替换，结束后无论成败都删掉这份明文 zip。无口令的普通 zip 路径完全不变。
锁 = 口令经 SHA-256(`salt:code`) 存 `app_lock` 单行表，前端 `LockOverlay` 与 Rust 门禁双保险：锁态期间 `run_blocking` 直接返回 `locked`；前端监听 `window blur` / `visibilitychange` 起 `lockLeaveDelayS` 计时（0 表示从不），到点调 `app_lock_engage`。设置与锁状态查询走 `run_blocking_public`，保证锁屏也能渲染主题；`asset_paths` 同样公开（只返回目录路径）。启动即锁定时 `App.vue` 的 `viewsReady` 拦住路由视图挂载，第一次解锁后才挂载并取数；解锁时再递增 `viewEpoch` 强制重建，运行时加锁则保留视图、只盖遮罩。`asset` 协议本身不参与锁门禁（§3.1）。明确告诉用户这是 UI 门，文件不加密。

**6.6 分享接收（Android）**

Android 的 `MainActivity` 在 `onCreate` / `onNewIntent` 处理 `ACTION_SEND` 与 `ACTION_SEND_MULTIPLE`：文本取 `EXTRA_TEXT`，文件取 `EXTRA_STREAM`（缺失时回退 `clipData`），逐个把 `content://` 流复制到 `cache/share-inbox/<uuid>.<ext>`，再把 `{text, files:[{path,name,mime}]}` 追加进同目录的 `pending.json`（先写 `pending.tmp` 再 rename，避免读到半截 JSON）。`AndroidManifest.xml` 为 `text/plain`、`image/*`、`audio/*`、`video/*` 声明 SEND / SEND_MULTIPLE intent-filter；`launchMode=singleTask` 让热启动走 `onNewIntent`。收件箱落在 Tauri 的 `app_data_dir/cache`（Android 上即 `activity.cacheDir`），与 `AppPaths.cache_dir` 同一路径。

Rust 的 `share_drain` 读取收件箱：每个文件 `canonicalize` 后必须仍在 `share-inbox` 内，以流式读取交给 `assets::import_reader`（与媒体导入共用扩展名/嗅探/缩略图管线），正文 = 各段文本 + 附件 markdown（`![](ddasset://image/…)`、`<audio|video controls …>`），再调 `journal::save` 生成新条目（`recorded_at=now`，分界点照常生效）；成功后删除 `pending.json` 与被引用的缓存文件。JSON 解析失败把文件改名为 `pending.bad.json` 留证，避免轮询死循环；锁态下命令返回 `locked`，收件箱原样保留。

前端不引入原生插件桥：`App.vue` 在挂载、窗口聚焦、`visibilitychange` 可见、解锁，以及 Android 前台每 1.5s 调一次 `share_drain`；拿到 `id` 就 `router.push('/j/:id')` 打开新条目。桌面同样调用，只是收件箱通常为空。

**6.7 笔记导出为图片**

预览态顶栏的导出按钮走一条纯前端管线，条目数据不写库、不广播：

1. **离屏渲染**：`exporting` 置真后挂载一段 `position: fixed; inset-inline-start: -10000px; width: 640px` 的 `.export-card`（日期行 + `.preview` 容器，背景 `--c-surface`）。`utils/notePreview.ts::renderNotePreview` 由屏幕预览与导出共用同一份 `Vditor.preview` 选项（主题跟随 `html[data-scheme]` 的明暗、`anchor: 0`），导出时强制 `lazyImages: false`。
2. **等稳定**：`Vditor.preview` 的 promise 只等到 `md2html`，公式 / mermaid / hljs 都是异步补渲染；因此 `utils/noteImage.ts::waitForStable` 用 `MutationObserver` 观察 DOM，静默 250ms 才继续（4s 上限兜底），避免截到半张图。
3. **替换媒体**：`<audio>` / `<video>` / `<iframe>` 在 `foreignObject` 里截不出内容，`replaceMediaElements`（异步）逐个换成静态块。音频建一个隐藏的 `<audio preload="metadata">` 读 `duration`，块内是类型标签 + 起点进度条 + `0:00 / 总时长`；视频用隐藏 `<video crossOrigin="anonymous">` 读时长后 seek 到 10%（上限 1s、下限 0.15s），`drawImage` 到 canvas 取一帧转 JPEG data URL 作封面，块内是 360px 高的封面（`object-fit: cover`）+ 居中播放标 + 右下角时长。`crossOrigin` 依赖 asset 协议回与窗口 origin 匹配的 ACAO，canvas 才不会被污染；元数据超时（4s）或编码不支持时降级成一行类型说明并 `console.warn` 完整错误，不阻断导出。`<iframe>` 仍是一行「嵌入内容 · 文件名」说明；音频与视频块不显示随机附件文件名。动态节点没有 scoped 的 `data-v` 属性，样式必须写成 `.export-card :deep(…)`。
4. **内联图片**：`html-to-image` 走 SVG `foreignObject`，不能引用外部 URL。先 `prepareExportImages` 调 `asset_export_views`，由 Rust 在 `assets_dir/.export/{stem}.webp` 生成导出副本（WebP；**限宽 1280px、限总像素 800 万，高度按比例保留**，长图不裁也不压扁；先按 EXIF 方向拍平，尺寸按拍平后的宽高算），再把导出卡片里对应的 `<img src>`（正文里是 `convertFileSrc` 生成的 asset 地址）换成副本地址；已是小图的跳过、沿用原图，原图与正文引用始终不动。这层是因为 Android WebView 会整张拒解码大位图——实测 4223×8598 的 PNG 连原始 asset 地址在新 `Image` 里都报 `EncodingError`，仅限宽度管不住长图，所以再用总像素预算（解码后约 32MB）兜底；生成失败退回原图，真解不出来仍由下面的错误暴露。随后 `inlineImages` 对每个非 `data:` 的 `<img>` 执行 `fetch(src)` → `FileReader` → data URL → `image.decode()`，读不到或解码失败按 `export_image` 抛出、**不静默换成占位**。这一环依赖各平台 WebView 对 asset 协议的 CORS 行为，必须真机验证。为了在真机上能区分失败环节，detail 不只给源地址：按阶段（下载 / 转 Data URL / 设置后解码）列出 HTTP 状态与响应类型、Content-Type、Blob 类型与字节数、Data URL 字符数与前缀、元素 `complete` 与 `naturalWidth×naturalHeight`；解码失败时再对 Data URL 和当前 `<img>` 地址各起一个独立 `Image.decode()` 探针，写明各自成败与尺寸。
5. **分段**：整篇高度超过 7500px 时，`splitExportChunks` 把卡片按顶层块切成多张，不做降采样。先克隆一份空卡片外壳（保留 scoped 的 `data-v` 属性，`:deep` 样式才继续生效），把原卡片的顶层子节点按顺序搬进新卡并实时量高；越线就把当前节点移到下一张。单个块自身超高时只能让它独占一张。切完移除原卡片，并把每张的日期行改写为「日期 · 第 i/N 张」；只有一张时原卡片原地保留，单图路径与文件名不变。
6. **截图**：`capturePng` 对每张卡片先看 `getBoundingClientRect().height`——0 报 `export_empty`，>16384 报 `export_too_long`（detail 带单块实际高度，此时已无块可切）；再等 `document.fonts.ready`，`pixelRatio = min(2, max(1, (16384-64)/height))`（短图 2 倍，长图自动降倍以免越界），背景取 `--c-surface`，调 `html-to-image` 的 `toPng`。`toPng` 抛错时包成 `export_image`，detail 带卡片宽高与实际 pixelRatio，便于区分是尺寸问题还是渲染问题。
7. **保存**：`data:` 之后的部分交给 Rust。只有一张时走原路径：桌面先 `plugin-dialog` 的 `save` 拿路径（默认 `Diary2Day-{day_key}.png`）再 `image_export`；Android 直接 `image_export_android` 走 `save_file`。多张时桌面弹一次目录选择框（`plugin-dialog` 的 `open({directory:true})`），按 `Diary2Day-{day_key}-01.png` 序号逐个 `image_export`；Android 调 `images_export_android`，`pick_dir` 选一次目录后写入全部。Rust 只校验 base64 与 PNG 魔数（`bad_image`），写入后返回字节数 / 显示名 / 目录名。取消返回 `null`，前端不提示；失败按 §3.6 展示完整 code / message / detail。保存通道按 UA 里的 `Android/iOS` 分流，不调 `data_info`——那条命令会统计目录大小，不适合每次打开详情都跑一遍。

**字节跨 IPC 的唯一例外**：PNG 由 WebView 画布生成，Rust 无法自己读，只能把 base64 传给 Rust（分段时每张各传一次）；媒体导入与其它文件 IO 仍走 asset 协议 / Rust 直读（§3.1、§3.2）。编辑态不提供导出（出图以预览排版为准），不进入导出流程就不重建预览，撤销栈、滚动位置与保存状态都不受影响。

---

## 7. Vditor：四条补丁 + 一笔样式预算

参考项目 140 行 JS 里绝大部分是在给 Vditor 擦屁股，四条直接搬：

1. **工具栏抢焦点**：`toolbar.onmousedown = e => e.preventDefault()`，需要真正触发 click 的按钮加白名单 class。效果是移动端点工具栏时键盘保持展开。
2. **媒体插入不过 IPC**：不用 Vditor 自带的 upload（`cdn` 之外的 HTTP 端点），也不配 `upload` 选项。工具栏加一个自定义按钮 `dd-image`，`click` 调 `media_pick_import`：桌面由 Rust 侧 `dialog` 打开系统选择器，Android 由 SAF `Picker::pick_files` 选择，选中的文件直接由 Rust 读取、校验、落盘并入库，只回 `Asset[]`。拖放走 Tauri 原生 `onDragDropEvent`（`src/utils/fileDrop.ts` 把物理坐标按 `devicePixelRatio` 换算成 CSS 坐标，`elementFromPoint` 向上找到已注册的编辑区或今日卡片），`drop` 把路径交给 `media_import_paths`；HTML5 的 `drag*` 不再使用（桌面默认由 Tauri 拦截系统拖放）。粘贴由 Rust 读系统剪贴板：桌面 `arboard` 先取文件列表、再取图片（编码成 PNG 后走同一条 `import_bytes`），Android 返回 `paste_media_unsupported` 并提示用附件按钮；前端只负责从 `clipboardData.files` 判断"这次粘贴的是媒体"并阻止默认行为。三种入口拿回的 `Asset.name` 拼成逻辑 URL `ddasset://{kind}/{name}`，插入前经 `expandAssetUrls` 展开，用 `editor.insertValue()` 插进正文（§3.1）。`ingest` 一进入就把 `busy` 置真并在编辑器底部显示软提示（按类型取"正在插入图片/视频/音频…"，混合时为"正在插入音频或视频…"），成功与失败都在 `finally` 里撤掉；失败时同一位置保留 `describeError` 的详情 pill，正文不动。今日卡片走同一份 `utils/media.ts`，只是附件先排队不插正文（UI.md §3.9）。导入成功的 name 通过 `assets-imported` 事件交给 `DetailView`，作为退出时清理未使用附件的一半依据（§3.1）。
3. **光标恢复**：编辑态与只读态来回切时，从 `vditor[vditor.currentMode].range` 取回选区再 `addRange`，点击后光标停在用户点的位置。
4. **大纲（TOC）**：`.vditor-outline__content` 在窄屏不可用，克隆到自己的抽屉里。大纲排期在 M3。

5. **样式覆盖是一天独立工期，排在编辑器能打字之后、首页接线之前。** 要盖的是 `.vditor-reset`、`.vditor-reset h1…h6`、`.vditor-ir__marker`、`.vditor-toolbar`、`pre` 五处，全部写进 `styles/vditor-override.css`，用 `:where()` 保持零特异度，让后续覆盖始终可用普通选择器完成。取值对照 UI.md §4。

工具栏的覆盖是同一文件里唯一用容器查询的地方：`.vditor` 上 `container-type: inline-size`，宽容器单行 flex、按钮平分宽度，容器 ≤559px 切 7 列网格两行并隐藏分隔线；不出现横向滚动。

另两件事：公式/mermaid/hljs 资源本地化到 `dist/`，`cdn` 选项指向 `_assets/` 下的本地路径（PRD §7）；§9 的 CSP 里 `style-src 'unsafe-inline'` 与 `data:` 是 Vditor 需要的。

---

## 8. 依赖

**Rust**：`tauri@2` + `tauri-plugin-dialog` / `tauri-plugin-opener`、`arboard`（桌面剪贴板，仅非 Android 目标）、`rusqlite{bundled}`、`serde`+`serde_json`、`uuid`(v7)、`chrono`(固定 offset，本地偏移取 `chrono::Local`)、`base64`（游标）、`tokio`(随 tauri)、`sha2`(锁口令，SHA-256)、`zip`、`mime_guess`(§3.1 的 Content-Type)、`image`(feature 开 `jpeg`/`png`/`webp`/`gif`/`bmp`，出缩略图用)、`argon2`(备份口令 KDF)、`chacha20poly1305`(XChaCha20-Poly1305)、`getrandom`(salt/nonce)、`zeroize`(派生密钥擦除)。数据目录取 `tauri::path`；FTS 偏移转码点用 `&str[..pos].chars().count()`。

**前端**：`vue@3.5`、`vue-router`、`pinia`、`vditor`、`@tauri-apps/api`、`@tauri-apps/plugin-dialog`、`@tauri-apps/plugin-opener`、`@mdi/js`(取 SVG path 常量；PRD §7 那 17 个心情图标名就是它的 `face-*`)、`html-to-image`(笔记导出图片，§6.7)、`vite`、`typescript`。月历与格子规格在 UI.md §3.2，自己实现；附件缩放（UI.md §3.10）用 pointer 事件与 `transform` 写在 `AssetViewer.vue` 里，不引 panzoom；日期显示走 `Intl`（§5）。

CI：当前跑 `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`、`npm run build`（内部即 `vue-tsc --noEmit` + `vite build`）。计划中但尚未接线的：`ts-rs` 生成物 diff 校验、`vitest run`、`node scripts/check-style.mjs`；三平台 matrix 跑构建，测试全在 ubuntu runner 跑一遍。

`check-style.mjs`（约 30 行正则扫 `.vue` 与 `.css`）是计划项，拦 UI.md §7 的三条禁忌：组件里出现十六进制色值、出现五档之外的 `font-size`、`!important` 出现在 `vditor-override.css` 之外。脚本落地前靠 review 守这三条。

---

## 9. 要改的脚手架配置

1. `tauri.conf.json`：`app.security.assetProtocol` 已开启（`enable: true`，scope 运行时收窄）。将来显式写 `app.security.csp` 时，要给附件通道留口——`img-src 'self' asset: http://asset.localhost data:`、`media-src 'self' asset: http://asset.localhost`，两种写法并列覆盖 Windows/Android 与其它平台。
2. `capabilities/default.json`：权限清单只列 `core:default`、`dialog:default`、`opener:default`。文件能力由 Rust 命令承担，WebView 保持零文件系统权限。
3. 窗口配置：桌面最小尺寸；现有 800×600 固定尺寸在手机上会退化。移动端安全区由下面的第 8 条负责，不再依赖 WebView 的 `env(safe-area-inset-*)`。
4. `clippy.toml`：`unwrap_used`/`expect_used` 设为 `deny`，对应 §4 的 `panic = "abort"` 约束。
5. `tauri-plugin-dialog`（桌面另存为/打开目录）要动三处：`Cargo.toml`、`tauri.conf.json`、`capabilities`。
6. 应用图标：`public/app-icon.svg` 是唯一矢量源，`index.html` 以它作为 favicon；修改后运行 `npx tauri icon public/app-icon.svg` 重新生成桌面与移动端图标，不直接编辑 `src-tauri/icons/` 和 `src-tauri/gen/` 里的位图。
7. `tauri-plugin-android-fs = "=30.2.0"`：放在 `[target.'cfg(target_os = "android")'.dependencies]`，`lib.rs` 里只在 Android 注册；桌面构建不下载也不编译。它要求 `tauri ^2.12`，并依赖 `tauri-plugin-fs`。
8. Android 窗口适配：`targetSdk 36` 强制 edge-to-edge，WebView 的 `env(safe-area-inset-*)` 不可靠。`MainActivity.onWebViewCreate` 注册 `ViewCompat.setOnApplyWindowInsetsListener`，把 `systemBars + displayCutout` 与可见的 `ime()` 换算成 dp，经 `addJavascriptInterface` + `evaluateJavascript` 推给 `window.ddApplyInsets(top, right, bottom, left, ime)`；`main.ts` 写 `--safe-top/right/bottom/left` 与 `--ime`，CSS 只认这几个变量（默认值回退 `env()` / 0）。Activity 设 `windowSoftInputMode="adjustResize"` 以接收 IME insets（edge-to-edge 下窗口本身不缩），键盘高度由 `--ime` 扣减：`.app` 用 `calc(100dvh - var(--ime))`，`html[data-keyboard="1"]` 时隐藏底部导航与浮动按钮，输入类浮层在 `--ime` 内避让，聚焦控件在键盘打开过程中反复滚入视野。返回键由 `onBackPressedDispatcher` 接管，`evaluateJavascript("window.ddBack()")` 返回 `true` 表示已消费，否则 `finish()`。状态栏/导航栏图标明暗经 `ddInsets.setDark(true|false)` 交给 `WindowInsetsControllerCompat`，`main.ts` 观察 `html[data-scheme]` 随主题同步。
9. 版本号：`package.json` 是唯一来源。`tauri.conf.json` 的 `version` 写 `../package.json`，桌面安装包与 Android `versionName` 都从它解析；`Cargo.toml` 的 `version` 只作 crate 元数据，手动保持一致。Android `versionCode` 由 Tauri CLI 按 `major*1000000 + minor*1000 + patch` 生成到 `gen/android/app/tauri.properties`（Gradle 读取，不手改、不入库）；需要覆盖时写 `bundle.android.versionCode`。关于页与桌面侧栏的版本号在构建时从 `package.json` 读取，不再硬编码。

---

## 10. 测试面

`domain/` 是主战场，优先级最高的是 `md_doc` 往返等价（它承担 PRD §1 "数据可达"）。

- `day_key`：固定 offset 的日期归属、offset 为 +8/+5:30/-5 的边界时刻、夏令时切换前后同一 `day_key` 的稳定性、`recorded_at` 排序与 `day_key` 分组一致性；分界点 0 / 240 / 479 时 00:00–07:59 的归属、跨月与跨年边界。
- `words`：纯中文、中英混排、代码块、表格、超长文本；`title`/`preview` 的 100/300 截断与换行折叠。
- `md_doc`：随机生成 100 条含全字段与 `ddasset://` 引用的日记 → export → import → export → 比较。
- `trash`：注入假时钟，检查过期集合与保留期边界。
- `name_key`：全角/半角、大小写、尾空格折叠后视为同名。
- `assetUrl`：逻辑 URL ↔ 物理 URL 往返（Windows `%5C` 编码、其它平台 `/`、括号编码、`.thumbs` 不参与折叠），scope 外与不存在的文件在手动回归里确认 403/404（§3.1）。

`db/` 用真实临时库（`Connection::open_in_memory` + 跑迁移）：

- 迁移：逐版本 up，中途注入失败后 rollback 的完整性。
- 派生数据：随机 1000 次 save/delete/purge 后，`day_mark` 与 `asset_n`/`audio_n`/`video_n`/`cover_asset` 必须与 `data_rebuild` 的全量结果逐行相等（PRD §6 第 3 条的防线）。
- 分界点重排：在 17:30Z/+480 存一条（归属次日），改 `dayBoundaryMinutes=240` 后同一条归属到前一日；`day_mark`、列表与回收站同步；改回 0 后归属复原。
- `search`：三字符走 FTS、单/双字符走兜底且 `partial=true`、结果集只含存活条目、`sensitive` 仅在解锁态参与、码点偏移与 `chars()` 计数一致。
- `journal_save` 并发：去抖到点与显式冲刷同时提交 → 一次成功写入，另一次得到 `bad_revision` 并被上层丢弃。

`backup_crypt` 用临时目录做全流程：同一口令 encrypt → decrypt 逐字节相等（空文件、短内容、正好一块、多块跨帧）；错误口令、密文篡改、尾部截断一律 `bad_password`；普通 zip 不被 `is_encrypted` 误判。

前端测 `utils/date` 与三个 composable：`diaryDayKey` / `diaryToday` / `displayParts` 的分界点边界与 24+ 显示、与 Rust 用例的对照表；`useAutosave`（`vi.useFakeTimers`，覆盖"未落库就切走必须 await"）、`useBreakpoint`（只在档位变化时 emit）、`useFind`（ranges 变化时游标夹取、空结果、单命中）。视觉一致性由 UI.md 的 token 与 §8 的 `check-style.mjs` 保证，四个视图的观感靠 UI.md §7 的手工清单回归。

---

## 11. 开工前要验的五件事（M0 准入，一天内能出结论）

| # | 验什么 | 受阻时的退路 |
|---|---|---|
| V-1 | 打包的 SQLite 是否开 FTS5 且 ≥3.34（`trigram`） | 走有界 `LIKE`（`partial` 恒 true），后续加索引即可。策略选择已在 Rust 内部（§6.3），前端零改动 |
| V-2 | ~~自定义协议在 Windows/Android WebView 下 404、Range 能否拖动~~ **已定案**：图片与音视频统一走内置 `asset` 协议 + `convertFileSrc`，scope 收窄到 `assets_dir`；原 `ddasset` 自定义协议已删除。剩余待验：Android WebView 下 `http://asset.localhost` 出图与视频拖动 | 若 Android 仍不可用，再考虑 `blob:` 或本地回环服务；逻辑 URL 与前端映射层不变，只替换 `assetUrl.ts` 的物理 URL 生成 |
| V-3 | Vditor 在 WKWebView / Android WebView 上的四条补丁（§7）能否照搬生效，以及 `:where()` 零特异度盖得住它自带的选择器。最早验 | 换 CodeMirror 源码态，"详情即编辑"降级到 M2；样式盖得勉强就加特异度，仍然集中在同一个 override 文件里 |
| V-4 | `Mutex<Connection>` + `spawn_blocking` 下，导出 2GB 附件目录时 UI 读写是否仍顺 | 已按第二只读连接设计（§4）；若仍卡顿，导出改成分片让出连接（每 50 条重新取锁） |
| V-5 | ~~CSS Custom Highlight API 可用性~~ **已绕开**：搜索跳转高亮用 DOM `<mark>`（§6.3），两个 WebView 都不需要验它 | 若将来要"零 DOM 改动"的高亮，再回到 `CSS.highlights` / `::highlight()` |

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
| `plugin-store`、`plugin-fs`、`plugin-upload`、`plugin-stronghold` | settings 进 SQLite；文件能力走自己的 `fs/` 命令；上传场景在桌面/移动里就是系统选择器；备份包加密用 RustCrypto（§6.5），整库加密按 PRD §5 不做 | 出现前端需要直接读写任意路径的需求 |
| TanStack Query | `useCollection()`（20 行）+ `dd://data-changed` 事件驱动失效 | 视图数超过 8 个或出现跨页缓存共享 |
| Tailwind/UnoCSS、VueUse、`vue-i18n`、`dayjs`/`date-fns`、日历与图标字体、连接池以外的 UI 库 | UI.md 的 token + 手写月历 + `@mdi/js` 常量 + `Intl` + 100 行 `t()` | token 体系撑不住多主题时；语言数超过 4 种时 |
| 图片压缩、转码、EXIF 清理、视频首帧图 | 只有两处加工：§3.1 的 512px 缩略图，和导出前按需生成的限宽 / 限总像素副本（`.export/`，原图一律不动，§6.7） | 需要 EXIF 清理或用户要求视频转码时 |
| 附件与日记的关联表、引用计数、媒体库 | 正文里的 URL + 两个派生列；退出编辑/切条目/离开详情时先确认保存成功，再按文档内容差集调用 `asset_gc`（保存失败跳过本轮，`gc_unused` 不做引用校验）；另有一个手动触发的"清理未使用附件"，按下时做一次全库引用扫描并删孤儿文件（§3.1），不做自动化 | 附件量级大到必须整理时（PRD §2.4 的只读画廊视图） |
| 组件快照测试、覆盖率指标 | §10 那三类：天归属、往返等价、派生一致性；视觉走 UI.md §7 手工清单 | 有第二个 contributor 接手时 |
| 向历史/未来日期补写、给旧日记改日期 | 写入只发生在当前日记日，旧日记只读（PRD §2.8） | 用户明确要求补记，且能给出不破坏时间线的交互时 |
| 冲突三选框（本地/远端/合并） | `bad_revision` 一律"丢弃本次，以最后一次冲刷为准" | 出现多设备同步（PRD §4 已排除） |
| 双 DOM 并存 + CSS `hidden-*` 断点方案（参考项目） | `useBreakpoint` 档位变化 + 组件按 `isWide` 分支渲染 | 无 |
