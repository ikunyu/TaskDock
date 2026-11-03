# TaskDock 1.0.0 月历查询修复

日期：2026-10-30。

## 原因与修改

月历加载同时调用月份汇总和选中日期的任务搜索。`src-tauri/src/data/query.rs` 的共用搜索 SQL 在逾期条件的 `COALESCE` 参数内多写了一个右括号。SQLite 编译整条查询时返回 `near ")": syntax error`，即使没有启用逾期筛选也会失败。搜索和使用该查询的 CSV 导出同样受影响。

修正括号，使逾期布尔表达式与回退值 `0` 正确位于同一个 `COALESCE` 调用内。`data/commands.rs` 的通用错误提示不再将所有失败暗示为磁盘或文件权限问题，也不再声称“已有备份保留”。详细底层错误仍写入日志。

月历修复已合入统一的 1.0.0 源码、安装包及前端版本显示，安装指南与 README 同步。没有修改迁移、IPC 权限、数据库标识或任务数据格式，数据库目标保持 Schema 5。

## 检查与交付

- 使用 Python SQLite 3.49.1 的内存连接，仅加载项目 Schema 2 表定义并执行 `EXPLAIN` 编译检查；修复前搜索 SQL 报语法错误，修复后共用搜索和月历汇总 SQL 均可编译。没有执行任务查询或接触真实用户数据库。
- `npx prettier --check package.json src-tauri/tauri.conf.json src/services/bridge.ts src/stores/app.ts`：通过。
- 9 个既有迁移文件 SHA256 与第六阶段基线一致。
- Rust 格式检查及正式安装包构建结果、交付路径见下方最终构建记录。

没有运行自动化测试、启动软件、执行 GUI 验收、安装或恢复数据库。SQL 编译检查与构建成功不等于全部功能验收通过。

## 最终构建记录

`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
`npm run package:windows -- --ci` 通过，含 TypeScript 检查、Vite、Rust Release 编译及 NSIS 打包。
受限沙箱第一次构建在解析 `../dist` 时被路径权限阻止；在授权环境重新执行成功，日志见 `artifacts/calendar-fix-build.log`。

交付文件：

- `F:\tododock\artifacts\release\TaskDock_1.0.0_x64-setup.exe`
- `F:\tododock\artifacts\release\TaskDock.exe`
- `F:\tododock\artifacts\release\TaskDock-1.0.0-source.zip`

可执行文件产品版本确认为 1.0.0；主程序和安装包均未数字签名。多余版本的交付文件已清理，校验值见同目录 `SHA256SUMS.txt`。

## 升级方式

从托盘退出旧版，再运行 1.0.0 安装包覆盖升级。无需先卸载、删除数据库或恢复备份。升级后打开月历，检查当前月份、切换月份、选中日期及分类筛选；也可检查搜索和 CSV 导出。实际安装和界面表现仍待用户确认。

2026-10-30：本功能已合并到统一的 1.0.0；最新完整构建及 TypeScript 检查通过，日志见 `artifacts/unified-1.0.0-build.log`。当前交付已替换为这一构建，旧小版本安装包、主程序副本及源码包已删除。
