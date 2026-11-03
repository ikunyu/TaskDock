# 第三阶段验收报告

日期：2026-10-29（Asia/Shanghai）。版本：0.3.0，Schema 3。第三阶段桌面集成代码已交付，自动化检查通过，已列 Windows 11 原生检查通过；托盘菜单实点、拖动缩放及其他环境项目仍待人工验收，不能将本报告理解为全部平台验收通过。停止于第三阶段，不进入第四阶段。

## 功能完成情况

| 范围          | 实现                                                                                                                                         |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 托盘          | Rust TrayIconBuilder，主窗口/快速添加/widget显隐/设置/退出菜单，双击聚焦主窗口；关闭main默认隐藏，支持改为退出                               |
| 全局快捷键    | 默认Ctrl+Alt+T，后台隐藏时可用，Pressed触发已有快速窗，支持修改、严格校验和冲突回退；退出注销，无键盘钩子/输入记录                           |
| 悬浮窗        | 复用WidgetWindow/TaskList，标题文字和空白区拖动、右下角缩放，默认320×400、min260×220、max600×800；置顶默认关、60–100%整体alpha、显隐及持久化 |
| 多屏/窗口状态 | SQLite几何；物理位置与逻辑尺寸分离，当前工作区/缩放恢复，负坐标与离屏回退，750ms检查显示器配置变化；支持安全默认位置重置                     |
| 自启动        | 默认关，显式保存时通过用户级Run注册/移除；--background不弹main，按widget偏好恢复；无需管理员和后台服务                                       |
| 设置          | 统一SettingsPanel，主题立即保存；透明度/置顶/显隐/快捷键/自启动/关闭行为显式保存，运行实际状态与错误提示，失败保留输入                       |
| 生命周期      | single-instance首插件，二次启动激活已有main；固定三窗口不重复创建；退出等待活动数据库请求、保存几何、注销快捷键、checkpoint并关闭服务        |
| 原业务        | 四视图、任务/分类/子任务及审计继续复用，未另建任务服务、任务数据库或重复Pinia设置store                                                       |

Windows通知、提醒调度、重复引擎、自动备份恢复、月历高级搜索、完整历史界面、正式安装器/签名均未实现。

## 关键技术决策

1. 三个窗口仍在唯一LocalService注册后由配置创建，create=false。单实例插件最先运行，第二个进程在创建服务之前退出；已有实例只展示/聚焦main。
2. 桌面偏好扩展既有SettingsSnapshot/Pinia/app_settings.revision/settings-changed；task revision不因窗口设置变化而增加。RuntimeStatus仅反映OS资源，不代替SQLite偏好。
3. 窗口几何独立存表。x/y为物理桌面坐标，width/height为逻辑内容尺寸，保存scale供说明；恢复使用当前目标屏的work_area/scale重新约束。主屏安全回退，不复用全局统一缩放。仅几何变化时写入，避免拖动刷任务事件。
4. ShortcutLease先注册候选，保留旧键直到数据库提交；失败释放候选。成功切换有效键后清理旧注册；handler只接受effectiveShortcut，旧键清理失败也不能触发业务。测试夹具仅占Ctrl+Alt+Y，不监听输入。
5. 启动项、窗口属性和SQLite不能跨系统原子提交。桌面设置串行检查expectedRevision，原生应用失败/数据库失败执行补偿；保留草稿并显示实际状态，补偿失败另行告警。启动本身不会因保存偏好而自动重新注册启动项。
6. 前端没有global-shortcut/autostart/window-default权限。main仅新增受限桌面设置/重置/退出，widget仅新增本窗口拖动/缩放；隐藏走带显隐持久化的受限命令，快速窗保持原权限。无任意路径/SQL/事件发送权限。
7. 指针拖动/缩放使用同步Rust命令，与Tauri原生窗口命令路径一致，避免异步调度错过mouse-down；输入控件和按钮不触发标题拖动。Windows整体alpha仅对本进程widget HWND使用Get/SetWindowLongW与SetLayeredWindowAttributes，安全范围60–100%。
8. LocalService生命周期读写锁让退出等待已开始请求完成，然后阻止重新打开连接并checkpoint。普通关闭隐藏不关闭服务；显式退出和关闭行为exit共用shutdown。
9. 主题立即保存会增加设置revision，因此设置面板有未提交桌面输入时需要重新载入再保存；这是一致性保护，不会自动覆盖草稿。

