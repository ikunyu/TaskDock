# 架构与技术决策

悬浮布局保持原模板和字体，通过padding/margin压缩空白；WidgetWindow按body.scrollHeight、固定控件和外层横条计算展开高度，正常120–320，提示/控件过高时提高上限至必要高度（最大600）；使用尺寸键去重与既有队列。Rust set_widget_layout只采用展开高度参数，保留捕获的宽度；原生后备240×240/min220×120，继续DPI适配/失败补偿/持久化。

2026-11-02 字体/顺序调整：widgetTasks支持today/plan并保留完成记录；compareTaskOrder不依赖状态或completedAt，悬浮超时优先使用日期条件而非状态，周概览移除状态排序。未来按时完成项保留未来，已超时未来实例完成当天留在今日，避免勾选后跨页或追加。窗口默认240/min220及前端收起max240同步调整；恢复旧320默认尺寸时窄化，其他保存尺寸保留，无数据库迁移。

2026-11-02 简化：删除SearchView及EmptyState页面组件/入口，底层search_tasks与参数化查询仍供月历和导出复用。TaskList/TaskRow增加dense展示，只显示可打开的单行标题与完成框；原完成确认/事务/revision不变。周概览每日期行默认两项、其余显式展开；widget未来页用groupTaskWeeks按周一至周日分组，收起页仍今日。空状态留白，不隐藏错误或草稿冲突。

周概览复用现有tasks Store快照，WeekOverview.vue通过domain/weeks.ts按本地周一至周日分组，不新增数据服务、IPC、capabilities或SQL。任务有效日期与月历一致：plannedDate优先，其次dueDate和本地dueAt；排除软删除和无日期心愿。统计与分类/状态筛选共用同一分组结果，修改操作复用TaskList和原详情/事务/revision机制，切周不读写数据库。

新建日期限制通过DetailDrawer/QuickEntry/ScheduleEditor的动态最小日期、前端validateCreationDates和Rust create_task调用validation::creation_dates实现；Rust在写入工作线程中按Local当天校验，重复/恢复/历史编辑不调用此策略。TaskRow右侧button使用role=checkbox和受控aria-checked，图标仅按已提交快照渲染，避免原生input的乐观勾选使失败或取消确认显示错误状态。

超时展示复用 domain/tasks.ts 的 isTimedOut；截止时间优先、纯截止日期按本地自然日、无截止则计划日期兜底。今天/未来/悬浮视图都显式传入 Store.now，超时派生状态不写库、不改变 revision。Store 单一下一边界计时器配合30秒与聚焦校时，dispose 清理。收起小窗标记宽度与正文共同测量，保留原几何/IPC架构。

当前 1.0.0 删除流程：主窗口不再提供已删除视图，TaskList 与 DetailDrawer 共用 DeleteTaskDialog，经明确确认直接调用 purge_task；服务校验乐观锁，在同一事务中删除关联记录并更新重复排除日期。set_task_deleted 不再注册 IPC 或前端权限；数据库旧软删除字段/记录和内部重复系列替换语义保留。

当前操作历史浏览功能已移除：不再提供全局/单任务历史页面、查询 IPC 或相关权限。既有 task_history 表、事务审计写入及备份/恢复/JSON 导出兼容保留，不修改迁移或删除用户记录。

当前“任务”工作区合并今天/未来事项页签，心愿单独立展示。复用 planned_date 区分有日期任务与无固定日期的心愿，不新增数据服务或迁移；已划掉心愿保留在心愿单。详见 [任务与心愿单](task-workspace.md)。

1.0.0 增加主窗口专用 `purge_task` IPC，复用 LocalService、任务写事务与 revision 发布；只接受明确确认、版本匹配且已软删除的任务，事务内清理关联行并保留重复日期排除项。详见 [彻底删除说明](permanent-delete.md)。

## 分层

```text
main / widget / quick-entry (同一 Vue 入口，不同窗口标签)
  -> Pinia 状态 + typed bridge
  -> Tauri v2 invoke / listen
  -> Rust 命令边界（窗口、参数验证；结构化错误）
  -> 统一 LocalService / Database 服务
  -> rusqlite + WAL + 版本化迁移
```

第一阶段的设置服务保留；第二阶段在同一 LocalService / Database 边界加入 tasks 仓储与业务方法，前端不获得原始 SQL 或任意文件读写能力。rusqlite 使用 bundled SQLite，减少终端用户运行环境依赖。连接由 Mutex 保护，阻塞操作放入 async command 的 spawn_blocking。

