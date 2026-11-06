# TaskDock 项目交接文档

更新日期：2026-11-06。适用于转移开发目录、换电脑、换 IDE 或交给其他开发者/AI 继续开发。本文记录当前源码和已有交付状态，不代表完整功能验收通过。

## 1. 接手时的基线

| 项目            | 当前状态                                                                                   |
| --------------- | ------------------------------------------------------------------------------------------ |
| 原开发目录      | `F:\tododock`；迁移后以新目录为准，运行数据位置由实际 EXE 路径决定                         |
| 应用名称 / 标识 | TaskDock / `com.taskdock.desktop`                                                          |
| 显示版本        | **1.1.1**；未经用户要求，不自动提高版本                                                    |
| SQLite 目标     | **Schema 8**，8 个上行迁移、7 个下行脚本，共 15 个 SQL 文件                                |
| 内部构建号      | **202611030007**，见 `release-config.json`                                                 |
| 更新协议版本    | `1.0.0+202611030007`；与显示版本分开管理                                                   |
| 源码仓库        | <https://github.com/ikunyu/TaskDock>；交接核对时为公开仓库，默认分支 main                  |
| 业务代码基线    | `8be6f4a3f2dc163843a1f0d18ce418bf3b65b9fe`，锁图标透明度修复                               |
| 远端核对        | 本次只读 fetch 后，本地 HEAD 与 origin/main 一致；新增本文的文档提交会晚于上述业务代码提交 |
| 最新已发布构建  | <https://github.com/ikunyu/TaskDock/releases/tag/build-202611030007>                       |

后续接手先读取实际 `git status`、最新 main、`release-config.json`、`package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/src/db.rs`，不能只按本文中的快照操作。

本次交接只新增/整理文档，不启动软件、不访问真实数据库、不构建新版本，也不发布新的安装包。

## 2. 产品现状与用户习惯

已完成六阶段开发及后续迭代，核心代码包括：

- 任务创建、编辑、完成/撤销完成、确认后永久删除，分类、优先级、描述和子任务进度。
- “任务”入口下的今天 / 未来事项；独立心愿单、已完成、月历、周概览。周按周一到周日计算，周概览采用紧凑行布局。
- 主窗口、桌面悬浮小组件、快速输入；托盘、全局快捷键、开机启动和单实例保护。
- 首次指定时间、截止及提前提醒、稍后提醒、定期提醒同一任务、Windows 通知及应用内提醒中心。
- 每日、每周、每月、自定义间隔的重复任务，独立实例、结束日期及此后实例规则调整。
- SQLite 一致性备份、保留策略、手动备份、安全恢复和 JSON/CSV 导出。
- GitHub 检查更新、下载、签名验证和用户确认安装重启。

以下是已确认的界面和业务取舍，继续开发时保留：

- 心愿单用于无固定日期的个人愿望，做完自行勾掉，完成项保留划线。
- 已删除入口、搜索与筛选页面、操作历史界面已移除；后端审计、查询及旧软删除数据仍有用途，不据此删除相关服务或数据表。
- 完成控件在任务后面，勾选框约14像素、点击区域28像素；与删除×对齐，悬停不变浅。
- 悬浮窗今日与未来任务完成后保持原位置并划线，不立即消失或排到末尾；未来按周分组。
- 小组件要窄、小、按内容自适应，任务多时内部滚动；保留展开/收起与今日/未来页签。
- 小组件展开默认宽180逻辑像素，收起宽度通常在120–180内自适应；实际尺寸约束以 runtime、geometry 和 WidgetWindow 中的代码为准。五类字号可独立调节。
- 不透明度20%～100%；锁定后只有锁形按钮可点，其余区域穿透到下方应用。开锁/关锁图标用形状区分，背景不随状态变色。托盘和设置仍可解锁。
- 无事项时留白，不添加空状态插图或引导提示；实际错误、草稿冲突和危险操作确认必须保留。
- 主界面的快速输入入口已移除，底层快速输入窗口、快捷键及小组件快速添加仍保留。
- 界面日期名称为“截止日期”；截止方式保留“无截止 / 仅日期 / 精确时间”，精确时间使用原来的本地日期时间选择器。
- 定期提醒同一任务与“重复任务生成新实例”是两种机制，不能混为一谈。

