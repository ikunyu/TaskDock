# 数据模型与迁移

当前 1.0.0：用户删除通过 purge_task 确认后永久删除；原 deleted_at 字段继续用于兼容旧数据和内部重复实例替换，不新增迁移、不清理旧软删除记录。永久删除、关联清理和重复日期排除在同一事务内，失败回滚。

当前操作历史浏览功能已移除：不再提供全局/单任务历史页面、查询 IPC 或相关权限。既有 task_history 表、事务审计写入及备份/恢复/JSON 导出兼容保留，不修改迁移或删除用户记录。

当前 UI 将无 planned_date 的记录展示为心愿，有日期的记录为正式任务；心愿已完成后仍保留在心愿单，不进入有日期任务的完成视图。当前 Schema 8；旧任务记录和已应用迁移不修改，旧无日期记录的截止/提醒配置保留。详见 [任务与心愿单](task-workspace.md)。

1.0.0 的彻底删除不增加 Schema：在同一事务内移除任务的 reminders、task_schedule、subtasks、task_history 以及 tasks 行；重复系列 exclusions_json 保留实例日期，提醒去重账本及已有备份保留。普通删除继续使用 deleted_at。详见 [彻底删除说明](permanent-delete.md)。

数据库由Rust在实际EXE目录下data创建taskdock.sqlite3，旧app_local_data_dir只用于首次只读迁入（应用标识com.taskdock.desktop不变）。连接使用 bundled SQLite、foreign_keys=ON、WAL、busy_timeout=5000。前端没有 SQL 或数据库路径参数。

## migration 0001（保留原文）

| 表                | 字段与约束                                                             |
| ----------------- | ---------------------------------------------------------------------- |
| schema_migrations | version 主键、name、原始 sql、applied_at；启动核验已应用迁移名称及内容 |
| app_settings      | 单行 id=1、theme system/light/dark、revision≥0、updated_at             |

## migration 0002（第二阶段）

新建五张 STRICT 表，不重建/覆盖旧数据库，不修改迁移 0001。

| 表           | 已实现字段与约束                                                                                                                                                                                                                 |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| task_state   | id=1、revision（0 到 JS 安全整数上限）；与设置 revision 独立                                                                                                                                                                     |
| categories   | id UUID、name（1–100，NOCASE 唯一）、color #RRGGBB、sort_order、version、created_at、updated_at                                                                                                                                  |
| tasks        | id UUID、title（1–500）、description（≤20000）、status pending/completed、priority none/low/medium/high、category_id 可空、planned_date、due_date、due_at、created_at、updated_at、completed_at、deleted_at、sort_order、version |
| subtasks     | id UUID、task_id、title（1–500）、completed_at、sort_order≥0、created_at、updated_at；UNIQUE(task_id, sort_order)                                                                                                                |
| task_history | id、transaction_id、revision、entity_type task/category、entity_id、task_id 可空、operation、before_json 可空、after_json、occurred_at；JSON 有效性 CHECK                                                                        |

任务/分类/子任务 UUID 在 Rust 和数据库两层校验。task.category_id 外键 ON DELETE SET NULL；subtasks.task_id 和 history.task_id 外键 ON DELETE RESTRICT，防止硬删除丢失关联。服务删除分类前先清空任务分类、增加这些任务 version 并写 category_unlink 审计，不级联删除任务。软删除只写 deleted_at，状态、完成时间与子任务保留；恢复清空 deleted_at。

status 与 completed_at 互相约束：pending 必须无完成时间，completed 必须有完成时间。version 与 revision 使用 JS 安全整数范围；输入排序值在 Rust 限制相同范围。任务创建时按现有最大排序值递增 1024，编辑可保留/修改排序值。子任务顺序为聚合数组顺序，保存时重排为连续整数；最多 200 项。

索引：idx_tasks_active_plan(status, planned_date, sort_order) 为未删除任务部分索引；idx_tasks_completed 为完成时间倒序部分索引；idx_tasks_category、idx_tasks_deleted；idx_subtasks_task(task_id,sort_order)；idx_history_task(task_id,revision)、idx_history_revision。

## 日期、进度与事务