## 多窗口

三个窗口在配置中声明，使用同一个打包入口。配置使用 create=false，由 Rust setup 在注册 LocalService 后通过 WebviewWindowBuilder::from_config 创建，避免 release 内嵌资源先于服务执行 IPC 的竞态。Rust 根据预声明标签打开和聚焦窗口，不接受任意 URL 或标签。前端通过 getCurrentWindow().label 选择视图。普通浏览器预览显式标记“浏览器预览”，使用内存设置，不伪装原生服务或持久化。

初始化先注册设置事件监听，再读取数据库快照。数据库使用单调递增 settings_revision，前端忽略旧版本快照和旧事件，防止乱序回滚。主题写入和 revision 更新使用同一事务，提交后全局 emit，通知失败不回滚已提交设置，前端聚焦/重新打开时重新获取快照。监听器随组件卸载移除。

主窗口关闭即退出（本阶段无托盘）；子窗口关闭隐藏，以便再次打开。使用系统标题栏，保留 Windows 的移动、缩放和辅助功能。第三阶段才引入持久化位置与多显示器工作区修正；目前不恢复旧坐标。

## 第二阶段任务服务

`tasks/model.rs` 定义序列化契约，`validation.rs` 校验 UUID、日期、文本及范围，`repository.rs` 实现聚合读写和审计；`task_commands.rs` 负责调用方授权、阻塞调度及提交后事件。数据库仍通过统一 LocalService 延迟初始化，未增加第二连接服务。

每次任务/分类写入使用 IMMEDIATE 事务，将 task_state.revision、实体 version、子任务和前后快照审计一起提交。失败完整回滚。更新需 expectedVersion；旧版本返回 CONFLICT，避免静默覆盖。分类删除同事务清除关联并递增受影响任务版本。父任务完成的未完成子任务检查在 Rust 再执行，不能仅依赖前端确认。

`taskdock://tasks-changed` 仅发送已提交 revision，和既有设置 revision 独立。三个窗口先订阅再读完整快照，Pinia 合并连续刷新并忽略旧响应；写入返回事务快照以更新当前窗口。获焦/手动刷新补读，30 秒时钟更新本地日期与逾期标记。事件失败不撤销数据库提交。

前端 `domain/tasks.ts` 统一四视图过滤和日期处理；`services/tasks.ts` 为 typed IPC，`stores/tasks.ts` 管理同步和持久状态。详情抽屉持有草稿及基准 version，保存成功才关闭；失败保留草稿，外部更新不覆盖编辑内容。浏览器模式只有空布局预览，写操作返回 DESKTOP_REQUIRED，不另建模拟任务数据库。

| 窗口        | 任务快照 | 创建 | 编辑/删除/恢复 | 完成/撤销 | 分类写入 |
| ----------- | -------- | ---- | -------------- | --------- | -------- |
| main        | 是       | 是   | 是             | 是        | 是       |
| widget      | 是       | 否   | 否             | 是        | 否       |
| quick-entry | 是       | 是   | 否             | 否        | 否       |

AppManifest、permissions、capabilities 和 Rust caller 校验同时限制上述操作。没有新增 SQL、文件、shell 或插件默认权限。审计暂不提供前端读取 IPC。

## 安全

- build.rs 用 AppManifest::commands 注册所有自定义命令，使命令进入 ACL。
- capabilities 按窗口授权。main 可打开子窗口和修改主题；三个窗口均可读快照、监听/注销设置事件、提交受限错误代码；子窗口只可隐藏自己。
- 不启用 core:default、SQL、shell、fs、HTTP、opener、远程来源或前端事件发送权限。
- CSP 禁止远程脚本、eval、外部 frame 和对象，生产仅允许本地资源/IPC。开发模式单独放行本地 Vite HMR。
- Rust 固定数据库位置为实际EXE目录下的data（旧app_local_data_dir仅作为首次迁入来源）；不接受用户提供的文件路径或 SQL。错误对 UI 只暴露稳定代码及中文消息，内部细节写日志。
- 日志由 tauri-plugin-log 在应用日志目录滚动保存，限制大小与文件数；前端仅提交白名单错误代码，不传任意错误正文。

## 失败策略

数据库在首个 bootstrap 中通过进程内唯一 LocalService 延迟初始化，初始化/迁移失败时中止打开数据库，不删除、重建或重命名现有数据库。失败不会缓存为成功连接，界面保留错误面板，可点击重新连接重试。迁移在一个立即事务内执行；校验已应用迁移的名称和原始 SQL，拒绝未知未来版本。数据库忙超时 5 秒，外键始终开启。

