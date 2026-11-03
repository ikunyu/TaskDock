# TaskDock 1.0.0 第六阶段交付报告

2026-10-30 后续整合：按用户要求统一保留 1.0.0，包含月历 SQL 修复、已删除任务的彻底删除，任务列表右侧“已完成”/“撤销完成”按钮，以及统一“任务”入口的今天/未来事项页签和独立心愿单，清理多余的小版本交付包。下方产物大小和哈希已更新为本次统一构建；最新构建日志为 `artifacts/github-updater-build.log`。安装包允许覆盖此前较高应用版本，最新独立字号设置使用新增 Schema 7，不回退或重建数据。相关功能说明见 [月历修复](calendar-fix-report.md) 和 [彻底删除](permanent-delete.md)。

日期：2026-10-29。目标平台：Windows 10/11 x64。源码路径保持 `F:\tododock`；数据库目标仍为Schema5。

**Release与NSIS构建成功；没有执行功能测试、GUI验收或安装。** 本阶段未启动TaskDock、未打开/升级/恢复真实数据库、未删除用户备份。未数字签名，无发布认证或稳定性验收声明。

## 整合与界面

- 2026-11-02 悬浮日期/名称同一行：周日期范围缩到10px淡色，本周/下周文字保持15px；事项名称13px，日期/时间10px，展开的今日/未来共用inlineDate变体，长名称单行省略并保留全文悬停说明。安排日期简写MM.DD，同日精确截止可显示HH:mm，悬停保留完整安排/截止。收起/主窗口/周概览不改，完成确认、删除线和顺序不变。
- 主要文件：TaskList.vue、TaskRow.vue、WidgetWindow.vue、desktop.css及当前说明；不修改后端、数据库、迁移、权限、依赖或版本。TypeScript检查通过，不运行自动化测试、软件或GUI验收，真实窄窗排版待用户体验确认。 Prettier和 `npm run package:windows -- --ci` 通过（artifacts/widget-inline-date-build.log），EXE/安装包副本更新并核对EXE哈希，仍为1.0.0、未签名；与前一源码包核对后端、迁移、权限、依赖及任务排序逻辑不变。

- 2026-11-02 悬浮未来周分组边界修正：标题下分隔线移除，改为每个widget-week-group的细边框/小圆角，周标题和所属任务包含在同一边界内；周组间距保持6px，仅增加3×4px内边距。窗口宽度、字号、任务顺序与完成逻辑不变。仅desktop.css和本报告改动；不启动软件或执行GUI/自动化测试。 Prettier、TypeScript和Release/NSIS构建通过，日志artifacts/widget-week-border-build.log；用户退出后EXE/安装包副本已更新并核对哈希，仍1.0.0、未签名。

- 2026-11-02 再收窄悬浮：默认宽200、最小180，收起宽上限200；旧220/240/320宽度恢复为200，其他保存尺寸保留。原布局模板、字号、分组、任务顺序和高度自适应不变，只缩短展开按钮padding以保证窄窗可见。
- 修改WidgetWindow宽度上限、desktop.css按钮边距、原生runtime/geometry/config和既有配置测试源码及说明，无数据库/权限/版本变更。TypeScript、格式及Rust all-targets静态检查通过（artifacts/widget-narrow-check.log）；不运行自动化测试、应用或GUI验收。 Rust格式及 `npm run package:windows -- --ci` 通过（artifacts/widget-narrow-build.log）；EXE/安装包副本更新并核对EXE哈希，版本仍1.0.0、未签名。模板/任务逻辑、九份迁移、权限及依赖对照不变；实际180/200宽度显示仍待体验确认。

- 2026-11-02 减少悬浮占用、保留原布局：仅压缩外边距、页签/分组/任务间距；对照上次源码包确认WidgetWindow模板、TaskRow/TaskList、任务数据/排序逻辑及字体/grid声明不变。正常展开高度按实际内容120–320，任务多时内部滚动，提示高时为控件保留空间；不额外隐藏控件，不改变当前/已保存宽度。
- 复用widgetLayout高度参数和队列/尺寸去重；原生后备240×240、最小220×120，既有失败补偿、位置/DPI和持久化保留，无数据库/权限变更。主要文件：WidgetWindow.vue、desktop.css、release.css、Rust runtime/geometry、tauri.conf.json、现有安全配置用例及当前文档。
- TypeScript、Rust all-targets静态检查通过（artifacts/widget-space-check.log）；仅编译现有用例，不运行自动化测试、软件、安装或GUI。九份迁移和权限不变。真实自适应高度、边界/横条、多屏和手动缩放交互待用户体验确认。 Rust格式、Prettier及 `npm run package:windows -- --ci` 通过，日志artifacts/widget-space-build.log；EXE/安装包副本更新并核对EXE哈希，仍为1.0.0、未签名。

- 2026-11-02 完成方框悬停保持原色：删除tasks.css中task-completion的hover配色覆盖。未完成的边框/背景及完成后的填色/对勾均按原状态显示，鼠标移入移出不变色；键盘焦点、禁用及完成/撤销逻辑不改。仅样式和本报告改动，版本仍1.0.0，不启动软件或执行GUI/自动化测试。 Prettier、生产TypeScript及Release/NSIS构建通过，日志artifacts/checkbox-hover-build.log；EXE/安装包副本均更新并核对EXE哈希。

