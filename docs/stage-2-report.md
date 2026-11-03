# 第二阶段验收报告

验收日期：2026-10-28（Asia/Shanghai）。版本：0.2.0。第二阶段基础任务管理已实现，最终自动化检查及 Windows 11 本机 Release 启动、三窗口交互检查通过；其余环境和长跑项目明确列为待验收。交付止于第二阶段，不自动进入第三阶段。

## 已完成功能

- 任务创建、编辑、完成、撤销完成、软删除与恢复；UUID、描述、计划、独立截止、优先级、分类和完成时间持久化。
- 今天/计划/收集箱/已完成四视图及计数，分类筛选，已删除恢复清单。过去未完成计划进入今天并标计划逾期；截止逾期独立显示。
- 截止支持无截止、纯日期和精确时间；纯日期不做时区转换，精确时间统一 UTC。
- 分类新增/编辑/删除；删除分类清空引用、保留任务，并更新相关任务版本及审计。
- 子任务新增、读取、编辑、删除、排序和完成状态；父任务进度自动计算。全部子任务完成不会自动完成父任务；仍有未完成子任务时完成父任务须确认，Rust 再校验。
- 详情抽屉、长度与日期校验、未保存草稿离开确认、过期编辑冲突提示。保存失败保留草稿和数据库原状态，不显示成功。
- 悬浮窗显示真实今日任务、任务完成比例和子任务进度，可完成/撤销、确认及刷新；保留当日相关已完成项以便撤销。
- 快速输入创建收集箱或今天任务，可选择分类，成功后清空标题，失败保留输入。
- 三窗口共享原 LocalService/SQLite，新增独立任务 revision 事件，不重写原设置同步。数据库为唯一真相源，浏览器仅预览空布局，禁止假保存。
- 每次任务/分类变更在同一事务写入聚合前后快照审计，为第五阶段历史浏览预留基础。

托盘、全局快捷键、自启动、透明度/置顶/位置记忆、提醒调度、重复任务、自动备份/恢复、月历高级搜索及正式安装器均未实现。

## 核心实现和技术决策

Rust 按 tasks/model、validation、repository、tests 分模块；task_commands 负责受限 IPC 和提交后通知。Database 继续由唯一 LocalService 延迟初始化，Mutex 保护连接，阻塞工作使用 spawn_blocking。没有前端 SQL、路径或任意文件权限。

任务与分类通过 expectedVersion 乐观锁防止旧草稿覆盖新内容；事务写入 revision、实体 version、子任务及审计后读取一致快照、提交，再发送 taskdock://tasks-changed。事件仅携带 revision，前端合并补读并忽略旧响应，窗口获焦/手动刷新补偿漏事件；30 秒时钟更新跨天和逾期。

四视图共享 domain/tasks.ts：未完成且无计划进收集箱；计划≤本地今天进今天；未来进计划；completed 进已完成。常规视图排除 deleted。小组件额外保留今天计划的已完成项，以及过去计划但今天完成的项。全局计数不受分类筛选影响。

详情草稿独立于数据库快照，任务外部更新不覆盖草稿，保存成功才关闭。全局错误横幅与窗口内容独立渲染，避免保存失败卸载编辑器。DialogFrame 支持仅顶层响应 Esc、Tab 焦点约束及关闭后焦点恢复。

权限矩阵：三个窗口可读；main 可创建/编辑/完成/软删除恢复及管理分类；quick-entry 仅创建；widget 仅完成/撤销。AppManifest、permissions/capabilities 和 Rust caller 校验均限制写入，没有新增宽泛 capabilities。Tauri v2 官方调用/事件/权限文档已复核，API 版本沿用 lockfile。

## 数据库迁移与数据保护

保留 0001_initial.sql 原文，新增 0002_tasks.sql，将 Schema 1 原地升级至 2。新增 STRICT 表 task_state、categories、tasks、subtasks、task_history，包含状态/日期/截止互斥/完成时间/UUID/外键等约束，以及计划、完成、分类、删除、子任务顺序和历史索引。详细字段见 data-model.md。

迁移 DDL 和版本记录同一 IMMEDIATE 事务；重复执行幂等，未知版本或已应用 SQL 被改写时拒绝打开。迁移失败不删除、不重建用户库。实际旧本机库启动前为 Schema 1、theme=system、设置 revision=5；升级后 Schema 1/2 均存在，原设置保留，quick_check=ok。

额外提供 0002_tasks.down.sql 供开发空库显式事务回退验证；已有任务、分类或审计时拒绝下行，防止丢数据。没有生产下行 IPC。已应用的 0001/0002 不再改写，后续使用新迁移。不能直接使用阶段一旧二进制打开 Schema 2。