主题使用 CSS tokens，system/light/dark 三态，支持系统主题变化、键盘焦点和减少动画设置。界面采用响应布局，阶段三补充跨 DPI 和多屏实测。

原生窗口标题栏复用同一主题偏好：desktop/theme.rs 使用锁定的 Tauri 2.12.5 `WebviewWindow::set_theme`，system 传 None 以恢复系统跟随。初始化显示窗口前、主题事务提交后、恢复设置后同步全部现有窗口；与设置/恢复共用 DesktopRuntime.gate 防止乱序。原生失败记录日志和运行状态警告，不回滚已提交的数据库，也不新增前端权限。主窗口使用标准原生标题栏和客户区；左侧品牌与底部主题区域不收缩，中间导航独立滚动，避免选择底部主题后将品牌卷出视口。

## 第三阶段桌面集成（0.3.0）

此节取代第二阶段关闭主窗口即退出、无几何记忆的运行行为。默认主窗口隐藏到托盘，可设为退出；显式退出保存几何、注销快捷键、checkpoint 并关闭唯一 LocalService，阻止关闭后重开数据库。

桌面偏好仍是 SQLite 真相源，扩展 app_settings.revision/settings-changed，不创建第二 Pinia store。DesktopRuntime 仅记录 OS 实际有效快捷键、启动项状态及告警。窗口几何独立存表，不增加任务/主题 revision。750ms 检测工作区及几何变化，只在状态变化时持久化。

single-instance 最先注册，第二次启动只激活已有 main；三个预声明窗口仍在注册服务后各创建一次。托盘和 global-shortcut/autostart 均只在 Rust 调用，前端无对应插件权限。widget 专属受限命令允许本窗口拖动/右下角缩放/隐藏；main 专属命令允许验证后的设置、位置重置和退出。

ShortcutLease 先注册候选，数据库提交后切换 effectiveShortcut 并移除旧键，失败释放候选。事件只接受当前有效键，旧注册清理失败也不能触发业务。使用 OS 热键注册，无键盘钩子/键盘记录。OS 与 SQLite 不能跨系统原子提交，因此操作序列化、检查 expectedRevision、失败补偿并显示实际状态，不能假装保存成功。

Windows 透明度仅修改本进程 widget HWND 的 layered alpha（20–100%），置顶使用 Tauri API。保存物理 x/y、逻辑内容 width/height 和当时 scale；恢复使用当前 Monitor::work_area 和当前 scale，负坐标合法、离屏回退主屏。--background 不弹主窗口，按保存偏好决定 widget 可见性；应用启动本身不修改系统启动项。

1.1.1新增悬浮窗锁定：Rust按序调用Tauri set_ignore_cursor_events和set_focusable，最后应用layered alpha；相关操作在UI线程执行并向后台工作线程返回结果。拖动和缩放命令仍同步处理。锁定状态写入Schema8的desktop_settings.widget_locked，使用同一服务与设置revision事件；锁定后整体穿透，托盘或主窗口设置提供独立解锁入口。失败补偿原生样式，窗口显隐、布局与恢复后重新应用样式。详见[锁定说明](widget-lock.md)。

## 第四阶段：提醒与重复（源码0.4.0）

新增 schedule/{model,calendar,repository,commands,runtime}.rs，仍使用唯一 LocalService/Database 及任务 revision。任务聚合写入同事务保存提醒配置、系列变更与审计，mutate_tasks 在提交前取消完成/已删除任务的提醒。主窗口独享稍后提醒和通知导航命令，通知插件只在 Rust 使用，不授予 WebView 插件权限。

setup 只注册一次通知点击回调和后台线程。线程每2秒检查数据库；事务内按默认32天（按更早提醒扩展）的范围、每批最多50实例/20系列推进持久游标，以(series_id,occurrence_date)唯一约束防重。每系列每次最多扫描366天，长离线可分多轮补齐。休眠/重启后用当前墙钟补齐；合并最多500条到期提醒为一条系统通知，两次通知至少间隔30秒。线程复用DesktopRuntime.quitting与LocalService生命周期锁，不改变第三阶段退出流程。