- planned_date 与 due_date 都是纯日历字符串 YYYY-MM-DD，范围 0001–9999，SQLite 和 Rust 检查真实日期，不做 UTC 转换。
- due_at 与 due_date 互斥且都可空。due_at 为 UTC RFC3339 毫秒（例如 2026-10-28T00:00:00.000Z），输入须有时区；UI datetime-local 按本地时间转换。不存在的本地时间拒绝，未修改的时间保留原精度。
- 创建/修改/完成/删除及审计时间由 Rust 生成 UTC；前端显示时本地化。计划跨天过滤依据本地今天，最迟由 30 秒时钟或窗口获焦更新。
- 进度从子任务 completed_at 非空数量计算整数百分比，不保存冗余比例。零子任务为 0/0、0%；父任务完成独立，不自动勾选子任务。
- 写入 IMMEDIATE 事务内先增 revision，再验证 expectedVersion 和业务规则、保存聚合、增加 version、写 before/after JSON 审计、读结果快照并提交。失败包括审计失败全部回滚。返回成功后才广播 revision。
- 审计 operation 包括 create/update/complete/reopen/delete/restore/category_create/category_update/category_delete/category_unlink。一次分类删除的多条审计共享 transaction_id 和 revision。子任务变更保存在父任务前后聚合中；历史浏览 IPC 待第五阶段实现。

## 升级、失败回滚与安全下行

所有待应用迁移在一个 IMMEDIATE 事务内执行，DDL 和 schema_migrations 一起提交。已应用 SQL 或名称变化、未知未来版本都会拒绝打开；失败保留原库，允许界面重试，不删除、重命名或重建数据库。重新执行迁移幂等。

0002_tasks.down.sql 供开发验证，仅可显式事务执行。先检查任务、分类及审计全为空，非空会失败，不能用它删除已有业务数据；空库才依次删除新增表及版本记录，可再升级。应用没有生产下行 IPC，也不能直接用旧二进制打开 Schema 2。用户数据迁移失败回滚与开发空库下行是不同路径。

## 后续扩展（本阶段未创建）

| 实体                                | 预留设计方向                                                     |
| ----------------------------------- | ---------------------------------------------------------------- |
| recurrence_rules / task_occurrences | 规则与实例分离，rule_id + occurrence_key 唯一生成                |
| reminders                           | kind、scheduled_at、state、snoozed_until、delivered_at、attempts |
| window_state                        | logical_bounds、monitor、scale_factor、topmost、opacity          |

后续以新迁移增加关系和索引；Schema 2 没有假重复/提醒字段或调度器。备份、导出和生产恢复另行实现。

## migration 0003：桌面集成

不修改0001/0002。新增 desktop_settings STRICT 单行表：widget_opacity 60–100默认100，widget_topmost/visible/autostart 布尔0/1默认0，shortcut 默认 Ctrl+Alt+T，close_behavior hide/exit默认hide。写偏好与 app_settings.revision 同事务；任务 revision 不变。

window_state STRICT 表：label main/widget/quick-entry 主键，物理 x/y 范围±1000000，逻辑 width/height 范围100–10000，scale范围0.25–8。恢复再按窗口 min/max 与当前工作区约束；最小化/最大化临时外框不覆盖普通几何。

非法但可解码的桌面偏好记录告警并使用安全默认值，写入严格验证。0003_desktop.down.sql 仅开发者显式事务使用，删除新增偏好/几何并保留任务；无生产下行IPC。Schema 2任务保留、幂等、失败回滚及下行再升级有临时库测试。window_state 已实现，不再属于上文未来规划；提醒/重复/备份结构仍未创建。

## migration 0004：提醒与重复（代码已加入，未在本次会话应用到真实库）

不修改0001/0002/0003。新增STRICT表：recurrence_series（UUID、rule/template JSON、起止日、游标、active/version、已完成日期排除集）；task_schedule（task_id主键、配置JSON、series_id外键、不可变occurrence_date、UNIQUE(series_id,occurrence_date)）；reminders（UUID、task外键、逻辑slot、UTC scheduled_at、pending/notified/cancelled/dismissed、channel、claimed_at、预留delivered_at、UNIQUE(task_id,slot)）。待发送与系列游标有索引；没有硬删除任务或重建原表。

