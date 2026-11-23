# TaskDock E盘迁移交接

更新日期：2026-11-23。本文专门说明迁移后的项目位置；完整功能与架构见[项目交接](../HANDOFF.md)。

## 当前唯一有效的开发位置

**E:\vibecoding\tododock**

以上是本机当前路径。其他电脑或云端克隆以其实际检出根目录为准，先定位再执行，不能照搬任何过期盘符。

旧位置 F:\tododock 已不存在；本次也确认 E:\tododock 不是本项目目录。不要根据旧聊天、旧命令、旧阶段报告猜测工作目录。

开始任何工作前先明确目录并检查：

```powershell
Set-Location -LiteralPath 'E:\vibecoding\tododock'
Get-Location
Test-Path -LiteralPath '.\package.json'
Test-Path -LiteralPath '.\src-tauri\tauri.conf.json'
git -C 'E:\vibecoding\tododock' rev-parse --show-toplevel
```

使用执行工具时，应显式传入工作目录 E:\vibecoding\tododock；文件读写和最终交付链接同样使用E盘绝对路径。若旧会话环境仍登记 F:\tododock，那是过期的会话记录，不是项目实际位置；交接给新会话、IDE或开发者时必须选择当前E盘项目。

## 路径速查

| 内容              | 当前实际路径                                                          |
| ----------------- | --------------------------------------------------------------------- |
| 项目根目录        | E:\vibecoding\tododock                                                |
| 主交接文档        | E:\vibecoding\tododock\HANDOFF.md                                     |
| 路径交接          | E:\vibecoding\tododock\docs\path-migration-handoff.md                 |
| 前端源码          | E:\vibecoding\tododock\src                                            |
| Rust源码          | E:\vibecoding\tododock\src-tauri\src                                  |
| SQL迁移           | E:\vibecoding\tododock\src-tauri\migrations                           |
| Release构建主程序 | E:\vibecoding\tododock\src-tauri\target\release\taskdock.exe          |
| 便于交付的EXE     | E:\vibecoding\tododock\artifacts\release\TaskDock.exe                 |
| 当前安装包        | E:\vibecoding\tododock\artifacts\release\TaskDock_1.2.2_x64-setup.exe |
| 当前源码包        | E:\vibecoding\tododock\artifacts\release\TaskDock-1.2.2-source.zip    |
| 更新清单          | E:\vibecoding\tododock\artifacts\release\latest.json                  |
| 项目内工具缓存    | E:\vibecoding\tododock\.tools                                         |
| 私钥/本机授权目录 | E:\vibecoding\tododock\.private（仅说明位置，不包含密钥或令牌内容）   |

本次核对源码版本为1.2.2、Schema8、内部构建202611030020，本地更新清单也记录该构建。后续接手以release-config.json、两个Cargo/npm版本文件和实际Git提交为准，不按早期1.1.1交接快照回退版本。

## 开发目录与任务数据目录不同

程序按实际运行EXE的目录创建data、logs和webview。开发目录搬到E盘，不会替另一处安装的软件自动移动数据。

- 若运行交付EXE，数据库通常在 E:\vibecoding\tododock\artifacts\release\data\taskdock.sqlite3，备份在同目录data\backups。
- 若运行Rust Release构建EXE，data在 E:\vibecoding\tododock\src-tauri\target\release\data。
- 若运行另一个安装目录中的软件，以那个EXE目录为准；不要把这份文档里的开发路径强行当成用户数据库路径。
- 设置→数据管理可在用户自行启动软件后查看实际数据目录。本次没有启动软件、读取或升级任务数据库，也没有移动用户数据。

真实数据需正常从托盘退出后整体转移，不能只复制正在WAL写入中的主数据库；不能为了修正路径而删除数据库、备份或恢复日志。

## 已核对的路径配置

- Rust storage.rs 使用current_exe动态定位数据根目录，未发现把原F盘路径硬编码为运行数据位置。
- 构建、发布和工具链脚本按脚本自身位置定位项目根目录；文档采用当前E盘路径，脚本继续保持相对定位。
- 本机.git/config的GitHub credential helper已指向 E:/vibecoding/tododock/.private/gh 和 E盘项目内gh.exe，无需再替换成旧路径。它是本机配置，不随普通clone共享。
- 历史阶段报告保留当年F盘路径，并在开头注明历史属性；不能将当年的构建命令和输出路径冒充当前位置。
- 本次仅更新文档、代理接手路径说明和源码归档清单，不改业务、版本号、迁移、Git认证或软件设置，不构建或发布安装包。

## 发给新开发者或AI的说明

```text
项目已经移动到 E:\vibecoding\tododock，旧 F:\tododock 已不存在。
请以E盘目录作为工作区和每次执行命令的workdir，先阅读根目录AGENTS.md、HANDOFF.md以及docs/path-migration-handoff.md。
先核对Get-Location、git rev-parse --show-toplevel、release-config.json和实际源码；当前检查到1.2.2、Schema8、build202611030020，若后续源码已有更新，以实际文件为准。
不要沿用旧聊天里F盘的cwd、下载链接或产物路径；不要回退到旧版本，也不要重初始化工程或用户数据库。
程序数据随实际EXE目录保存，不能把项目迁移等同于已安装软件的数据迁移。
本次是路径交接，不要求启动软件、运行测试、构建或发布；接下来按用户的新需求工作。
```