提醒先在事务里从pending转notified并写审计/广播，再重新持数据库锁检查任务和提醒是否仍有效后提交给通知插件；任务完成与通知排队串行。notified仅代表进入应用内提醒中心/占用发送机会，channel=unknown，不代表系统实际展示。崩溃在事务提交与系统提交之间可能只有应用内记录；重启不重复发送已认领提醒。插件在后台异步投递，已提交给OS的通知不能承诺撤回。始终保留应用内替代方案，不使用桌面端始终返回Granted的权限API判断用户是否禁止通知。

通知携带taskId，点击在系统支持范围内聚焦已有main，通过受限命令消费待打开任务；若当前有详情草稿则保留并提示。合并通知打开主窗口提醒中心。Windows通知中心里的旧toast不保证回调；完全退出不唤醒进程。

## 第五阶段：数据保护与查询（0.5.0）

新增data模块，复用唯一LocalService、主任务连接和生命周期锁，不创建独立任务数据库/前端数据store。自动备份线程复用DesktopRuntime.quitting，数据操作由data_gate串行；主窗口通过新增data-tools受限IPC调用，无SQL、fs、dialog插件或shell前端权限。dialog插件只在Rust选择导出位置；打开目录只启动固定备份路径的Explorer，不接受任意命令或路径。

备份和流式导出使用同一个数据库的短期只读连接，始终持有LocalService生命周期读锁。在线备份按128页推进、两分钟截止，复制失败不销毁有效备份；长导出使用只读快照事务，在WAL模式下无需长期占用主任务连接mutex。恢复先置maintenance阻止新请求，再获得生命周期写锁等待活动读写/导出/备份完成，因此所有短期连接会在替换前关闭。不是第二套任务服务。

恢复在副本上核对SHA256、完整性/外键、迁移ledger和精确表结构（拒绝未知触发器/视图），兼容迁移旧版本，验证调度JSON后再关闭当前连接。Windows使用ReplaceFileW保留同卷原文件；持久恢复日志和原子发布的commit标记覆盖中断和部分Win32失败。未提交操作在下次初始化服务前回退；安全备份、回退/隔离文件保留。SQLite WAL检查点及DELETE模式切换后才关闭连接，回退候选的WAL/SHM隔离，不用于原库。恢复日志异常时拒绝重建数据库并报告错误。

恢复提升task/settings revision及任务/分类/备份配置版本，旧草稿不能以相同版本覆盖恢复数据。epoch阻止恢复前已认领通知批次在恢复后派送；保留现有和备份的reminder_delivery_guard，按系列/实例日期/slot/时间去重历史提醒。已有唯一实例键及游标继续有效。恢复成功广播database-restored，三个Vue根视图重新初始化；明确确认后清除草稿，主题/任务/调度从新数据库读取。桌面OS资源复用原runtime重载，不重复注册线程/监听器；原生失败作为警告显示，不谎称应用成功。

搜索采用参数化SQL及51取50分页，聚合只读选中任务和其子任务；日历只查询月聚合和选定日分页。事件revision/30秒时钟更新视图，前端防抖和请求代次避免旧响应覆盖。LIKE为转义后的包含匹配，未引入FTS；大数据检索/长读事务仍需第六阶段测量。

## 第六阶段发行整合（1.0.0）

保持28个受限自定义IPC、四份窗口capability、唯一LocalService和Schema5；本阶段不增加迁移。应用标识com.taskdock.desktop不变，安装目录与既有本地数据目录分离。设置按外观桌面/数据/运行状态分组，隐藏页不参与弹窗焦点循环；窗口拖动/缩放走同步受限Rust命令，鼠标仅响应左键。

NSIS currentUser安装、HKCU元数据，无管理员级应用服务；WebView2采用embedBootstrapper，缺少运行时需联网。复用官方安装模板，默认卸载不勾选清理数据，升级也不清理；额外hooks只将完成页运行改为默认不勾选，并在已有启动项时更新其安装路径，不新增用户未选择的启动项。无签名配置，不伪造认证。

本阶段只核对源码/安装脚本、静态检查、Release和安装包构建，没有运行应用或执行功能/GUI验收。

悬浮窗显隐独立于快捷键/开机启动资源注册：runtime.visibility仅切换本窗口可见性，事务更新desktop_settings.widget_visible及必要的app_settings.revision，并发布既有设置事件；重复请求同一显隐值不增加revision。失败时按实际旧可见性补偿。完整设置保存仅在用户更改快捷键配置时申请注册租约，未改动但不可用的快捷键不重试、不伪造effective_shortcut、不清除警告。