## 数据库迁移

新增0003_desktop.sql：desktop_settings单行STRICT表、window_state STRICT表。默认widgetOpacity=100、topmost/visible/autostart=false、shortcut=Ctrl+Alt+T、closeBehavior=hide；约束范围与Rust校验一致。设置写入与app_settings.revision同事务；几何独立事务。任务表和旧迁移不改、不重建用户库。

SHA256基线核对0001_initial.sql、0002_tasks.sql均UNCHANGED。新增0003_desktop.down.sql只供开发显式事务下行，清除桌面偏好/几何并保留任务；无生产下行IPC。临时库覆盖Schema 2任务/主题升级保留、重复升级、失败DDL回滚、下行再升级。原Schema 2下行测试仍限制其测试范围为迁移1/2，没有删掉原断言。

本机已原地升级到Schema 3。重启前后只读记录显示3条任务、task revision=16、quick_check=ok、无外键异常，旧任务未重建。新建且保留一条“阶段三验收 · 同步冲突”测试任务，目前已完成，描述草稿未保存；原两条阶段二记录保留。未硬删除任务或审计。

## 环境与官方核对

沿用F:\tododock已有工程，无Git仓库，未重初始化或改动无关项目。实际Node24.16.0、npm11.13.0、Rust/Cargo1.99.1；Windows 11当前x64宿主，沿用项目本地MSVC/SDK脚本。Tauri Rust2.12.5、JS API/CLI2.12.4、tauri-runtime2.12.2；新增global-shortcut2.4.1、autostart2.7.0、single-instance2.5.5和tray-icon feature。锁文件记录实际传递依赖。

2026-10-29通过HTTPS实际读取Tauri v2托盘、全局快捷键、自启动和单实例四个官方页面，均HTTP200，并读实际安装源码核对Monitor::work_area、底层Window::start_resize_dragging。未使用v1API；Tauri没有set_opacity，已明确使用Windows专属实现。记录见references/tauri-v2.md及artifacts/stage-3/official-docs.jsonl。

## 实际命令与结果

原生命令前执行 `. .\scripts\Use-LocalToolchain.ps1`，测试数据库均为tempfile，UI失败模拟采用typed bridge，不破坏真实库。

| 命令                                                                                    | 实际最终结果                                    | 证据（artifacts/stage-3）                             |
| --------------------------------------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------- |
| npm run typecheck                                                                       | 通过，Release内再次执行通过                     | 构建日志及工具输出                                    |
| npm test                                                                                | 9文件、67项通过、0失败（保留原58项）            | frontend-tests.log                                    |
| npm run format / npm run format:check                                                   | 通过                                            | format-check.log                                      |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check                               | 通过                                            | 工具执行输出                                          |
| cargo test --manifest-path src-tauri/Cargo.toml --locked                                | 34项通过、0失败（保留原21项）；main/doc无测试项 | rust-tests.log                                        |
| cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings | 通过                                            | clippy.log                                            |
| npm run tauri -- build --no-bundle                                                      | 通过，Windows Release，无安装器                 | release-build-final.log                               |
| rustc scripts/hotkey-blocker.rs -o artifacts/stage-3/hotkey-blocker.exe                 | 通过，临时Ctrl+Alt+Y占键测试后RELEASED          | 工具输出                                              |
| TaskDock exe --background                                                               | 实际启动，无可见TaskDock窗口，进程存活          | 原生观察及进程检查                                    |
| Start-Process exe（二次启动）                                                           | 原main恢复，短暂第二进程退出，最终仅一个PID     | 原生窗口和进程检查                                    |
| 只读sqlite3 mode=ro                                                                     | Schema 1/2/3、完整性ok、重启几何/偏好及任务一致 | native-before-restart.json、native-after-restart.json |

新增Rust覆盖几何负坐标/换DPI/断屏回退/缩小工作区/NaN、偏好约束/安全回退/版本冲突、快捷键候选冲突和提交失败释放、Schema3升级/下行/故障、退出等待活动请求及禁止重开。新增前端覆盖设置失败保留输入、显式保存、自启动默认关、外部revision保草稿、重置退出路由、指针操作边界和最小权限。