- 2026-11-02 调整字体/窄窗：周概览星期17px/650字重、日期11px；悬浮周标题15px/650、事项11px，减小内边距。展开默认240×400/min220，前端收起自适应宽上限240；原生runtime/geometry/config同步修改。恢复旧320默认宽为240，其他自定义保存宽保留；仍可手动缩放，无迁移。
- 修复完成项消失/下移：widgetTasks同时支持today/plan，未来按时完成保留在未来，已过截止的未来实例完成当天保留今日；收起也保留今日完成项。排序不依赖状态/完成时间，超时分组按日期而非状态；周概览也去掉状态排序。复用原TaskList完成确认、撤销和数据库/revision，不改真实任务日期或完成逻辑。
- 主要文件：domain/tasks.ts、domain/weeks.ts、WidgetWindow.vue、WeekOverview.vue、desktop.css、Rust runtime/geometry、tauri.conf.json、既有测试源码和当前文档。新增未来/今日原位保留与超时归类用例，仅类型/编译检查未执行；TypeScript及Rust all-targets检查通过（artifacts/widget-stable-check.log）。不运行自动化测试、软件、GUI或真实数据库验收，混合DPI和真实尺寸/重绘仍待验证。 本轮Rust格式、Prettier及 `npm run package:windows -- --ci` 通过，日志为artifacts/widget-stable-build.log；EXE/NSIS交付副本更新并核对EXE哈希，仍为1.0.0、未签名；九份迁移、权限和依赖不变。

- 2026-11-02 按用户要求简化：移除“搜索与筛选”导航、mode及SearchView；保留月历/导出共用查询和原IPC权限。删除EmptyState组件及主窗口/悬浮/月历/分类/备份空记录文案；空任务列表不保留固定高度空卡片，移除装饰标语和部分重复说明，错误、草稿及危险确认保留。
- 周概览改为单列每日一行，TaskList/TaskRow复用dense模式：仅单行标题、受控完成框，日期及添加/展开在侧；默认每日两项，+N展开；缩小标题/间距，统计改一行，移除大卡片/进度条，空日任务栏留白。默认布局尽量单页，多任务展开后可滚动，不声称所有数据量都单屏显示。
- 悬浮未来事项按groupTaskWeeks归为周一至周日，显示本周/下周/日期范围，跨年显示年份，只渲染非空组；今日/超时/完成与原自适应保持，空小窗保留展开按钮。
- 主要修改：MainWindow/WidgetWindow/WeekOverview、TaskList/TaskRow、domain/weeks、CalendarView、CategoryManager、DataManagement、DetailDrawer、QuickEntryWindow及共享样式；删除SearchView/EmptyState；现有周分组用例源码及当前文档同步更新。功能总览为11模块59项。不改原生代码、数据库、迁移、权限或依赖；本轮不执行自动化测试、GUI或启动应用。 TypeScript和Prettier检查、源码移除引用与原生/九份迁移/依赖版本对照通过；`npm run package:windows -- --ci` 成功，日志为artifacts/minimal-layout-build.log。EXE与NSIS交付副本已更新，EXE哈希与原生输出一致；仍为1.0.0、未签名。紧凑布局真实可见范围与多事项展开待用户体验确认。

- 2026-11-02 新增周概览：左侧月历旁新增入口，每周周一至周日；默认本周，可前后周/返回本周；按日展示任务、完成率、总数/完成/未完成/超时统计，支持分类及状态组合筛选。每日前五项加展开；点击标题复用详情、右侧完成框和确认删除，按日期新建后留在周概览，历史日期不新增。使用响应卡片和既有明暗token。
- 复用唯一tasks Store与后端同步快照，不新增表、迁移、IPC、capabilities或数据库查询/写入。有效日期与月历保持一致：安排日期优先，截止日期/本地截止时间兜底；软删除和无日期心愿排除；重复仅显示已生成实例。周一/跨月/跨年和时区日期计算使用本地自然日，不用UTC日期截断。
- 新增domain/weeks.ts、WeekOverview.vue、tests/weeks.test.ts、docs/weekly-overview.md；修改MainWindow.vue、README、架构/需求/安装及功能总览。当前功能总览为11模块61项。周边界、跨年/闰日、截止兜底、分类/状态及删除排除用例仅类型检查，未执行测试。没有启动应用、安装、GUI或访问真实数据库。 本轮TypeScript和Prettier检查、`npm run package:windows -- --ci` 通过（日志artifacts/weekly-overview-build.log）；EXE和NSIS交付副本已更新，EXE哈希与原生输出一致，仍为1.0.0、未签名。对照前一源码包确认原生源码、权限、九份迁移、依赖及版本均未修改。

- 改善日期选择：新增受控DateField.vue，安排日期、纯日期截止、重复结束及快速输入日期复用；保留键入与原生min/max/required校验，右侧日历按钮提供年份直接输入、月份切换、翻月和日期网格，并提供今天/明天/一周后快捷选择。不再依赖原生弹出日历逐月滚动；限制外日期/月份禁用，历史日期编辑保留。
- 面板内联布局，沿用明暗tokens，不引入第三方依赖、文件权限、IPC或数据库结构。外部点击关闭并清理监听；Esc只在打开面板时消费，选择后恢复按钮焦点，继续沿用前后端新建日期校验。主要文件：DateField.vue、DetailDrawer.vue、ScheduleEditor.vue、QuickEntryWindow.vue及当前说明。
- TypeScript检查通过；本轮不运行自动化测试、应用或GUI验收，真实WebView2日期输入、年月选择和小窗口布局仍待体验确认。 Prettier和 `npm run package:windows -- --ci` 通过，日志为 artifacts/date-picker-build.log；版本仍1.0.0，九份迁移、依赖锁和原生配置不变。