同一逻辑提醒slot使用稳定UUID，日期改变重新置pending；未改时间与配置的普通编辑保留已发送/已取消状态。稍后提醒使用独立snooze slot。取消、稍后、认领与对应审计都在原任务事务边界内；reminder\_\*审计包含提醒状态快照，task create/update审计包含配置。delivered_at预留且保持NULL，因为OS不提供可靠展示回执。

系列初始任务为用户显式创建的实例，之后从模板按日历生成，子任务复制为未完成并分配新UUID。仅此一次保留系列模板；此后修改按原occurrence_date切断旧系列并创建新系列，旧未完成实例软删除，完成实例与排除日期保留，防止再次生成其日期。停止系列通过“不重复+此后”实现。纯日期保持YYYY-MM-DD，精确时间RFC3339 UTC；重复按本机当前时区保留墙钟，DST重叠选第一次、缺失时间顺延到当天首个有效分钟（最多180分钟），跨整个日期跳变等特殊情况报错保留数据待调整。

0004_scheduling.down.sql仅供开发者在显式事务中使用，三张新增表非空即拒绝回退；无生产下行IPC。本次未实际执行迁移或下行。

## migration 0005：数据保护与查询

不修改0001–0004。新增STRICT表：

- data_settings：单行id=1，auto_backup默认1，interval_hours默认24（1–8760），retention默认30（1–365），乐观version、last_success/last_attempt/last_error。显式配置与app_settings.revision事务更新；运行状态不会刷任务revision。
- backup_records：UUID、唯一文件名、auto/manual/safety类型、UTC时间、SHA256、字节数、Schema版本。完整DB文件与JSON清单先发布，再记录；文件清单在固定backups目录读取，恢复后合并本地清单，避免旧快照丢失新备份入口。
- reminder_delivery_guard：delivery_key主键、claimed_at。key由系列ID（普通任务用task UUID）、实例日、slot、scheduled_at组成；迁移从已认领提醒初始化。认领与guard同事务，恢复携带当前guard，重新生成过去实例时不重发已认领通知。

新增任务计划/截止/状态优先级/最近修改及历史时间索引。不修改既有审计数据，查看不会产生写入。0005_data_tools.down.sql仅开发者显式事务，存在备份记录或提醒guard即拒绝；应用不提供生产下行。

JSON导出format=taskdock-data、formatVersion=1、appVersion/schemaVersion/exportedAt和tables，含迁移、设置、任务、子任务、分类、规则、提醒、guard、历史和备份目录记录；JSON字段内原始序列化字符串保留原样，不是JSON导入格式已获实现的承诺。未来导入须先拒绝未知formatVersion/更高Schema，再逐级迁移和校验，禁止直接执行导入SQL。

恢复在暂存副本迁移到Schema5后，对任务/分类版本、两类revision和备份配置version抬高到旧/恢复值之上；历史原记录保持不变。备份文件不依赖实时WAL/SHM，不直接复制正在写入的主文件。目录清单不是任意路径能力。

## 1.0.0发行说明

Schema目标继续为5，本阶段未修改任何既有迁移，未打开或升级真实用户数据库。安装资源不含SQLite数据库、备份或用户日志；数据目录标识保持com.taskdock.desktop，默认卸载保留。首次由用户启动新版时仍通过原事务迁移，不重建数据库。

## Schema 6：悬浮窗字号

0006_widget_typography.sql 为 desktop_settings 增加 widget_font_size INTEGER NOT NULL DEFAULT 13，约束10–20。保持既有迁移0001–0005原文不变。backup_records 在同一迁移事务内完整复制到约束支持1–6的目录表后替换，保留全部记录。新备份可记录Schema6；恢复Schema1–5备份继续由现有暂存库升级路径处理，字号默认13。

0006_widget_typography.down.sql仅供开发者在显式事务中使用；存在Schema6备份记录时约束阻止下行并由事务回滚，不丢弃记录。不提供生产数据库降级IPC；用户恢复备份不等同Schema下行。本轮未打开或升级真实用户数据库。