DateField.vue为无数据服务的受控日期组件，复用原生type=date的键入/required/min/max校验，日历入口使用内联年月直达面板及相对日期快捷按钮，选择通过update:modelValue回传。统一使用本地纯日期和现有validDate，订阅外部点击随组件卸载移除，Esc仅在面板展开时消费，保留父层对话框键盘语义。

### 悬浮窗字号设置（1.0.0 后续改进）

沿用 DesktopSettings、LocalService、设置 revision 和现有跨窗口事件，无新增 IPC/capabilities。widgetFontSize 10–20、默认13；前后端校验，保存与 revision 同事务，失败保留草稿。WidgetWindow 用局部CSS变量调整任务、周标题和日期的相对字号，主窗口及控件尺寸不变；监听保存字号变化并复用DOM测量调整窗口。Schema6通过新迁移增加字段，同时保留并扩展备份目录记录的Schema范围；既有备份恢复到暂存库后正常升级，新Schema备份由旧程序拒绝。

### 窄版小组件与独立字号

窗口默认宽180、最小160，保留手动缩放及展开尺寸持久化；旧默认宽200/220/240/320采用新默认，其余自定义宽度保留。收起窗按内容在120–180逻辑像素内自适应，常规高度上限280、展开320，内容多时内部滚动；错误/控件较高时允许扩展上限以保持可见。两种模式共用今日/未来页签、任务过滤及按周分组，展开收起不重置所选页签。收起不显示统计、添加按钮或完成操作，已完成任务仍保留删除线。

字号分为展开任务、收起任务、任务时间、周标题和周日期五项，沿用DesktopSettings与revision事件。WidgetWindow局部CSS变量直接使用各设置，字号或页签变化触发完整DOM测量；收起测量包括页签和周标题宽高，避免新增控件被裁切。Schema7保留Schema6原字段与迁移。

### 同一任务的间隔提醒

ScheduleConfig新增可选reminderRepeat（interval、unit=minutes/hours/days），与recurrence生成新任务相互独立。旧JSON缺字段默认关闭，保留Schema7和全部旧迁移、IPC及capabilities。periodic.rs负责计算并在现有事务内维护当前/下次槽位；使用规范化首次时间、单位、间隔、序号构造确定性periodic槽位，复用(task_id,slot)唯一约束和reminder_delivery_guard去重。

现有Rust调度器领取到期提醒前，仅对有到期周期槽位的任务重新计算最近/下次时刻；错过多次时取消旧待发槽位，仅领取最近一次，保留下一次。超过截止不补发。分钟/小时使用UTC时长，按天复用本地日历/DST移时规则。任务完成/删除沿用cancel_inactive；撤销完成时仅恢复定期槽位，不重新开启历史一次性提醒。更新标题保留已领取状态；更改规则取消旧周期槽位，取消所有提醒清除reminderRepeat。提醒中心旧周期条目转已处理但记录和去重账本保留；派发继续使用原有二次有效性检查及30秒合并系统通知。

设置修改、槽位变化、领取及审计继续走唯一LocalService数据库锁、mutate_tasks事务、revision快照和跨窗口事件；恢复校验新增间隔及首次/截止关系验证，JSON导出、SQLite一致性备份自然包含配置与提醒状态。无新线程、数据库连接或系统通知权限。

### 单一完成日期

普通任务UI只显示完成日期及可选time输入，不再显示独立截止方式与第二个日期。新普通任务以planned_date承载用户的目标完成日，due_at仅在选填时间时写入同一日期的精确时刻；全天任务due_date/due_at均空，后端effective_deadline依次用旧due_at、旧due_date、planned_date计算提醒截止，纯日期仍为本地23:59提醒。前端超时逻辑本已支持planned_date次日逾期，不改动窗口任务布局。

旧独立日期记录不批量转换：详情取实际旧截止日展示，标题/描述等编辑保留原日期字段；显式改日期/时间时才同时采用新的完成日期。服务按实际旧/新截止日平移未主动修改的首次提醒锚点，沿用事务及乐观锁。恢复校验兼容旧无显式截止的周期配置；调度遇到新截止早于旧周期首次时安全取消该周期待发项，避免阻塞其他任务。Schema7、迁移、IPC及权限不变。

### 截止方式选择器恢复