- 缩小完成框并对齐删除图标：TaskRow完成框可见边长从28改为18逻辑像素，对勾从18改为14；完成与删除按钮都使用28×28点击区域，操作栏align-items:center，删除按钮取消通用按钮的大内边距，两个图形中心保持同一高度。只修改TaskRow.vue与tasks.css，不改变完成、确认、删除或数据逻辑。
- 本轮受影响文件Prettier与生产TypeScript检查通过；未运行自动化测试、应用或GUI验收，视觉效果待用户确认。版本保持1.0.0。 Release/NSIS构建成功，日志为artifacts/completion-align-build.log；用户退出后EXE和安装包副本已更新并核对EXE哈希，均未签名。

- 最新交互：任务行右侧“已完成/撤销完成”文字按钮改为小方框，未完成为空、提交成功后显示对勾，再点撤销；心愿同样用方框划掉。复用受控已提交状态及原有父任务确认，不使用input自动勾选，取消/失败保持原状态；收起小窗仍无操作控件。明暗主题使用原token。
- 新建日期只允许本地今天及以后，覆盖安排、截止、指定提醒和重复结束日期；前端日期控件/保存校验加Rust用户创建IPC校验，保存时重读当天以处理跨午夜。月历过去日期禁用新增，仍可查看；历史任务编辑、内部重复生成、数据恢复不受影响，无迁移/数据清理。
- 主要文件：TaskRow.vue、AppIcon.vue、tasks.css、domain/tasks.ts、DetailDrawer.vue、ScheduleEditor.vue、QuickEntryWindow.vue、CalendarView.vue、task_commands.rs、tasks/validation.rs、既有测试源码及当前用户文档。TypeScript、Rust all-targets静态检查通过；回归用例仅编译，不运行测试或软件。Rust检查日志为artifacts/date-checkbox-check.log。日期控件/键盘/勾选交互仍待原生体验确认。 本轮 `npm run package:windows -- --ci` 成功，日志为 artifacts/date-checkbox-build.log；EXE与NSIS交付副本已更新并核对EXE哈希，版本仍1.0.0、未签名；九份迁移不变。

- 新增超时任务展示：截止精确时刻/纯日期本地日结束/无截止且计划日已过，未完成项红色标记并优先显示在主窗口今天与悬浮窗今日。未来安排但已过截止的任务移入今日；完成或延期后重新判定。收起小窗显示红色“超时 · 标题”，自适应文字测量包含标记，展开保留完成项的撤销功能。普通无日期心愿不变，旧版带截止的无日期记录超时后额外出现在今日。
- 超时属于派生界面状态，继续使用 pending/completed，不改数据库、任务日期、迁移、IPC、capabilities、提醒/重复引擎。复用 Pinia.now，并用下一截止/午夜单一计时器加原30秒/聚焦校时，不增加持续写库；计时器随dispose清理。搜索/日历标记显式使用传入时钟。
- 主要文件：domain/tasks.ts、stores/tasks.ts、TaskRow.vue、WidgetWindow.vue、MainWindow.vue、SearchView.vue、CalendarView.vue、tasks.css、task-domain/task-store/task-interactions测试源码及当前用户文档。新增截止边界、排序、完成/延期/删除、临时无写库刷新、小窗简洁标记用例；仅类型检查，未执行自动化测试或启动软件。TypeScript与格式检查通过，后台节流、休眠、多窗口实际更新时间待原生体验确认。 `npm run package:windows -- --ci` 已成功生成 Release EXE 与 NSIS，日志为 artifacts/timeout-build.log。九份迁移、版本、依赖锁及原生配置与上一交付一致；未运行测试或 GUI。

- 最新删除规则（取代下方历史软删除/回收站描述）：主窗口移除“已删除”入口；列表和详情共用 DeleteTaskDialog，确认后直接永久删除。提示含任务标题、不可恢复及重复实例范围，有草稿时另行提醒；失败保留确认框和草稿。旧版软删除记录不清理，既有字段和九份迁移不变。
- 删除服务沿用 purge_task 的确认参数、UUID/乐观锁、事务、revision 事件、重复排除日期和提醒去重账本，仅取消必须先软删除的前置条件。移除 set_task_deleted 的 handler、AppManifest、权限和前端服务；内部旧数据兼容测试及重复系列替换逻辑保留。不新增数据库结构或前端权限。
- 主要文件：MainWindow.vue、TaskList.vue、TaskRow.vue、DetailDrawer.vue、新增 DeleteTaskDialog.vue、stores/tasks.ts、services/tasks.ts、tasks.css、tasks/repository.rs、task_commands.rs、lib.rs、build.rs、tasks.toml、现有前后端测试源码与当前使用文档。功能总览同步为11模块60项。
- 本轮 TypeScript 检查、Rust all-targets 静态编译、Rust 格式检查和受影响文件 Prettier 检查通过。新增确认/取消、删除失败/草稿保留、版本冲突、旧软删除数据保留及事务回滚用例，仅编译未执行。未启动应用、运行 npm test/cargo test 或访问真实数据库。原生交互与删除故障回滚仍待验证。Rust 检查日志为 artifacts/direct-delete-check.log。 `npm run package:windows -- --ci` 本轮成功生成 Release EXE 和 NSIS，日志见 artifacts/direct-delete-build.log；主程序/安装包交付副本均更新并核对 EXE 哈希，仍为1.0.0、未签名。