## Schema 7：独立字号

0007_widget_font_controls.sql新增widget_compact_font_size（10–24、默认14）、widget_date_font_size（8–18、默认10）、widget_week_font_size（10–26、默认15）、widget_week_date_font_size（8–18、默认10）；保留widget_font_size作为展开任务字号（10–20、默认13）。迁移从旧字号推导日期和周标题，收起任务至少14。备份目录复制保留全部记录，扩展Schema约束至7；字号与设置revision仍在同一事务保存。0001–0006未修改。开发下行脚本存在Schema7备份记录时拒绝执行，须显式事务回滚，无生产下行IPC。本轮没有访问真实数据库。

## Schema 8：悬浮窗锁定与不透明度

0008_widget_lock.sql在迁移事务中复制替换desktop_settings，完整保留所有旧偏好，将widget_opacity约束扩展为20–100，新增widget_locked布尔字段（0/1，默认0）。backup_records完整复制到允许Schema1–8的表，保留既有备份记录。锁定、解锁和完整设置保存沿用app_settings.revision，不修改任务revision。旧迁移0001–0007不变。下行脚本在锁定开启、不透明度低于60或有Schema8备份记录时拒绝执行，不静默丢失新设置。

## 定期提醒JSON扩展（Schema仍为7）

ScheduleConfig.at在启用定期提醒时用作首次提醒锚点；reminderRepeat缺失或null表示一次性，否则为{interval:正整数,unit:minutes/hours/days}。分钟最长5256000、小时87600、天3650；首次时间必填，有截止时不能晚于截止。task_schedule.config_json及重复任务模板JSON承载可选字段，无新增表列，不改已应用迁移。

reminders.slot新增periodic:<规范化首次UTC时间>:<单位>:<间隔>:<序号>，每次物化独立UUID。只保留当前最近/下次槽位为待发，更早待发记录取消，旧已通知周期记录转dismissed；原记录和reminder_delivery_guard不删除。字段随任务写入与审计事务持久化；恢复继续保留去重账本。旧记录缺少reminderRepeat由serde默认None读取，新版JSON备份导出包含该可选字段；新增字段的配置需要本次或更新程序读取。

## 单一完成日期的界面映射

数据库继续保持Schema7及原字段，不执行数据重写。新普通全天任务：planned_date=完成日期，due_date=NULL，due_at=NULL；选填具体时间时due_at=同一完成日期本地时刻转换的UTC时间。提醒截止优先due_at，其次due_date，否则使用planned_date的当地23:59；界面全天逾期仍从次日开始。精确时间继续按UTC instant判定，纯日期不经UTC换日。

旧独立截止字段保留原优先级。详情的唯一日期显示旧截止日，原有计划日仅因主动修改日期/时间才与之统一；只改标题不改变原日期。旧心愿未设日期仍无日期，不批量覆盖。定期提醒保存校验使用有效完成截止；历史周期首次晚于新默认截止时停止待发，读取/恢复仍允许保留旧配置，不让其阻塞其他提醒。

## 截止方式恢复后的UI映射

本轮仅改前端，不新增Schema或修改迁移。正常新任务默认date模式：planned_date与due_date为同一截止日期，due_at=NULL；time模式：planned_date为datetime-local的本地日，due_at为对应UTC instant，due_date=NULL；none模式保留用于视图的planned_date，清除显式due_date/due_at及截止/提前提醒。沿用已有后端planned_date基础逾期与提醒fallback语义。旧记录不改日期/方式/时刻时保留原字段；精确时间输入与上方日期双向同步，避免新记录出现两个截止日。

## 安装目录存储

数据库Schema仍为7，没有新增迁移。data/taskdock.sqlite3内继续包含所有任务、分类、子任务、设置、历史、提醒及重复规则，data/backups包含完整备份和清单。迁入时保留原库、原备份、任务UUID/revision/提醒去重账本，在新盘暂存快照上执行现有版本化迁移，校验成功后原子发布；不修改旧库迁移记录。迁移清单data/migration.json记录来源、时间和新快照SHA256，不承载任务数据。