详细用户说明：[安装与使用](docs/installation.md)、[任务与心愿单](docs/task-workspace.md)、[周概览](docs/weekly-overview.md)、[锁定功能](docs/widget-lock.md)。README 和早期阶段报告保留了一些历史尺寸/版本说明，当前行为以最新说明及源码为准。

## 3. 技术架构与入口

| 位置                                                             | 用途                                                                    |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `src-tauri/src/lib.rs`                                           | Tauri 初始化、插件、窗口创建、唯一 LocalService、IPC 注册和后台服务启动 |
| `src-tauri/src/service.rs`                                       | 数据库服务、连接和生命周期、维护状态及并发协调                          |
| `src-tauri/src/db.rs`、`migrations/`                             | 数据库版本、迁移原文校验、设置快照和迁移事务                            |
| `src-tauri/src/tasks/`、`task_commands.rs`                       | 任务聚合、校验、事务、乐观锁和任务 IPC                                  |
| `src-tauri/src/desktop/`                                         | 托盘、快捷键、窗口布局、位置恢复、主题、不透明度、锁定及设置            |
| `src-tauri/src/schedule/`                                        | 提醒、定期提醒、重复实例、持久化状态及后台调度                          |
| `src-tauri/src/data/`                                            | 一致性备份、恢复、导出、日历/查询及数据管理                             |
| `src-tauri/src/storage.rs`                                       | EXE 相邻目录存储、旧 AppData 导入、暂存和安全发布                       |
| `src-tauri/src/updates.rs`                                       | GitHub 更新检查、下载校验、关闭服务及安装重启                           |
| `src/windows/`                                                   | MainWindow、WidgetWindow、QuickEntryWindow                              |
| `src/stores/`、`src/services/`                                   | Pinia 与 typed IPC；app、tasks、updates 状态及 bridge                   |
| `src/components/DetailDrawer.vue`                                | 任务详情、日期、子任务、提醒和重复设置                                  |
| `src/components/SettingsPanel.vue`                               | 统一设置界面；数据管理使用独立组件                                      |
| `src/styles/`                                                    | 主题 tokens、布局及各模块样式                                           |
| `widget-lock.html`、`src/widget-lock.ts`                         | 解锁点击辅助页的独立 Vite 入口，不能在重构时漏打包                      |
| `src/components/WidgetLockButton.vue`、`desktop/lock_control.rs` | 锁定时的点击接收区域和位置同步                                          |
| `src-tauri/capabilities/`、`permissions/`、`build.rs`            | 分窗口 IPC 白名单及命令声明                                             |
| `tests/`、Rust 各模块 tests                                      | 已有回归用例；存在用例不等于最近已执行                                  |

架构约束：

1. SQLite 是唯一真实数据来源，多窗口共用一个 Rust LocalService。前端不直接执行 SQL、不持有独立任务数据库。
2. 写操作与审计使用事务；区分任务 revision、设置 revision 和任务自身 version。编辑冲突保留草稿，保存失败不展示已保存。
3. 纯日期按本地日历日期处理，精确时间按 instant 处理，不能用 UTC 换日来计算所有任务日期。底层 `planned_date`、`due_date`、`due_at` 仍承担旧数据兼容职责，不因界面改名而直接删字段。
4. 恢复、更新安装、退出与调度器共享现有生命周期协调，不新建绕过维护锁的连接或服务。
5. 新 IPC 同时维护命令注册、AppManifest、permission、capability、调用方校验及 TypeScript 契约。没有通用 SQL、任意路径或全量 core 权限。

### 锁定与透明度的当前实现

widget 本体使用 Tauri 的鼠标穿透；另有名为 `widget-lock` 的按钮尺寸 owned 辅助窗口，只接收解锁点击，不初始化任务 Store、调度器或更新器，也不显示任务栏图标或获取焦点。