- 按用户最终要求，仅移除主窗口左侧“快速输入”按钮；保留快速输入窗口、Ctrl+Alt+T、托盘快速添加、悬浮窗快速添加及全部原有设置/权限/数据库结构。此次源码业务变更仅 MainWindow.vue 的一个按钮，其余为 README 与交付记录；此前尚未完成的整体移除改动已撤回。与上次源码包逐文件比对确认只有此按钮及两份说明改变。TypeScript、Prettier 和 Release/NSIS 构建通过，未运行自动化测试或启动软件。

- 修复深色页面仍显示浅色原生标题栏：新增 desktop/theme.rs，依据 SQLite 的 system/light/dark 偏好调用锁定 Tauri 2.12.5 的 `WebviewWindow::set_theme`。在初始化显示前、主题事务提交后、恢复设置后同步三个现有窗口；system 使用 None 恢复系统跟随，与设置/恢复共用 runtime gate 避免乱序。失败保留已提交设置并显示运行状态警告。不增加 IPC 或 capabilities。
- 修复左侧顶部裁切：品牌与底部主题区不收缩，导航单独滚动；主布局限制在客户区内，适配小高度窗口并保持悬浮窗既有尺寸逻辑。主要文件：desktop/theme.rs、desktop/mod.rs、desktop/runtime.rs、commands.rs、MainWindow.vue、app.css、release.css、README、architecture、installation、本报告。
- 本轮 `npm run typecheck`、`cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked`、Rust 格式检查及受影响文件 Prettier 检查通过；`npm run package:windows -- --ci` 完成 Release 和 NSIS。9份迁移哈希不变。日志为 artifacts/theme-check.log、artifacts/theme-build.log。未运行自动化测试、应用、安装器或 GUI；原生标题栏实际颜色、系统跟随和不同 DPI 下的可见区域仍待体验确认。

- 修复快捷键冲突阻止显示小组件：显隐不再调用完整设置保存/注册流程，改为独立更新widget_visible的事务；不覆盖其他设置，重复显隐不增加revision，失败补偿实际窗口可见性。完整设置保存只在配置快捷键发生改变时注册，保留不可用键的真实状态和警告；不会注销仍在使用的同一有效键。修正冲突文案，不再声称没有的有效键已保留。
- 本轮 cargo check --all-targets --locked 通过，包括新增的无效快捷键不应被无关保存重试、显隐字段独立持久化回归用例的编译；未运行cargo test、npm test或WindowsGUI验收，没有访问真实用户数据库。Schema和迁移保持不变。

- 收起悬浮窗的任务之间增加使用主题边框色的1px分隔线，最后一项不加底线；自适应测量沿用真实DOM高度，自动包含分隔线。不修改数据和操作行为。

- 修复自适应小窗裁掉展开按钮的根因：原 html/body/#app 全局 min-width=260px，在原生窗口缩到120px后仍保持260px页面宽度。App按窗口标签标记根元素，悬浮窗单独解除最小页面宽度；标题栏采用三列Grid，为展开按钮保留独立36px位置。测量等待基础服务就绪，并计入错误提示等外层高度，避免内容挤掉标题栏。
- 本轮TypeScript及正式构建通过。独立JSDOM CSS级联核对确认修复前三个根节点最小宽度均260px，修复后均0px，标题栏为Grid且按钮在第3列。证据为 artifacts/widget-css-check-result.json。尝试隔离无界面Edge检查未返回DOM结果，两次均未记为通过；该CSS核对不等同原生窗口或真实浏览器像素布局验收，未执行npm test/cargo test或启动TaskDock。

- 收起窗口改为内容自适应，取消固定220宽与固定行高公式：前端测量真实字体文字和DOM高度，监听列表尺寸/字体加载变化，合并并去重尺寸请求；后端校验宽高并按目标屏幕工作区和DPI限制、调整位置。保持展开尺寸与几何持久化语义。静态检查通过，未进行原生GUI验收。

- 收起模式实际调用Tauri原生尺寸API：按DOM实际文字及内容尺寸自动调整宽高，长标题换行，受屏幕工作区及120～480宽/44～600高逻辑像素范围约束，超出时内部滚动，隐藏完成/撤销按钮及已完成项。展开恢复原尺寸与260×220～600×800限制。新增 set_widget_layout 仅widget可调用，参数有限值校验；几何保存/工作区恢复识别紧凑状态，继续持久化展开尺寸。Rust/TS/格式静态检查通过，真实尺寸及混合DPI行为仍待用户运行确认。

- 悬浮窗新增展开/收起，默认缩小窗口，只显示今日未完成任务标题；展开才显示日期、页签、进度和快速添加。收起始终回到今日，展开时恢复原尺寸和缩放能力，保留同步机制；复用列表 minimal 展示，新增仅限 widget 的受限尺寸 IPC，不改数据库。

- 悬浮窗增加“今日 / 未来事项”两个紧凑页签，复用任务过滤及 revision 同步；保留今日进度，未来页显示数量，支持键盘切换，不增加 IPC 或数据库字段。

- 按用户要求移除操作历史页面、导航、详情入口及对应 IPC、权限、前后端查询/展示代码；保留数据库审计写入和已有数据，备份/恢复/导出兼容不变。功能清单更新为 63 项。Rust 静态检查及格式检查通过，本轮不运行测试。

- 未来任务页签统一命名“未来事项”；主窗口空列表移除“添加第一项任务”及重复添加按钮，仅保留页面上方新建入口。

