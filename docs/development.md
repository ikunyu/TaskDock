# 开发指南

## 当前版本约定

当前应用版本为 TaskDock 1.1.1，数据库目标为 Schema 8。未经维护者明确要求，不自动递增应用展示版本。GitHub 更新使用独立递增的内部 build 和旧客户端兼容协议，见 [GitHub 更新说明](github-updates.md)。依赖版本和数据库 Schema 按各自的兼容性需求管理，不随应用展示版本改写。

源码仓库为 [ikunyu/TaskDock](https://github.com/ikunyu/TaskDock)，采用功能分支与 Pull Request 协作。克隆、环境准备、提交约束及发布分工见 [贡献与协作指南](../CONTRIBUTING.md)。下文各阶段条目记录当时的实现与验收范围，当前约定以本节及现有源码为准。

## 环境

推荐 Node.js 24 LTS、npm 11、Rust 1.99+ MSVC、Visual Studio 2022 C++ 构建工具、Windows SDK 10/11 和 WebView2 Evergreen。实际版本见阶段报告和 lockfile。依赖缓存与项目内工具链不提交；新开发机需自行安装依赖和构建工具。

```powershell
npm ci
npm run typecheck
npm test
npm run tauri -- dev
```

只预览前端：`npm run dev`，打开终端给出的 localhost URL。`?window=widget` 和 `?window=quick-entry` 可查看子窗口布局，页面会明确标识浏览器模式，不连接 SQLite。

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri -- build --no-bundle
```

若使用本次项目内工具链，先执行 `. .\scripts\Use-LocalToolchain.ps1`（仅影响当前 PowerShell 进程）。`.tools` 不提交；标准安装的开发机无需该脚本。不要把工具链、target、运行数据或数据库加入版本控制。

## 目录

`src/components` 共享基础组件；`src/windows` 三窗口；`src/services` typed IPC、日志/错误；`src/stores` Pinia；`src/styles` 主题；`src-tauri/src` 原生服务；`src-tauri/migrations` 只增不改的 SQL；`src-tauri/capabilities` 和 `permissions` 最小授权；`docs` 需求、决策与验收。

## 规则

增加 IPC 时同时更新 Rust AppManifest、permission、capability、前端类型和测试。禁止引入任意 SQL/路径参数。新增迁移只能添加下一个版本，不能修改已发布 SQL。业务状态只有提交事务后才能发事件，前端应支持事件乱序和重新获取快照。

当前数据库位于实际 EXE 目录的 data/taskdock.sqlite3，备份在 data/backups，日志及 WebView 缓存分别在同目录 logs、webview。首次启动在新 data 无库时可能只读导入旧版本地应用数据，细节见[安装说明](installation.md)。开发/运行应使用隔离数据，不指向真实库。不要记录任务标题、正文或导出内容。初始化失败保留原库，依据日志定位问题，不自动删除数据库。

## 第二阶段模块与验证

- `src/task-types.ts` / `src/domain/tasks.ts`：任务契约、纯日期、过滤和截止转换。
- `src/services/tasks.ts` / `src/stores/tasks.ts`：受限 IPC、revision 同步和失败状态。
- `src/components/DetailDrawer.vue`、TaskList/TaskRow、CategoryManager、DialogFrame/ConfirmDialog：任务交互、草稿及确认。
- `src-tauri/src/tasks`：模型、校验、事务仓储及 Rust 业务测试；`task_commands.rs`：IPC 与提交后通知。
- `tests/task-*.test.ts`：Vitest + Vue Test Utils + jsdom；`fixtures.ts` 仅用于测试，不作为应用示例数据。

Rust 测试在 tempfile 数据库执行，包括并发连接、失败触发器及迁移回退；禁止将测试 SQL 指向真实库。前端测试模拟 typed bridge 验证失败与竞态，原生 IPC/ACL/三窗口由 Windows 操作另行验收。

迁移 0001/0002 已发布使用，不得改写；修改表结构新增 0003。0002.down.sql 仅在显式事务内对空业务库验证回退，有任务/分类/审计时会拒绝，应用没有生产回退命令。任务聚合写入需带 version；遇 CONFLICT 保留草稿，由用户决定重新载入。

最终检查增加 `npm run format:check` 和 `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`；Rust 测试和 Clippy 使用 `--locked`。构建前关闭正在运行的 release，避免 Windows 可执行文件被占用。详细命令与结果见阶段报告。

## 第三阶段开发

新增锁定Rust插件global-shortcut 2.4.1、autostart 2.7.0、single-instance 2.5.5；Tauri 2.12.5启用tray-icon，tauri-runtime 2.12.2用于ResizeDirection。实际WebviewWindow无start_resize_dragging，通过as_ref().window()使用底层Window API；没有虚构set_opacity，Windows alpha封装在native模块。

src-tauri/src/desktop分为settings、geometry、runtime、native、shortcut、commands及tests。SettingsPanel和desktop-types复用app Pinia。测试仍使用tempfile，禁止在真实库注入故障。原生测试构建前需保存草稿并退出旧进程；默认关闭主窗口不再退出，请用托盘或软件设置的显式退出。

自启动绑定当前exe路径，移动未安装exe后需关闭再重新开启。--background可验证后台路径，但不等同真实Windows登录验收。实际快捷键冲突需要已知程序占键，不把纯注册单测称为原生冲突通过。交付依据见stage-3-report.md与手动清单。

## 第五阶段开发说明

源码0.5.0 / 迁移目标Schema5；第四/第五阶段未启动软件执行真实库升级，现有Release仍为第三阶段。新增tauri-plugin-dialog=2.8.4（仅Rust调用）、sha2=0.10.9和rusqlite backup feature。对话框的fs传递依赖不等于前端文件系统授权，capabilities仅main增加data-tools。

新增迁移不得修改0001–0004。备份/恢复只可通过LocalService生命周期和数据操作锁，不得另写文件复制实时WAL主库；只读导出/备份连接必须在生命周期租约内结束。恢复日志及回退文件不可在不明确状态时手工清理。用户操作说明见data-management.md。

本阶段仅允许vue-tsc、cargo check及格式静态检查；没有执行npm test/cargo test、GUI/软件启动或Release构建。后续验证应使用授权的隔离数据，先覆盖恢复各失败点和提醒去重，再触及用户实际恢复流程。

## 1.0.0 Windows打包

当前 `npm run package:windows` 生成x64 Release与NSIS，前置执行vue-tsc及Vite、递增内部build并使用维护者本机的原始密钥签署更新包；不会自动安装/启动TaskDock。其他开发者可执行普通前后端编译，无需获取发布私钥。bundle active=true，windows.nsis.installMode=currentUser；内嵌WebView2 bootstrapper，缺少运行时需联网；输出未做Windows Authenticode签名。更新包签名与Windows代码签名是不同机制。图标由scripts/generate-icon.py生成，含多尺寸ICO及NSIS BMP美术资源。

真实数据库及备份不得加入bundle或源码压缩包。第六阶段按用户范围未运行npm test/cargo test、GUI、安装卸载或恢复故障注入。构建记录、脚本静态核对和产物哈希见stage-6-report.md及artifacts/stage-6。