最新修复把可见开锁/关锁图标放回 widget 本体绘制，随整体不透明度变化。辅助页不绘制图标，原生 alpha 固定为最低非零值 **1/255**，以保留22逻辑像素区域的鼠标命中。不能随意设成0，否则点击也会穿透；不能恢复为与 widget 相同的不透明度，否则背景叠加会再次出现亮色方块。最低非零输入层仍需在目标 Windows、不同DPI和透明度下实际确认。

按钮位置由 widget 上报受限的客户区矩形，再按当前 DPI 转成物理像素。辅助窗口仅允许读取设置、订阅事件、报告就绪和解锁，不能编辑任务、锁定窗口或操作任意窗口。拖动和缩放仍保留原有同步命令。

## 4. 迁移时分别带走什么

| 内容                                    | 交接方式                                                                      |
| --------------------------------------- | ----------------------------------------------------------------------------- |
| 开发源码                                | Git 仓库或源码 ZIP；包括隐藏配置、lockfile、图标、SQL、文档及两个 HTML 入口   |
| 维护者签名密钥                          | 单独安全转移原 `.private/taskdock-updater.key`；不提交 Git、不夹入公开源码包  |
| GitHub 登录                             | 新电脑重新登录；原凭据依赖 Windows keyring，复制 `.private/gh` 不代表授权可用 |
| 真实任务及备份                          | 仅用户需要迁移运行数据时，在托盘正常退出后复制实际软件目录的完整 `data`       |
| `.tools`                                | 可选的大体积本机工具链缓存；新电脑通常重新安装工具，不需要公开交接            |
| `node_modules`、`target`、`dist`、`gen` | 可重新生成，不作为开发源码必需品                                              |
| `artifacts`                             | 本机产物、日志和历史证据；可按需另存，不属于 Git 源码                         |

运行数据以**实际 EXE 所在目录**为根：

```text
软件目录/
  TaskDock.exe
  data/
    taskdock.sqlite3
    backups/
  logs/
  webview/
```

开发版、构建目录中的 EXE、交付目录 EXE 和已安装 EXE 可能各有自己的 data。移动源码目录并不自动移动另一个安装目录的任务库。需要携带任务时至少带 EXE 和完整 data；日志、WebView 缓存通常可重建。软件放在C盘，数据也会跟随在C盘。

旧路径是 `%LOCALAPPDATA%\com.taskdock.desktop\taskdock.sqlite3`。仅当新 data 尚无库时，程序使用只读旧连接和 SQLite Online Backup 创建暂存快照、升级校验、复制备份后启用新目录，原C盘文件保留。已有目标库不覆盖；不完整目录、恢复日志等异常会阻止创建空库顶替。不要手工清理 `restore-journal.json`、恢复回退文件或 `.taskdock-import-*` 来绕过报错。

程序运行时可能使用 WAL，不能只复制主库文件作为可靠备份；优先使用应用内一致性备份，或正常退出后转移完整 data。其他设备之间没有任务云同步，GitHub 更新也不传任务数据。

## 5. 新环境准备与迁移顺序

目标发布平台为 Windows 10/11 x64。准备 Git、Node.js **24**、npm、Rust **1.99+ MSVC**（Cargo 声明的最低版本）、Visual Studio C++ 构建工具、Windows SDK、WebView2。发布脚本还需 Python 3.11+ 和 GitHub CLI `gh`；已有图标直接使用仓库资源，重生成图标的依赖另看脚本。

前端版本快照：Vue3.5.44、Pinia4.1.0、TypeScript5.9.3、Vite8.3.5、Tauri JS/CLI2.12.4。Rust Tauri2.12.5、updater2.14.1、rusqlite0.41.0（bundled、backup）。完整依赖以两个 lockfile 为准，不为了搭环境盲目升级。

建议顺序：