- “任务”统一入口下设今天/未来事项页签，共用添加入口；心愿单不设固定日期，划掉后保留记录并可撤销，不进入有日期任务的完成列表。复用旧字段和数据，详见 [任务与心愿单](task-workspace.md)。

- 任务列表和悬浮窗取消任务前方完成勾选框，使用右侧“已完成”按钮；已完成项显示“撤销完成”。保留父任务完成确认、保存失败状态和子任务勾选；相应交互测试源码已调整，本轮未运行。

- 保留任务四视图、分类/子任务、三窗口、托盘/快捷键/自启动、提醒/重复、备份恢复/导出、月历/搜索/历史及主题；未另建服务、数据库或状态系统。
- 静态分析确认28个前端IPC命令与Rust handler/build白名单对应，四份窗口capability维持最小权限；没有新增任意SQL、fs、shell或事件发送能力。
- 设置按“外观与桌面 / 数据管理 / 关于与运行状态”分组；修复隐藏设置页参与焦点循环的问题，键盘焦点计算不依赖布局矩形。
- 统一按钮换行、表单宽度、弹窗滚动/标题层级、紧凑导航与日历小尺寸布局。沿用原明暗CSS tokens；没有执行可视化验收。
- 悬浮拖动/缩放保留同步受限Rust命令，改为原生鼠标按下路径，限制左键、排除交互控件、阻止缩放事件冒泡并增大手柄。原交互测试事件同步为mousedown但未运行；真实拖动缩放仍待体验确认。
- 名称/版本统一TaskDock 1.0.0；完善16–256多尺寸ICO、PNG及NSIS头图/侧图，保留勾选标识并提高抗锯齿质量，源码生成脚本可复建。

## 安装与数据

启用Tauri NSIS，`currentUser`/HKCU、简体中文/英文、LZMA，默认安装到 `%LOCALAPPDATA%\TaskDock`。应用数据标识保持 `com.taskdock.desktop`，数据库及backups位于独立的本地应用数据目录，不放入安装资源。

使用 `embedBootstrapper`：内嵌微软WebView2引导程序，缺少运行时时仍需联网。本次只下载并检查其微软签名为Valid，没有运行。TaskDock主程序和安装包的Authenticode均为 **NotSigned**。

已检查实际生成的 `src-tauri/target/release/nsis/x64/installer.nsi`：currentUser分支为RequestExecutionLevel user；数据删除仅在用户勾选DeleteAppDataCheckbox且非升级时执行，默认不勾选、升级不清理。普通卸载移除启动项；安装hook仅在已有启动项时改写安装路径，不创建未开启的启动项；完成页运行默认不勾选。安装/升级/卸载实际行为未运行验证，保留数据时不要勾选清除选项。

对阶段开始记录的9个既有迁移文件（5个正向及4个下行）做SHA256比较，全部未改动；本轮另增0006字号迁移，不修改旧迁移。首次由用户运行新版时仍由既有事务迁移升级，不删库或重建。

## 实际命令与结果

原生命令前加载 `. .\scripts\Use-LocalToolchain.ps1`。

| 命令/检查                                                                 | 结果                                                                               |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `npm run typecheck`                                                       | 通过；最终生产构建再次执行通过                                                     |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked` | 通过，未执行测试                                                                   |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`               | 通过                                                                               |
| Prettier检查受影响源码/配置/文档                                          | 通过                                                                               |
| `npm run package:windows -- --ci`                                         | 最终通过：Vite生产构建、Rust优化Release、makensis生成x64安装包                     |
| PE/版本/签名只读检查                                                      | x64 Windows GUI，产品版本1.0.0；主程序与安装包NotSigned；导入表无额外MSVC运行库DLL |
| IPC/capability/迁移静态审查                                               | 28命令、4配置、9迁移文件核对通过                                                   |

未遇到源码编译错误。首次打包在官方NSIS下载阶段报 `timeout: global`，不是完成产物。随后通过PowerShell取得相同官方NSIS3.13与插件0.5.3，核对官方源码公布的ZIP/编译器/插件哈希，微软引导程序核验签名；启用 `useLocalToolsDir` 在 `src-tauri/target/.tauri` 缓存，重新完整构建成功。最后焦点修复也已重新完整构建，交付的是最终版本。

最终日志：`artifacts/stage-6/package-windows-final.log`；静态核对及产物记录同目录。没有运行npm test、cargo test、应用/安装器、GUI、性能测试或恢复故障注入。

## 产物

本轮原生 Release EXE 与 NSIS 安装包构建成功；EXE与安装包交付副本均已更新，并与原生构建文件核对SHA256一致。没有启动应用、安装或强制结束用户进程。

| 文件              | 实际位置                                                                                             |
| ----------------- | ---------------------------------------------------------------------------------------------------- |
| 原生Release主程序 | `F:\tododock\src-tauri\target\release\taskdock.exe`                                                  |
| 原生NSIS安装包    | `F:\tododock\src-tauri\target\release\bundle\nsis\TaskDock_1.0.0_x64-setup.exe`                      |
| 便于交付的副本    | `F:\tododock\artifacts\release\TaskDock.exe` 与 `TaskDock_1.0.0_x64-setup.exe`                       |
| 完整源码包        | `F:\tododock\artifacts\release\TaskDock-1.0.0-source.zip`（不含工具缓存、构建目录、运行数据库/备份） |
| 校验清单          | `F:\tododock\artifacts\release\SHA256SUMS.txt`                                                       |
| 安装使用说明      | `docs/installation.md`；备份/恢复说明见`docs/data-management.md`                                     |