业务审计和数据修改同事务；注入审计失败或子任务插入失败的临时库测试均完整回滚。分类删除的关联变更与分类审计共享 transaction_id/revision。没有硬删除用户任务。

## 实际环境

沿用 F:\tododock 已有项目和工具链，未初始化 Git、改动无关项目或重新安装工具。

- Windows 11 当前 x64 宿主，系统版本 10.0.26200；WebView2 156.0.4314.50。
- Node 24.16.0、npm 11.13.0；Rust/Cargo 1.99.1、rustup 1.29.2，MSVC target。
- 本地 .tools/vs 工具链和现有 Windows SDK，经 Use-LocalToolchain.ps1 加载。tauri info 仍有独立 SDK/MSVC 组合的发现警告，但实际测试、Clippy、Release 链接均通过；未为消除诊断警告重复安装工具。
- Tauri Rust 2.12.5、tauri-build 2.7.4、JS API/CLI 2.12.4；Vue 3.5.44、Pinia 4.1.0、Vite 8.3.5、TS 5.9.3、Vitest 5.0.5、rusqlite 0.41.0。
- 新增前端交互测试依赖 Vue Test Utils 2.5.1、jsdom 30.1.3；Rust 新增 chrono/uuid/serde_json，实际传递版本见两个 lockfile。

## 实际执行的命令与结果

Rust 命令及原生构建前执行 `. .\scripts\Use-LocalToolchain.ps1`。以下是最终源码结果；构建内再次执行了 typecheck 和 Vite build。

| 命令                                                                                    | 实际结果                                               | 证据（artifacts/stage-2 下）           |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------ | -------------------------------------- |
| npm run format                                                                          | 通过                                                   | 工具执行输出                           |
| npm run typecheck                                                                       | 通过；最终 Release 构建再次通过                        | typecheck.log、release-build-final.log |
| npm test                                                                                | 7 文件，58 项通过，0 失败                              | frontend-tests.log                     |
| npm run format:check                                                                    | 通过                                                   | format-check.log                       |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check                               | 通过                                                   | 最终 Rust 检查执行输出                 |
| cargo test --manifest-path src-tauri/Cargo.toml --locked                                | 21 项通过，0 失败；main/doc tests 无测试项             | rust-tests-final.log                   |
| cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings | 通过                                                   | clippy-final.log                       |
| npm run tauri -- build --no-bundle                                                      | 通过，生成 Windows release/taskdock.exe；无安装器      | release-build-final.log                |
| npm run tauri -- info                                                                   | 退出 0、读出版本；仍有 MSVC/SDK 发现警告               | environment.log                        |
| 只读 Python sqlite3 mode=ro 查询                                                        | Schema 1/2、quick_check=ok、无外键异常；重启后数据保留 | sqlite-check.json                      |

原第一阶段的 21 项前端、9 项 Rust 检查仍包含在当前测试集合；保留断言。一个旧迁移测试夹具原来假定只有一条迁移，第二条出现后会触发主键冲突，已将夹具改为仅修改 version=2 来模拟未来版本，原拒绝行为仍被验证。

58 项前端测试包括四视图/日期变更/跨天、截止语义、widget 保留完成项、两 store 同步、事件乱序与刷新合并、草稿/冲突/保存失败、父任务确认、子任务编辑排序删除、恢复、分类保存失败、旧成功提示清除、顶层 Esc/焦点及权限。根组件补充 main/quick-entry 的失败草稿存续测试，避免仅测局部组件遗漏卸载风险。

21 项 Rust 测试包括原设置和错误连接回归、Schema 1 文件升级/幂等/失败回滚/篡改拒绝、安全下行、任务生命周期/持久化/审计、日期/外键/状态约束、子任务进度与确认、分类删除、审计/子任务失败原子性、两连接旧版本冲突及 30 次并发独立写入一致性。自动化均使用临时库或模拟 bridge，未破坏真实用户库。

## Windows 原生交互检查

使用 computer-use 技能的 Sky 操作真实 Tauri/WebView2 Release，没有使用浏览器预览替代。实际测试日期为 2026-10-28。