修复过的失败：初始误用WebviewWindow未提供的resize方法，核对底层Window API后修正；Clippy类型复杂度抽为别名；测试日志bridge mock需返回Promise；jsdom PointerEvent触发器只读button问题改为原生MouseEvent分发。最终对应测试全通过，未把这些中间失败计作通过。

设置页原成功提示会停留到新编辑，已增加同步草稿watch清除并扩展交互回归。主题label不再窄列换行，设置与侧栏外观使用不同id。退出补充等待活动请求的回归，避免checkpoint前仍有已开始写入。

## Windows 11原生检查

使用computer-use技能的Sky操作实际release/WebView2；只检查本阶段相关及遗留场景，没有重跑全部第二阶段人工项。

| 检查                 | 实际结果                                                                                                                                                                 |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Schema3启动/旧数据   | 0.3.0显示Schema3，旧收集箱及已完成记录保留                                                                                                                               |
| 默认关闭main隐藏     | Alt+F4后main不在可见窗口清单，widget及进程继续运行，通过                                                                                                                 |
| 隐藏后Ctrl+Alt+T     | 在widget活动/main隐藏时打开原quick-entry，直接输入成功，通过                                                                                                             |
| Esc/重复窗口         | Esc隐藏quick不退出；再次热键使用同一窗口句柄，通过                                                                                                                       |
| 原生快捷键冲突       | 临时夹具占Ctrl+Alt+Y，设置保存返回SHORTCUT_CONFLICT，候选输入保留、有效键仍Ctrl+Alt+T且能打开quick，通过；夹具已释放                                                     |
| 透明度/置顶/显隐     | 设置60%+topmost+visible保存成功，widget整体半透明并覆盖主窗；最终100%/非置顶/隐藏恢复默认，通过                                                                          |
| 自启动注册/关闭      | 勾选并保存后HKCU TaskDock值指向本exe并带--background；关闭保存后该值不存在，通过；未注销/登录Windows                                                                     |
| 单实例               | 二次Start-Process激活已有main，最终仅一个taskdock.exe，句柄不重复，通过                                                                                                  |
| 显式退出             | 软件设置退出后所有TaskDock窗口消失，进程数0，通过；托盘退出菜单实点仍待验收                                                                                              |
| 最终后台启动         | --background运行时无可见main/widget（保存visible=false），进程存活，通过                                                                                                 |
| 状态恢复             | 最终重启前后desktop偏好和三窗口几何逐项相同，task_count/revision保持3/16，通过                                                                                           |
| 位置重置             | 点击恢复默认位置，main回到当前主屏安全位置（截图origin399,106），显示成功；不改任务，通过                                                                                |
| 明亮/深色            | 最终设置界面两种主题实际可读，主题选择同步侧栏；已恢复原dark偏好，通过                                                                                                   |
| 悬浮窗隐藏按钮       | 最终×按钮隐藏widget且main存活，通过                                                                                                                                      |
| 第二阶段遗留冲突     | 主窗未保存描述，widget完成同任务；返回main草稿完整保留、提示其他窗口修改、保存禁用；重新载入后版本2、数据库描述为空，通过                                                |
| 标题拖动/右下角缩放  | 注入drag后未观察到位移/尺寸变化；一次向窗外缩放被工具边界拒绝，改用窗内目标仍未观察到改变。同步命令/手柄已实现且前端边界测试通过，但原生鼠标实操仍待人工验收，不宣称通过 |
| 托盘菜单/双击        | 托盘已由Rust创建且后台驻留运行；工具可选目标未包含任务栏，未实点各菜单或双击，保持待验收                                                                                 |
| Win10/其他DPI/热插拔 | 无对应环境实际操作；纯几何单测不替代真机，待验收                                                                                                                         |

本轮一次main隐藏后工具报window is not a usable app window，重新选择并通过二次启动激活恢复；一次顶层widget遮挡主窗按钮导致工具拒绝，隐藏widget后复测成功。这些工具结果没有直接归为应用故障，也没有无限重试。