主程序18,887,680字节，安装包6,569,285字节。本轮交付主程序与更新安装包的最终SHA256：

```text
0E98F52E0BB2D770E2876FEC0A814CDE6B91106E042DEC3E8BF36D9CC34AFA6A  TaskDock.exe
33090D5993DF55F9DB444A17B649E165509E3478324DF6065B8C628C45DA9761  TaskDock_1.0.0_x64-setup.exe
```

主要修改：版本/锁文件、tauri.conf.json、installer/hooks.nsh、图标及生成脚本、source打包脚本、SettingsPanel/ThemeSelect/DialogFrame/WidgetWindow/MainWindow、release.css和入口、安装及相关开发文档。既有业务数据与已应用迁移不改；新增0006字号迁移。

## 尚未验证与手动建议

所有第四/第五阶段业务及本轮视觉修改尚未运行验收；特别是备份恢复的Windows锁、断电/失败回退、通知及重复跨时区/休眠、混合DPI/多屏、单实例与托盘、安装升级卸载/启动项。构建成功不代表这些功能已稳定可靠，数据恢复应先在隔离数据中演练。

建议用户后续先安装并体验三窗口、任务同步和主题，再检查快捷键/拖动/托盘退出、安装升级及默认卸载保留，最后安排隔离数据的提醒、重复、备份恢复和大数据搜索验证。应用未签名，缺少WebView2时联网安装及企业策略仍是部署限制。

**第六阶段交付后停止。没有自动安装、启动软件、执行GUI验收或清理用户相关文件。**

## 2026-11-03：悬浮窗字号

- 设置新增10–20字号滑块、数值显示及默认字号按钮；默认13，点击保存桌面设置后生效。沿用共享设置事务、乐观锁及revision事件，保存失败保留输入。
- 展开和收起模式均调整任务字号，日期较小、周标题较大；布局顺序、周分组、原位勾选与删除线不变。保存字号时触发DOM重新测量，自适应窗口尺寸并保留既有最大尺寸与内部滚动。主窗口和按钮字号不变。
- 主要源码：desktop-types.ts、SettingsPanel.vue、WidgetWindow.vue、desktop.css、desktop/settings.rs、db.rs；新增0006_widget_typography.sql及开发下行脚本。Schema6新增字号字段，并原样保留备份目录记录、扩展版本约束至6；0001–0005未修改。应用版本仍为1.0.0。
- 实际静态检查：npm run typecheck、cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked、Rust格式及受影响文件Prettier检查通过；npm run package:windows -- --ci完成Release/NSIS，交付副本均核对SHA256一致；未运行自动化测试、应用或GUI，未读取真实用户库。构建及最终产物信息见上文。字号实际显示、混合DPI、Schema6迁移与备份恢复仍待运行验证。

## 2026-11-03：更窄小组件、收起页签及独立字号

- 展开默认宽200→180，最小宽180→160，保留手动缩放；旧默认宽映射到180，其他自选宽度保留。收起宽度按内容120–180，普通最大高度280，展开320，超出内部滚动。
- 展开及收起都有“今日 / 未来”页签，切换展开不重置所选视图；未来继续按周分组。收起任务默认14px，已完成原位保留删除线，不增加空白提示或统计。
- 软件设置分别调整展开任务、收起任务、任务日期/时间、周标题、周日期范围，默认13/14/10/15/10px；沿用共享设置服务、失败草稿和revision同步。新迁移0007增加4个字号字段、扩展备份版本范围至7，保留任务、历史及备份目录记录；0001–0006未改。
- 主要文件：WidgetWindow.vue、desktop.css、desktop-types.ts、SettingsPanel.vue、desktop/settings.rs、desktop/runtime.rs、desktop/geometry.rs、db.rs、tauri.conf.json、0007迁移及下行脚本；同步更新已有几何用例的默认宽度断言（未运行测试）与README/架构/数据模型/使用说明。
- 本轮npm run typecheck、cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked、cargo fmt --check和受影响文件Prettier检查通过；npm run package:windows -- --ci成功，日志artifacts/widget-compact-tabs-build.log。与上次源码包比对确认11份既有迁移原文未变。用户从托盘退出后，EXE与安装包交付副本均已更新并核对SHA256一致。本轮不运行测试、软件、安装器或GUI，不打开真实用户数据库。实际窄窗文字显示、混合DPI、迁移与备份恢复仍待运行验证。应用版本保持1.0.0。

## 2026-11-03：完成任务方框缩小

- 完成方框视觉尺寸由18×18缩为14×14逻辑像素，圆角3px，对勾14→10px，继续居中；28×28点击区域、位置和与删除按钮的对齐不变，悬停配色规则不变。
- 仅修改tasks.css、TaskRow.vue和本报告；不改任务数据、字号设置、数据库迁移、权限或1.0.0版本。受影响源码Prettier检查通过。npm run package:windows -- --ci通过（含TypeScript检查），日志artifacts/completion-small-build.log；用户从托盘退出后，EXE与安装包交付副本均已更新并核对SHA256一致。
- 未运行自动化测试、软件或GUI，实际小框显示待用户体验确认。

## 2026-11-03：同一任务定期提醒与截止布局