| 检查              | 实际观察                                                                                                    |
| ----------------- | ----------------------------------------------------------------------------------------------------------- |
| 旧库升级          | Schema 1→2，原设置 system/revision 5 保留，完整性 ok                                                        |
| 主窗口创建与编辑  | 创建“阶段二验收 · 今日任务”，计划今天，带 1 项未完成子任务；显示 0/1                                        |
| 小组件完成确认    | 从 widget 勾选父任务出现确认；确认后完成，子任务保持未完成                                                  |
| widget→main 推送  | 未重新聚焦主窗口直接读取可访问树，今天 0、已完成 1                                                          |
| main→widget 推送  | 主窗口撤销完成；未重新聚焦 widget 读取到今日完成 0/1、checkbox 撤销                                         |
| 子任务进度        | 完成子任务并保存后显示 1/1、100%；父任务仍未完成                                                            |
| 软删除与恢复      | 主窗口删除后进已删除列表，恢复后今天计数回到 1，子任务状态保留                                              |
| 快速输入          | 创建“阶段二验收 · 快速记录”，无计划日期，选择临时分类；成功清空输入，主窗口后台收集箱计数更新为 1           |
| 分类删除          | 删除自建“阶段二验收分类”，收集箱任务仍为 1 项，分类标记清空；数据库审计包含 category_delete/category_unlink |
| 明暗主题          | 最终三窗口深色/明亮均实际显示且内容可读；详情抽屉 Esc 关闭并将可见焦点还给任务标题                          |
| 最终 Release 重启 | 退出旧进程后重新构建并启动，Schema 2、今天 1、收集箱 1、子任务 1/1 及明亮偏好保留                           |
| 最终三窗口        | 新构建主窗口可打开 widget，widget 可打开 quick-entry；小组件读到持久化任务，前端无浏览器标记                |

截图位于 artifacts/stage-2/screenshots。native-dark-inbox.jpg / native-light-inbox.jpg 为业务验收构建；final-\* 为最终错误边界修复后的构建。数据库只读检查在重启后取得 task revision=9，2 项未完成任务、1 项已完成子任务、0 分类、10 条审计；无 deleted 项。验收记录保留在本机，可由用户正常软删除，不硬删除测试任务或审计。

用户于本轮接管后续手动测试，已停止全部自动 UI 点击。当前主题保留为深色，未再恢复或更改用户窗口。推荐点击步骤见 [手动验收清单](stage-2-manual-checklist.md)；上述已经实际完成的检查记录保持有效。

## 已修复问题

1. 原“已保存”提示可能延续到新编辑或失败操作，现于新编辑、错误及 revision 变化清除，并补回归测试。
2. 保存后焦点回到文档层时，局部 Esc 可能失效；DialogFrame 改为窗口级且仅顶层响应，并验证 Tab 返回对话框。
3. 迟到的成功读取曾可能清掉写入失败提示；任务 store 保留写错误状态，直至下一次写操作或重新初始化。
4. 根组件 task error 与窗口内容曾共用 v-if/v-else，错误可能卸载编辑器丢草稿；拆开渲染并增加主窗口/快速输入根组件回归。修复后 58 项前端测试及 Release 重建通过。
5. 已删除任务不再提供标题编辑入口；恢复仍通过带版本服务。默认侧栏收紧，保留主题 tokens 和较小窗口滚动。

## 风险及未验收

- 仅本机 Windows 11；Windows 10、混合 DPI、多屏断开恢复、高对比度和读屏器未在对应环境验证。跨屏位置恢复本身属于第三阶段。
- 跨天规则通过固定日期/时区自动测试；未修改系统时钟或执行跨夜长跑。三窗口连续高频操作通过两连接及两 store 自动测试覆盖，但没有原生三窗口长时间压力测试。
- 保存失败/事务故障采用临时库注入及 jsdom 交互测试；未人为损坏或锁死真实用户数据库。没有将模拟测试宣称为原生故障注入验收。
- 快照目前读取全部任务和子任务，审计保存聚合前后 JSON；适合当前个人任务阶段，未做大数据量性能基准、分页或历史清理，不承诺容量/延迟。
- 分类 NOCASE 唯一性采用 SQLite 的默认 ASCII 大小写规则；不实现完整 Unicode case folding。
- 内容明暗主题与 Windows 原生标题栏外观可不同；后者沿用系统行为。未实现正式安装器/签名/升级卸载，也无系统通知/托盘验收结论。
- tauri info 的工具发现警告保留；受限沙箱下 Rust 和只读本机 SQLite 检查需额外执行权限，已在批准环境完成，未用删库或替换数据绕过。

## 下一阶段建议与停止点

待用户决定后才进入第三阶段：在现有任务服务上增加托盘、快捷键、窗口交互/状态、多显示器工作区恢复和启动行为，并制定 Windows 10/11 混合 DPI 验收。提醒重复、备份、历史浏览和安装器继续保持各自阶段边界。当前停止于第二阶段。

## 交付产物

- 可执行文件：src-tauri/target/release/taskdock.exe，未签名、无安装器。
- 最终 SHA256：F2EB1EFF05DDBE182DD2DAD3C61F102318CFDB7F8745284D2662E69509F46E1F。
- 需求/架构/数据模型/开发/测试文档已更新；第一阶段报告保持历史结论不改写。