1. 保留当前完整工程的离线副本，尤其保留旧迁移文件的原始字节；不要先用 Git checkout 或格式化覆盖旧工程。第8节有尚未处理的换行兼容风险。
2. 在新位置复制工程或克隆仓库。若克隆，保留旧工程作为兼容性核对依据。
3. 准备工具并先做静态检查；此步骤不启动 TaskDock。
4. 若仅参与开发，不需要签名私钥。若接手正式发布，再单独迁移原私钥、核对公钥并重新授权 GitHub。
5. 在隔离环境确认迁移原文兼容、原生窗口和打包更新流程，再接触真实用户运行数据。

新克隆的静态准备示例：

```powershell
git clone https://github.com/ikunyu/TaskDock.git
cd TaskDock
npm ci
npm run typecheck
cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

若复制原工程且继续使用本机项目工具链，可先执行 `. .\scripts\Use-LocalToolchain.ps1`。脚本按自身位置定位 `.tools`，但工具链和 Windows SDK 本身是否可跨电脑使用仍需检查；新电脑推荐安装标准 MSVC 环境。

需要前端预览时运行 `npm run dev`，不连接真实任务库。`npm run tauri -- dev` 会启动桌面服务并可能导入旧 AppData，必须由接手者明确决定在隔离账号/环境执行；本次交接没有运行。

复制 `.git` 时，本机 `.git/config` 的 GitHub credential helper 包含 `F:/tododock/.private/gh` 和项目内 gh.exe 的绝对路径。改目录后需要修正或重新配置 Git 认证，不能假定能直接 push。新克隆不会带走这份本机配置。也不要把原机器的 Git 作者身份当作新开发者身份。

## 6. 构建、签名与 GitHub 发布

### 日常协作

从最新 main 建立 `feat/...` 或 `fix/...` 分支，通过 Pull Request 合并，见 [CONTRIBUTING.md](CONTRIBUTING.md)。Code 与 Releases 分别管理，源码提交不会自动生成安装包。尚未配置 GitHub Actions 云端构建/发布；直接在网页创建 Release 不能代替编译。

仓库当前公开，未实施“私有源码仓库 + 公开更新仓库”拆分。有写入权限的协作者通常也能创建 Release，但持有源码不等于持有可供现有客户端信任的签名私钥。

### 版本规则

- 显示版本维持1.1.1，只有用户明确要求才提高。提高时同步 package.json/lock、Cargo.toml/lock、tauri.conf.json、release-config.json，以及 app Store / bridge 的初始版本。
- 发布一次新程序时递增 `release-config.json.build`。`npm run package:windows` 会自动递增，所以重试同一未发布构建不要再次加号。
- 更新客户端固定识别 `1.0.0+BUILD` 和兼容文件名 `TaskDock_1.0.0_x64-setup.exe`。**不能只因显示版本已是1.1.1就改掉这两个值。** 兼容别名与真实版本安装包字节相同。
- 更新地址固定为 `https://github.com/ikunyu/TaskDock/releases/latest/download/latest.json`，下载 URL 也限制在该仓库。移动开发目录不需要改地址；换仓库涉及旧客户端升级桥接，不能直接替换后假定旧设备可用。

### 新维护者认证

发布脚本优先使用 `.tools/github-cli/bin/gh.exe`，没有则使用 PATH 中的 gh；固定读取本项目 `.private/gh` 配置。使用系统 gh 时，在项目根目录授权：

```powershell
$env:GH_CONFIG_DIR = Join-Path (Get-Location) '.private/gh'
gh auth login --hostname github.com --git-protocol https --web
gh auth status --hostname github.com
```

原签名密钥放回 `.private/taskdock-updater.key`。公钥已嵌入 Tauri 配置。不得重新生成密钥覆盖现有更新通道，也不把私钥、令牌、数据库发给所有协作者。复制到新电脑后重新确认私钥访问权限。现构建脚本使用空密码参数；若加密私钥，需同步调整安全读取密码的方式，不能把密码写进源码。

### 正式发布顺序

本机源码必须先同步到准备发布的提交；如果当前没有云构建，就不能省略本地源码同步。先更新 `release-notes.md`，执行适用静态检查，再构建：

```powershell
npm ci
npm run typecheck
cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked
npm run package:windows
```