- 提醒新增独立定期设置：指定首次时间，自定义正整数分钟/小时/天，预设每30分钟、每小时、每6小时、每天、每周。与生成新任务的“任务重复周期”分开；首次时间及截止关系、间隔范围前后端校验。
- 完成、删除、取消所有提醒、截止后停止；撤销完成恢复周期。重启/唤醒只补发最近一次，截止后不补发，提醒中心保留最新周期项。保留原通知通道、稍后提醒、恢复去重与合并机制。
- 截止方式及对应日期/时间移到分类上方，标明安排日期为准备哪天做、截止为可选最晚完成期限。
- 主要文件：ScheduleEditor.vue、DetailDrawer.vue、schedule-types.ts、domain/tasks.ts、schedule.css、schedule/model.rs、calendar.rs、repository.rs、mod.rs、新增periodic.rs、tasks/repository.rs；同步README、安装/架构/数据说明。Schema7及13份既有迁移不变，无新IPC/权限/依赖，应用版本仍1.0.0。
- 静态编译中发现SQLite参数usize不受支持，已改为有界字符串长度的i64参数。TypeScript、Rust all-targets静态编译、Rust格式及受影响文件Prettier检查通过；npm run package:windows -- --ci完成Release和NSIS，日志artifacts/periodic-reminders-build.log。EXE/安装包交付副本均已更新并核对SHA256一致，依赖/版本/权限及13份旧迁移原文未变。本轮未运行测试、软件、GUI或实际提醒验收，不访问真实用户库。跨休眠/时区、长期间隔、恢复去重与通知投递仍待运行验证。

## 2026-11-03：安排日期与截止日期合并

- 普通任务仅保留“完成日期”及“具体时间（可选）”，移除独立截止方式和第二日期；不填具体时间按当天结束前完成。快速输入同步改称完成日期；重复的同日日期元信息不再显示两次。
- 截止/提前提醒与定期提醒停止时间使用完成日期，全天按本地23:59提醒，跨日未完成为超时。按实际完成日期变更平移关联首次提醒；普通周期不能晚于完成截止开始。
- 旧任务只改标题等内容时保留原计划/截止字段；显式改日期/时间才统一，精确旧时间不因仅改标题丢失秒/时区信息。心愿保持无日期。恢复仍接受旧无显式截止的周期配置，不满足新截止的旧周期安全停止，不阻塞其他提醒。
- 主要文件：DetailDrawer.vue、ScheduleEditor.vue、TaskRow.vue、QuickEntryWindow.vue、domain/tasks.ts、schedule/calendar.rs、repository.rs、periodic.rs、mod.rs、tasks/repository.rs及README/使用/架构/数据说明。Schema7、13份旧迁移、版本1.0.0、依赖和权限不变，无真实用户库操作。
- TypeScript、Rust all-targets静态编译、Rust格式及受影响文件Prettier检查通过；npm run package:windows -- --ci完成Release/NSIS，日志artifacts/completion-date-build.log。EXE与安装包交付副本均已更新，并核对SHA256一致。本轮未运行测试、软件、GUI、提醒或恢复验收；旧独立日期显示与周/月视图、日期改动后锚点同步、跨时区及截止边界仍待运行确认。

## 2026-11-03：截止日期命名与原截止方式恢复

- 日期名称改为“截止日期”，恢复截图中的“截止方式（最晚完成，可不设）”及无截止/仅日期/精确时间选项，仍在分类上方。默认新普通任务仅日期；复用上方截止日期，不增加第二个仅日期控件。
- 精确时间恢复datetime-local日期时间选择器，上方日期与完整时刻的本地日期双向同步；不使用单独type=time输入。旧精确时间在日期/方式/时间不变时保留原UTC值，失败/冲突继续保留草稿。
- 无截止沿用原基础日期规则，不设置额外显式截止或截止/提前提醒；定期提醒及原后端日期fallback不改。快速输入改称截止日期并默认写同日due_date，任务列表避免重复日期元信息。
- 主要文件：DetailDrawer.vue、TaskRow.vue、ScheduleEditor.vue、QuickEntryWindow.vue及README/使用/架构/数据说明。Rust源码、Schema7、13份迁移、依赖、IPC/权限及版本1.0.0保持不变。
- TypeScript及受影响文件Prettier检查通过，npm run package:windows -- --ci完成Release/NSIS，日志artifacts/deadline-selector-build.log。原截止提醒配置兼容识别为仅日期；与前包核对98份Tauri/Rust/配置/迁移文件完全不变，本轮未额外重跑Rust检查。用户从托盘退出后，EXE与安装包交付副本均已更新并核对SHA256一致。未运行自动化测试、软件、安装器或GUI，不读取或修改真实用户数据库；日期同步与原生时间选择器显示待用户运行确认。

## 2026-11-03：悬浮窗隐藏任务栏入口

- widget窗口配置skipTaskbar=true，不再单独占用Windows任务栏按钮，适用于“不合并任务栏按钮”的使用方式；主窗口、快速输入及系统托盘行为不变。仍通过现有from_config创建唯一悬浮窗，显示/隐藏、展开/收起不重建窗口。
- 核对当前Tauri CLI 2.12.4所带官方config.schema.json：skipTaskbar为布尔值，在Windows/Linux隐藏窗口任务栏图标；不是Tauri v1 API。透明度实现保留原扩展窗口样式，不覆盖此设置，无新IPC或权限。
- 主要修改tauri.conf.json的widget单项配置和本报告/使用说明，版本1.0.0及Schema7、旧迁移、任务/提醒/字号/窗口布局均不改。受影响文件Prettier及npm run package:windows -- --ci通过（含TypeScript），日志artifacts/widget-taskbar-build.log；用户退出后EXE和NSIS交付副本均已更新、SHA256与原生产物一致。未执行测试或GUI，不启动应用/安装器，不访问真实用户数据库；Windows不合并按钮时的实际显示仍待用户确认。

