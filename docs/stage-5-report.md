# 第五阶段代码交付报告

日期：2026-10-29。源码 **0.5.0**，迁移目标 **Schema 5**。

本次只实现功能代码并做静态检查。**未启动TaskDock、未打开或升级真实用户数据库、未运行测试或GUI验收、未构建Release。** 现有exe仍是第三阶段产物；本文不是运行验收通过结论。

## 已实现功能

- 自动备份默认开启，每24小时一次，默认保留30份自动备份；支持1–8760小时周期、1–365份保留量、手动备份、错误信息、时间及备份列表。手动/恢复前安全备份不受自动清理影响。
- SQLite Online Backup API增量复制到临时文件，检查完整性/外键/迁移及表结构，生成SHA256，再同卷发布数据库和清单、更新记录。清理前重新验证最新自动备份，失败保留有效备份；没有复制实时WAL主文件。
- 备份恢复有明确确认及恢复前安全备份。旧Schema在副本上迁移，高版本/损坏/校验或结构不符的备份拒绝；排空连接后替换，失败回退，中断时按持久恢复日志处理。恢复后重载任务、主题、窗口和OS设置，原生设置失败显示警告。
- JSON完整导出含格式版本、Schema、时间、设置、任务、分类、子任务、规则、提醒、历史与备份记录；保留未来导入兼容约定，当前不提供导入。
- CSV导出全部任务或搜索全部匹配项，UTF-8 BOM、正确引用/换行、公式前缀防护；保留任务原状态，另列软删除时间。Rust原生保存对话框选择位置，已有目标再次确认，应用数据库目录禁止作为导出目标。
- 月历作为额外入口，上一月/下一月/今天、分类过滤、数量/完成/高优先级/逾期标记，选定日分页及直接添加，任务使用原详情抽屉。计划日优先，未计划时使用本地截止日；显示已生成的重复实例。
- 搜索标题/描述/子任务，组合状态、完成、优先级、分类、计划/截止范围及逾期条件，清空、50项分页、前端防抖/旧响应丢弃；默认排除软删除。
- 全局及单任务历史，按名称/本地时间范围筛选、倒序分页；显示创建/修改/完成/撤销/删除/恢复/分类/日期及调度变化。安全解析早期字段名和JSON，展示友好字段前后值，不展示原始JSON、不改已有历史。

## Schema 5 与主要技术

新增 `0005_data_tools.sql`，**0001–0004未修改**：data_settings（默认备份策略、版本、尝试/成功/错误）、backup_records（文件名、类型、SHA、大小、Schema）、reminder_delivery_guard（历史发送去重键），并增加任务检索/近期排序和历史时间索引。开发下行脚本遇到备份/发送记录即拒绝，无生产下行IPC。

唯一LocalService继续拥有任务数据库。备份与流式导出使用同一数据库的短期只读连接，受生命周期读锁管理，在WAL下不长期占用主任务连接；恢复以maintenance和生命周期写锁等待全部操作结束。自动备份与手动/恢复通过data_gate协调，不另建任务数据库或前端store。

恢复在临时副本完成迁移和调度规则验证，再checkpoint、切换DELETE模式、关闭所有当前句柄。Windows ReplaceFileW保留同卷原文件；原子发布的恢复日志和commit标记支持失败/下次启动回退，WAL/SHM按候选隔离，安全及回退文件保留。恢复提升revision和实体版本，阻止旧草稿覆盖；epoch丢弃恢复前通知批次，发送guard与旧/恢复状态合并，避免恢复较早备份后再次通知历史实例。

搜索使用固定参数化SQL、转义LIKE和分页，只聚合本页任务/子任务；月历使用数据库月聚合，历史使用只读分页。主窗口增加data-tools命令权限，其他窗口不获数据管理能力；前端没有任意SQL、路径、fs、dialog或shell权限。

## 主要文件

- 新增后端：`src-tauri/src/data/{backup,restore,export,query,model,commands,runtime,mod}.rs`、Schema5及安全下行、`permissions/data.toml`。
- 后端接入：`service.rs`生命周期/epoch，`db.rs`迁移与备份结构核验，`tasks/repository.rs`选中任务聚合，`schedule`恢复去重，`desktop`重载，`lib.rs/build.rs`注册。
- 新增前端：`CalendarView.vue`、`SearchView.vue`、`HistoryView.vue`、`DataManagement.vue`、`data-types.ts`、`services/data.ts`、`styles/data.css`；接入MainWindow、DetailDrawer、SettingsPanel及App恢复事件。
- 更新版本/依赖及锁文件、main capability、README、架构、数据模型、需求、开发/阶段/测试计划、API参考；新增 `docs/data-management.md`。

## 静态检查

- `npm run typecheck`：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked`：通过，包含旧测试代码的编译，**没有执行测试**。
- Rust fmt及受影响前端/JSON/文档Prettier检查：通过。

Tauri v2对话框官方页面实际读取HTTP200，核对并精确锁定dialog 2.8.4及实际API；rusqlite仍为0.41.0，新增backup feature，SHA256使用sha2 0.10.9。未输出未经验证的性能或Windows验收结论。

## 待最终验证与第六阶段建议

优先验证真实WAL并发备份、磁盘满/权限/文件锁、保留策略，以及恢复各中断点、Windows ReplaceFileW部分失败、WAL/SHM、旧Schema升级、回退日志和旧草稿/提醒去重。上述恢复保证是实现方案，尚未通过故障注入或真机验证。保留的安全/回退/隔离及失败暂存文件可能占用额外空间，后续确认状态后再设计清理。

JSON/CSV与Excel行为、历史兼容、日历时区/DST、明暗主题、多窗口回归均待运行验证。包含匹配使用LIKE、分页使用OFFSET，极大数据量和长读事务/WAL增长需测量；原有首屏任务全量快照架构保留。日历不虚拟生成无限未来实例。Windows已显示的通知无法保证撤回。

下一阶段建议先在隔离数据上完成数据保护失败路径和全部回归，再进行Windows/性能/安装升级验收。**本次停止于第五阶段，未进入第六阶段或制作正式安装器。**