构建脚本包括 TypeScript/Vite、Rust Release、NSIS、签名、兼容别名复制、latest.json 生成和签名核对。成功后，核对源码与构建号，将源码和构建配置同步到 main，再发布：

```powershell
.\scripts\Publish-GitHubRelease.ps1
```

脚本先创建草稿并上传固定资产，再标记为最新公开 Release，标签是 `build-BUILD`，目标是 main。要仅准备草稿可加 `-Draft`。同一**未发布**构建失败后修复重试使用 `.\scripts\Build-WindowsRelease.ps1`，不带 `-NextBuild`。不要覆盖已经发布的构建号或把失败产物标为成功。

重要细节：

- 不要把 `npm run package:windows:raw -- --ci` 当作完整交付流程；缺少后续协议版本签名与清单步骤时，客户端可能无法验证。
- 不要照早期文档执行旧形式 `npm run package:windows -- --ci`；当前正式入口就是 `npm run package:windows`。
- 显式 `--private-key-path` 签名时，脚本会暂时清除继承的 `TAURI_SIGNING_PRIVATE_KEY`，避免 CLI 参数冲突；保留该处理。
- Windows 锁住正在运行的 EXE 时，不杀用户进程。先完成代码与构建，再让用户保存并从托盘退出后补齐交付。
- GitHub CLI 的代理可能与系统浏览器不同。发布脚本会尝试系统代理；旧机器曾使用127.0.0.1:7897，新环境不能写死照搬。
- 更新安装沿用当前 EXE 目录，安装前保存多窗口草稿并关闭服务。`cleanup_before_exit()` 后不得再调用 Tauri 窗口/状态 API；当前失败路径已有原生提示与退出处理。
- Updater Ed25519/minisign 签名与 Windows Authenticode 是两回事。现有 EXE/安装器仍显示 **NotSigned**，没有发布者证书或签名认证。

### 最新交付位置与参考哈希