## 新增和修改文件

以下按阶段开始的 SHA256 基线比较，不含 .tools、node_modules、dist、target、gen 和 artifacts；构建自动生成的 7 个受限命令 permission 文件计入。

### 新增

- `docs/stage-2-manual-checklist.md`
- `docs/stage-2-plan.md`
- `src-tauri/migrations/0002_tasks.down.sql`
- `src-tauri/migrations/0002_tasks.sql`
- `src-tauri/permissions/autogenerated/create_task.toml`
- `src-tauri/permissions/autogenerated/delete_category.toml`
- `src-tauri/permissions/autogenerated/save_category.toml`
- `src-tauri/permissions/autogenerated/set_task_deleted.toml`
- `src-tauri/permissions/autogenerated/set_task_status.toml`
- `src-tauri/permissions/autogenerated/tasks_snapshot.toml`
- `src-tauri/permissions/autogenerated/update_task.toml`
- `src-tauri/permissions/tasks.toml`
- `src-tauri/src/task_commands.rs`
- `src-tauri/src/tasks/mod.rs`
- `src-tauri/src/tasks/model.rs`
- `src-tauri/src/tasks/repository.rs`
- `src-tauri/src/tasks/tests.rs`
- `src-tauri/src/tasks/validation.rs`
- `src/components/CategoryManager.vue`
- `src/components/ConfirmDialog.vue`
- `src/components/DialogFrame.vue`
- `src/components/TaskList.vue`
- `src/components/TaskRow.vue`
- `src/domain/tasks.ts`
- `src/services/tasks.ts`
- `src/stores/tasks.ts`
- `src/styles/tasks.css`
- `src/task-types.ts`
- `tests/fixtures.ts`
- `tests/task-domain.test.ts`
- `tests/task-interactions.test.ts`
- `tests/task-store.test.ts`
- `docs/stage-2-report.md`

### 修改

- `docs/architecture.md`
- `docs/data-model.md`
- `docs/development.md`
- `docs/references/tauri-v2.md`
- `docs/requirements.md`
- `docs/roadmap.md`
- `docs/test-plan.md`
- `package-lock.json`
- `package.json`
- `README.md`
- `src-tauri/build.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/capabilities/quick-entry.json`
- `src-tauri/capabilities/shared.json`
- `src-tauri/capabilities/widget.json`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/src/db.rs`
- `src-tauri/src/error.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/service.rs`
- `src-tauri/tauri.conf.json`
- `src/App.vue`
- `src/components/DetailDrawer.vue`
- `src/main.ts`
- `src/services/bridge.ts`
- `src/stores/app.ts`
- `src/windows/MainWindow.vue`
- `src/windows/QuickEntryWindow.vue`
- `src/windows/WidgetWindow.vue`
- `tests/security.test.ts`

## 后续原生增量补测

用户随后重新授权自动测试，并要求避免重复已通过项目。本轮仅补测原生日期修改及编辑冲突，不改源码、不重跑完整构建。

- 过去计划：将已有“阶段二验收 · 今日任务”的计划日期改为 2026-10-27 并保存，仍出现在今天，显示“计划逾期”。通过，证据 extra-overdue.jpg。
- 未来计划：改为 2026-10-29 并保存，今天计数变为 0、计划为 1；进入计划视图，任务实际显示日期 2026-10-29。通过，证据 extra-future-plan.jpg。
- 恢复日期：改回 2026-10-28 后，今天为 1、计划为 0。通过。
- 编辑冲突：主窗口详情输入“阶段二验收：未保存冲突草稿”，不保存；随后在悬浮窗完成同一任务，widget 显示 1/1、100%。之后自动化工具切回主窗口两次报 failed to activate captured window；按技能要求重新选择窗口并重试一次仍失败，停止 GUI 操作。本轮没有验证到主窗口的冲突提示，因此不计作原生通过；此前相应自动化交互/乐观锁测试通过记录仍有效。
- 收尾只读检查：quick_check=ok，无外键异常，revision=14；今日验收任务为 completed、计划 2026-10-28、version=11，description 长度为 0，证明上述草稿未写入数据库。第二条收集箱验收记录保持 pending。记录于 extra-native-check.json。

本轮没有新增或删除任务；对已有验收任务的修改均带审计。今日验收任务目前保持完成，未绕过 UI 直接写库恢复，用户可在已完成视图或小组件撤销。本轮涉及的恢复失败属于 UI 自动化工具限制，尚无证据判定为 TaskDock 应用故障。后续测试以此实际状态为准。