## 2026-11-03：任务数据跟随软件目录

- 上一项widget.skipTaskbar已构建交付。本轮任务数据库/设置/备份改为EXE旁data；日志和WebView缓存改为EXE旁logs/webview；数据管理页显示实际目录。必须将软件安装/放置到D、F等非C盘可写目录，若软件仍在C盘，数据也在C盘；安装目录页增加说明，不静默改换路径。
- 首次迁入旧AppData库采用只读源、SQLite Online Backup、同卷暂存、仅副本升级、完整性/外键/任务设置调度校验、备份逐文件哈希/刷盘、关闭WAL后目录rename发布。已有目标库优先；旧恢复未完成、缺库但有备份/附属文件、非空无库的目标、链接/重解析点等会拒绝，原文件和失败暂存保留，不回退C盘或空库。初始化失败缓存至重启，避免后台重复搬运。
- 已开启的自启动入口在新位置刷新到当前EXE，原未开启的不注册。安装/更新不包含用户data资源；NSIS普通卸载非递归移除目录，非空data保留。移动设备/目录需退出后复制EXE及完整data，不再只复制EXE即可携带任务。
- 主要文件：新增storage.rs；lib.rs、service.rs、commands.rs、data/mod.rs、data/model.rs、desktop/runtime.rs、DataManagement.vue、data-types.ts、data.css、installer/hooks.nsh、package-source.py、.gitignore及README/数据/架构/安装文档。无新增IPC/能力、依赖或Schema迁移，版本仍1.0.0、Schema7。
- TypeScript及Rust all-targets静态编译通过；新增3项数据迁入/原库保留/缺失与恢复保护回归用例，仅编译、未运行。Rust格式、受影响文件Prettier检查及npm run package:windows -- --ci通过，日志artifacts/portable-data-build.log；EXE和NSIS交付副本均更新并核对SHA256一致。13份迁移、依赖、能力和IPC白名单未改，widget.skipTaskbar仍为true；生成NSIS仍使用非递归RMDir保留非空data目录。没有运行软件、测试、安装器或访问真实用户库，未实际执行迁移。Windows锁/断电/磁盘满、目录权限、跨设备复制、自启动路径与卸载保留行为仍待验收；成功构建不代表迁移已验证。

## 2026-11-03：GitHub更新

- 对接用户指定公开仓库ikunyu/TaskDock。启动后台检查，主窗口提示、设置手动检查、进度、下载验证和用户确认安装；网络失败不影响任务。旧设备需先安装一次本版本，后续方可软件内更新。
- 版本继续1.0.0，内部构建202611030001。官方updater=2.14.1已核对官方文档和源码；远程版本1.0.0+build，数值比较防回退，requireSignedVersion验证包与构建号的受信任签名注释。WindowsAuthenticode仍未签名。
- 复用单一数据库生命周期；更新前保存窗口/关闭连接，退出时注销快捷键、清理Tauri资源，安装目录固定当前目录，不改动data/备份或Schema7。仅main增加4个受限命令及app-updates权限，无通用HTTP/文件系统/updater前端权限。
- 主要文件：updates.rs、lib.rs、desktop/runtime.rs与mod.rs、service.rs、build.rs、Cargo依赖/锁、配置/permission、updates Store、UpdatePanel、App/SettingsPanel、release.css；新增release-config.json、release-notes.md、构建/签名/验证/发布脚本及github-updates.md。源码打包排除.private，GitHub发布仅固定4个公开文件，不上传源码、任务库、备份或私钥。
- TypeScript和Rust all-targets静态检查通过，受影响文件Prettier、Rust格式、PowerShell/Python/Node发布脚本语法检查通过；scripts/Build-WindowsRelease.ps1完成Release/NSIS，签名阶段发现环境私钥参数与--private-key-path冲突，修正后单独重新签名并由node scripts/verify-update.mjs验证安装包签名及签名内构建号通过；已更新EXE、安装包、.sig和latest.json。13份旧迁移未改。未运行自动化测试、TaskDock、安装器、真实数据库或Windows更新GUI验收。下载→重启、更新失败恢复、草稿确认、代理/离线等运行行为仍待最终体验确认。

### GitHub首发实际结果

- 已发布：https://github.com/ikunyu/TaskDock/releases/tag/build-202611030001 ，非草稿、标记为最新。
- 公开端点：https://github.com/ikunyu/TaskDock/releases/latest/download/latest.json ，实际GET返回1.0.0+202611030001，与本地清单和签名一致。
- 公开下载回安装包并核对SHA256，与本地签名包一致。发布资产恰好latest.json、安装包、.sig、UPDATE-SHA256SUMS.txt；仓库仅初始README与这些Release资产，没有上传源码、任务数据、私钥或CLI凭据。
- 实际发布命令scripts/Publish-GitHubRelease.ps1成功；证据artifacts/github-publish.log、github-release.json、github-publish-verification.json，签名阶段修正记录github-updater-sign.log。应用Authenticode仍NotSigned，更新用Ed25519签名与构建号绑定已验证。
- 校验只读取安装包/JSON/签名，不执行TaskDock、安装器或业务用例；Windows下载→关闭数据库→替换→重启路径未作原生验收。首版的其他设备需先手动安装一次，后续才具备软件内检查能力。