旧工程中位置是 `F:\tododock\artifacts\release\`；迁移后将前缀替换为新工程目录。

| 文件                                               | 参考值                                                                                    |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `TaskDock.exe`                                     | 19,209,728字节；SHA256 `51B36C558AB53330F2C5C53EC22ED40C528960CECBDD3384824BF6D2F217F04F` |
| `TaskDock_1.1.1_x64-setup.exe`                     | 6,621,302字节；SHA256 `98279008E0F3EB427EAAC01DB92743B592615EFD5518B479EC1C76C5E0F8B446`  |
| `TaskDock-1.1.1-source.zip`                        | 源码归档，无私钥/凭据/任务数据；新增交接文档后归档内容及哈希可能更新                      |
| `latest.json`、`.exe.sig`、`UPDATE-SHA256SUMS.txt` | 更新清单、签名与发布资产校验信息                                                          |

NSIS 按当前用户安装，缺少 WebView2 时联网安装运行时。普通卸载保留非空 data、logs、webview；不能为了打包递归删除安装目录。正式安装/卸载的实际结果仍待验收。

源码归档使用 `python scripts/package-source.py`，它是显式文件清单；新增根目录入口/交接文件时同时维护清单，不能靠“整个目录打ZIP”收集源码。

## 7. 实际验证状态

最新透明度修复构建已实际完成 TypeScript 检查、Rust all-targets 静态编译、Rust/受影响前端格式检查、Release/NSIS 构建及更新签名核对。已从公开更新地址下载清单和安装包并比对哈希。证据主要保存在旧工程：

- `artifacts/widget-lock-opacity-check.log`
- `artifacts/github-update-lock-opacity-build.log`
- `artifacts/github-lock-opacity-publish.log`
- `artifacts/github-lock-opacity-publish-verification.json`
- `artifacts/stage-6/outputs.json`

这些 artifact 文件不在 Git 仓库，新克隆没有它们。最新迭代没有执行 npm test、cargo test、TaskDock 启动、GUI自动化、安装器运行或真实数据库迁移/恢复；早期阶段报告中通过的测试不能替代当前版本验收。

必须继续标为待验证：

- 锁图标透明度修复后的实际视觉，20%/60%/100%、明暗主题、直接解锁和不同DPI下点击区域。
- Windows 10、多屏断开/分辨率变化、睡眠恢复和快捷键冲突。
- 提醒、重复实例去重、长时间后台调度及系统通知点击行为。
- 数据库旧版本升级、备份/恢复在 WAL、SHM、文件锁和中途失败下的完整性与回退。
- 应用内检查→下载→安装→重启，安装/卸载保留数据及开机启动的完整原生流程。
- 主窗口未保存草稿期间，悬浮窗改变同一任务状态，再保存时的原生冲突表现；此前相关自动化记录不能代替这项原生检查。

用户曾明确要求以功能代码和静态检查为主，不运行自动化测试、GUI或真实库验收。继续工作时遵循用户最新授权；不得把未执行项目写成通过，不以删除真实数据解决错误。

## 8. 迁移开发环境前优先关注的已知风险

### 旧 SQL 文件换行与迁移原文校验

本次 `git ls-files --eol src-tauri/migrations` 的只读结果显示：Git 索引中的SQL都是LF，本机0003部分文件是mixed，0006/0007是CRLF，其他文件多为LF。`.gitattributes` 规定SQL检出为LF。

`db.rs` 的迁移校验使用 `expected.sql == sql`，备份迁移记录也做原文比较；恢复还比较 sqlite_master 的结构SQL。新克隆/重新检出后 `include_str!` 编译进程序的文本可能与旧机器发行版不同，**因此有因换行而拒绝旧库或备份的兼容风险**。本次只检查源码，没有打开真实库，也没有复现该报错，不能称为已验证故障。

接手者先保留原工程迁移文件的字节副本和已发布程序，再在隔离数据中核对。后续若修复，只应对不改变SQL语义的兼容差异作明确设计与验证；不能直接改已发布迁移、改写真实库迁移台账、忽略所有校验或删除用户库。当前交接没有修改此逻辑。

### 其他注意点

- 老文档含历史版本、窗口宽度、数据存储位置和发布命令，按本文、最新源码与当前脚本核对。
- `.private`、工具链缓存、Git credential helper 和 Windows keyring 不随普通克隆迁移，开发能编译不代表能签名发布。
- Release 构建不能替代原生验收；最低非零alpha输入窗口在目标环境的视觉及命中仍需确认。
- 旧Schema8数据不能交给只支持更低Schema的程序打开；当前没有生产环境降级IPC。

## 9. 给下一位开发者或AI的起始说明

可将下面内容和本文一起交接，再附上新的具体需求：

```text
你接手的是 TaskDock Windows 桌面待办项目，请先读项目根目录 HANDOFF.md、README.md、CONTRIBUTING.md，再核对实际源码、Git状态和 release-config.json。
原目录 F:\tododock，若已移动，以我提供的新目录为准。当前业务基线为1.1.1、Schema8、内部build202611030007；继续核对实际最新提交。
保留唯一Rust LocalService、SQLite、revision事件、任务version乐观锁、设置草稿以及分窗口最小权限。
保留最新的开锁/关锁图标、直接点击解锁和统一透明度实现；widget-lock辅助页仅负责输入，不重复绘制可见图标。
不要重初始化、修改已发布迁移、删除用户数据、上传私钥或擅自提高显示版本。
先关注交接文档中的SQL换行校验和本机凭据路径迁移风险，不要直接用真实库验证或绕过校验。
未经我新授权，不运行自动化测试、GUI、安装器或真实数据库迁移/恢复；可以进行必要静态检查。构建成功不等于功能测试通过。
功能改完需按我要求交付EXE/安装包及GitHub源码/Release；显示版本和内部build分开管理，保留旧客户端更新协议及兼容安装包别名。
按照我接下来提出的具体需求开发，不自动开始额外阶段。
```

相关文档优先读：[开发指南](docs/development.md)、[架构](docs/architecture.md)、[数据模型](docs/data-model.md)、[GitHub发布](docs/github-updates.md)、[测试计划](docs/test-plan.md)、[锁定实现](docs/widget-lock.md)。六阶段报告主要用于追溯开发过程。