证据截图：shortcut-conflict.jpg、widget-alpha-topmost.jpg、task-conflict-draft.jpg、final-settings-light.jpg、final-reset-position.jpg。前两项来自业务验收构建，最终截图对应当前错误提示/布局修复后构建。没有虚构性能、Windows 10或多屏结果。

## 遗留风险与后续建议

- 必须按stage-3-manual-checklist.md补测托盘各菜单/双击/托盘退出、真实鼠标拖动缩放与最小布局、快捷键成功换键、关闭行为exit及真实Windows登录。当前自动化工具观察不足，不能宣称全部原生项已验收。
- 多屏插拔、分辨率变化、混合DPI、Windows10需实际设备；几何逻辑已测试，但Windows窗口最小尺寸、边框、极小工作区等行为仍需实测。
- 没有安装器，用户移动exe会使原启动项路径过期，需关闭再开启自启动。OS系统策略也可能禁用启动项，应以实际状态为准。
- OS与SQLite跨系统副作用只有补偿，进程强制终止/补偿失败时仍可能不一致；运行状态提示可帮助纠正。未做断电故障注入。
- 750ms工作区/几何检查及现有全量任务快照尚无资源/性能基准；不承诺长期内存、启动时间或任务容量。
- 仍无备份/正式恢复/系统通知/重复/历史浏览/安装升级签名，均保持后续阶段边界。

本轮结束时：测试热键占用已释放、TaskDock进程0、自启动注册不存在；widget100%、不置顶、隐藏、Ctrl+Alt+T、关闭行为hide、theme=dark。阶段三测试任务保留为completed，可正常撤销或软删除。第三阶段停止，下一阶段须用户另行授权。

## 交付产物

- src-tauri/target/release/taskdock.exe（未签名、无安装器）。
- SHA256：4DBF3DBA3F24B7A79B71C675AB2BEFC3D08DEA26B57A7141969B543D3BEB6FC5。
- docs/stage-3-manual-checklist.md、stage-3-plan.md及本报告；需求/架构/数据模型/测试/开发/README已更新。

## 实际修改文件

以下与阶段开始SHA256基线比较，不含.tools、node_modules、dist、target、gen、artifacts；自动生成的5个受限桌面命令权限文件计入。原阶段一/二报告保持历史结论，任务仓储和任务服务未重写。

### 新增

- `docs/stage-3-manual-checklist.md`
- `docs/stage-3-plan.md`
- `docs/stage-3-report.md`
- `scripts/hotkey-blocker.rs`
- `src-tauri/migrations/0003_desktop.down.sql`
- `src-tauri/migrations/0003_desktop.sql`
- `src-tauri/permissions/autogenerated/desktop_status.toml`
- `src-tauri/permissions/autogenerated/exit_app.toml`
- `src-tauri/permissions/autogenerated/reset_window_positions.toml`
- `src-tauri/permissions/autogenerated/save_desktop_settings.toml`
- `src-tauri/permissions/autogenerated/widget_window_action.toml`
- `src-tauri/permissions/desktop.toml`
- `src-tauri/src/desktop/commands.rs`
- `src-tauri/src/desktop/geometry.rs`
- `src-tauri/src/desktop/mod.rs`
- `src-tauri/src/desktop/native.rs`
- `src-tauri/src/desktop/runtime.rs`
- `src-tauri/src/desktop/settings.rs`
- `src-tauri/src/desktop/shortcut.rs`
- `src-tauri/src/desktop/tests.rs`
- `src/components/SettingsPanel.vue`
- `src/desktop-types.ts`
- `src/styles/desktop.css`
- `tests/desktop-settings.test.ts`
- `tests/widget-desktop.test.ts`

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
- `src-tauri/capabilities/shared.json`
- `src-tauri/capabilities/widget.json`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/src/commands.rs`
- `src-tauri/src/db.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/service.rs`
- `src-tauri/tauri.conf.json`
- `src/components/ThemeSelect.vue`
- `src/main.ts`
- `src/services/bridge.ts`
- `src/stores/app.ts`
- `src/types.ts`
- `src/windows/MainWindow.vue`
- `src/windows/QuickEntryWindow.vue`
- `src/windows/WidgetWindow.vue`
- `tests/security.test.ts`