日期控件统一命名“截止日期”，保留此前的dueKind下拉选项none/date/time；date模式复用上方日期写due_date，time模式恢复datetime-local并与上方日期同步，none仅去掉额外的显式截止字段/截止与提前提醒。底层planned_date仍用于视图及原有基础跨日超时，不改后端fallback、定期提醒、唯一服务或同步架构。日期/方式/时刻未改动的旧任务仍原样保存，精确旧时间保留原UTC instant。快速输入默认使用仅日期模式，写入同日planned_date/due_date。

### 数据跟随安装目录

storage::Paths以current_exe而非当前工作目录确定data、logs和webview；三个现有WebviewWindowBuilder均配置相同的本地data_directory，日志使用官方TargetKind::Folder。旧app_local_data_dir仅用于定位旧库，不创建新的任务库。LocalService::portable仍是唯一服务，首次database初始化锁内执行一次prepare；错误会缓存到进程结束，避免后台线程每两秒反复复制，修复后重启重试。验证失败不会切换回AppData或创建替代空库；bootstrap保留具体中文错误。

prepare优先已有data数据库。首次迁入使用现有SQLite Online Backup从只读旧库生成同卷`.taskdock-import-UUID`目录内的快照，仅对该副本执行迁移、设置/任务/调度校验及完整性/外键检查，关闭WAL为DELETE并sync；复制平面备份目录，每个文件hash核对并sync，写迁入记录后同卷rename发布整个data目录。已有目标内容、未完成旧恢复、缺失主库但有备份/附属文件、符号链接或Windows重解析点会明确拒绝，不删除原始数据；失败暂存目录留存。单实例在LocalService前生效，调度与恢复继续使用现有生命周期锁。已经启用且配置仍启用的自启动入口刷新到当前EXE，未开启的不注册。

数据管理状态沿用data_status增加dataDirectory只读字段；无新增IPC或文件系统权限。普通NSIS卸载仅删除已安装资源并非递归移除安装目录，因此非空data/logs/webview保留；目录页说明应选择非系统盘。源码打包/忽略规则排除运行数据和暂存目录，不能把用户数据库作为安装资源。应用版本1.0.0、Schema7及旧迁移均保持。

### GitHub签名更新

采用核实的官方tauri-plugin-updater=2.14.1，未使用前端通用updater权限。新增app-updates仅授main四个命令：update_status/check_for_updates/download_update/install_update；固定HTTPS endpoint为ikunyu/TaskDock最新Release的latest.json，安装包路径同仓库且固定文件名。App只有一个UpdateRuntime及一次启动检查，状态通过现有事件监听权限同步到三窗口。

release-config.json提供内部build ID，build.rs校验并编入TASKDOCK_BUILD_ID；公开应用/产品版本保持1.0.0。远程版本为1.0.0+build，使用官方version_comparator按数值只接受更大构建。公钥嵌入配置，requireSignedVersion=true；CLI以--app-version 1.0.0+build重新签署NSIS包，构建号位于受签名保护的trusted comment，防止只改更新JSON冒充新构建。私钥及CLI凭据仅.private目录，不上传或打进源码包。

启动八秒后后台检查，单次20秒超时；下载有180秒超时及进度事件，官方download/verify双重验证后缓存在内存，不执行未签名包。用户再确认安装：保存窗口状态、按现有生命周期锁排空写入并关闭SQLite，然后让Windows安装器替换应用；NSIS最后的/D=指向当前EXE目录，防止默认改装C盘。退出钩子注销快捷键并执行Tauri cleanup，成功由官方插件退出/重启；提取前错误可恢复服务，cleanup之后失败用原生错误消息并直接退出，避免调用已清理资源或留下孤儿进程。data及备份不被作为安装资源替换。

### 1.1.1直接解锁按钮

锁定时，按钮以外区域继续由Windows原生窗口样式鼠标穿透；同位置的按钮尺寸owned辅助窗口接收解锁点击。辅助窗口无任务/文件/任意窗口权限，不获取焦点或显示任务栏图标；主窗口、widget和quick-entry仍复用现有服务，辅助页只读取设置及发起受限解锁。Rust按widget上报的按钮矩形、当前客户区位置和DPI同步辅助按钮，不通过全屏捕获、鼠标轮询或键盘钩子实现。见[锁定说明](widget-lock.md)。

1.1.1锁按钮透明度修复：可见图标保留在widget内，避免背景alpha叠加；owned窗口仅作输入命中层，以Windows最低非零alpha=1/255保留点击，不重复绘制锁图标。不能设为0，否则点击也会穿透。既有设置、DPI定位、锁定恢复及受限权限保持。实际原生视觉和点击仍待验收。
