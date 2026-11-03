# Tauri v2 官方核对记录

核对日期：2026-10-28。通过 HTTPS 实际读取以下官方页面；依赖版本以 npm registry / crates.io 及最终 lockfile 为准，不混用 v1 API。

- https://v2.tauri.app/start/prerequisites/ ：Windows 需要 MSVC C++ 工具、WebView2 和 Rust MSVC。
- https://v2.tauri.app/develop/configuration-files/ ：v2 tauri.conf.json 与 capabilities 配置。
- https://v2.tauri.app/security/capabilities/ ：窗口级权限；自定义命令默认可调用，需 AppManifest::commands 纳入 ACL。
- https://v2.tauri.app/security/permissions/ ：自定义 permission 的 commands.allow，应用权限不带插件前缀。
- https://v2.tauri.app/reference/acl/core-permissions/ ：本项目仅授权 core:event:allow-listen / allow-unlisten。
- https://v2.tauri.app/develop/calling-rust/ ：`@tauri-apps/api/core` invoke，Rust command，参数/错误序列化。
- https://v2.tauri.app/develop/calling-frontend/ ：Rust Emitter，前端事件监听和清理。
- https://v2.tauri.app/reference/javascript/api/namespacewindow/ ：getCurrentWindow() 及逻辑/物理尺寸语义。
- https://v2.tauri.app/plugin/logging/ ：Rust 日志目标、文件大小与轮转配置；KeepSome 已同时核对实际插件源码。

使用 v2 import `@tauri-apps/api/core` / `window` / `event`；不使用 v1 `@tauri-apps/api/tauri`、allowlist 或旧 SQL 插件 API。SQLite 由 Rust rusqlite 管理，不给 WebView 暴露数据库插件。

窗口延迟创建同时核对了实际安装源码：tauri-utils 2.10.2 的 WindowConfig::create 与 tauri 2.12.5 的 WebviewWindowBuilder::from_config；在服务注册后创建窗口。

核对到 registry 版本：Tauri Rust 2.12.5；Tauri JS API/CLI 2.12.4；Vue 3.5.44；Pinia 4.1.0；Vite 8.3.5；Vitest 5.0.5。TypeScript 选 5.9.3（Vue 工具链稳定兼容版本），不直接追随当日最新主版本 7。

## 第二阶段复核

2026-10-28 再次通过 HTTPS 核对 calling-rust、calling-frontend 与 security/capabilities 页面（HTTP 200），沿用上列已安装依赖版本。任务采用 async command + spawn_blocking、AppHandle Emitter、前端 invoke/listen；自定义命令同时注册 AppManifest 和窗口级权限。未引入 v1 API 或数据库前端插件。

## 第三阶段官方核对（2026-10-29）

HTTPS实际读取HTTP 200：

- https://v2.tauri.app/learn/system-tray/ ：Rust TrayIconBuilder/MenuItem/DoubleClick、tray-icon feature。
- https://v2.tauri.app/plugin/global-shortcut/ ：GlobalShortcutExt、Pressed、注册注销，限Rust调用。
- https://v2.tauri.app/plugin/autostart/ ：ManagerExt::autolaunch，后台启动参数。
- https://v2.tauri.app/plugin/single-instance/ ：首插件与二次启动回调。

crates.io核对并锁定global-shortcut 2.4.1、autostart 2.7.0、single-instance 2.5.5。实际源码核对Monitor::work_area、WebviewWindow→Webview::window、Window::start_resize_dragging与tauri-runtime 2.12.2 ResizeDirection。窗口状态不用独立插件，统一存SQLite；Windows透明度限定本进程widget HWND。记录位于artifacts/stage-3/official-docs.jsonl。

## 第四阶段通知API核对（2026-10-29）

实际HTTPS读取 https://v2.tauri.app/plugin/notification/ 返回200，crates.io当前稳定版2.8.0，Cargo精确锁定tauri-plugin-notification=2.8.0。核对下载源码desktop.rs/lib.rs/models.rs：NotificationExt::builder/title/body/extra/show与on_action可在Rust调用；桌面permission_state始终Granted，不能判断Windows屏蔽；show后台异步派送；Windows仅屏幕上toast点击有回调保证范围，进入Action Center后不保证；remove_active在Windows无操作。代码据此始终保留持久化提醒中心，不宣称实际送达，也不为前端授予notification权限。本次未做原生通知验证。

## 第五阶段核对（2026-10-29）

实际HTTPS读取 https://v2.tauri.app/plugin/dialog/ 返回200，crates.io当前稳定版2.8.4；锁定tauri-plugin-dialog=2.8.4。核对实际源码DialogExt、blocking_save_file、FilePath::into_path、blocking_show，均在spawn_blocking调用，不阻塞Tauri主线程；插件只由Rust使用，不授予JS文件/对话框能力。

SQLite使用rusqlite0.41 backup feature：Backup::new/step增量Online Backup；不复制WAL中的主库。Windows文件替换使用windows-sys0.61的ReplaceFileW，先关闭SQLite连接、保留回退文件；未执行实际Windows文件锁或恢复验收。SHA256使用已锁定sha2 0.10.9。

## 第六阶段打包核对（2026-10-29）

实际读取Tauri v2 windows-installer和reference/config页面HTTP200，并核对已安装CLI2.12.4 schema、tauri-utils2.10.2配置源码。NSIS使用currentUser（RequestExecutionLevel user）、embedBootstrapper、SimpChinese/English、安装/卸载ICO与BMP；不配置签名。官方NSIS模板核对未勾选DeleteAppDataCheckbox默认保留数据，升级不走数据删除分支；完成后还需核对本次实际生成脚本。首次错误模板路径404、GitHub API限流后已通过官方raw正确路径取得模板，不以网络失败代替成功核对。
