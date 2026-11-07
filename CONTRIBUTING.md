# 参与 TaskDock 开发

源码仓库：[ikunyu/TaskDock](https://github.com/ikunyu/TaskDock)。当前应用版本为 **1.2.1**，数据库目标为 **Schema 8**。安装包在 Releases 发布，源码通过 Git 分支和 Pull Request 协作。

## 准备开发环境

Windows 10/11 x64，Git、Node.js 24、npm、Rust 1.99+ MSVC、Visual Studio C++ 构建工具、Windows SDK 和 WebView2。依赖版本以 package-lock.json 和 src-tauri/Cargo.lock 为准。项目内 `.tools` 工具链不上传，其他开发者需自行安装上述环境。

```powershell
git clone https://github.com/ikunyu/TaskDock.git
cd TaskDock
npm ci
npm run typecheck
cargo check --manifest-path src-tauri/Cargo.toml --locked
```

前端预览使用 `npm run dev`，不连接本地任务库。需要运行桌面开发版时使用 `npm run tauri -- dev`，会在开发版 EXE 所在目录创建自己的 data。不要把真实任务库复制到开发目录或提交仓库。第一次启动也可能导入本机旧版数据；验证数据迁移或恢复时请使用隔离的 Windows 测试账号/环境。

无需发布签名密钥也可执行普通编译：

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked
```

以上是静态检查，不等于业务或 Windows 原生功能验收。测试命令与范围见 [测试计划](docs/test-plan.md)；只对隔离数据运行测试，明确记录未运行的检查。

## 分支与 Pull Request

有仓库写权限的协作者先从最新 main 创建功能分支：

```powershell
git switch main
git pull --ff-only origin main
git switch -c feat/your-change
# 修改源码并执行适用的检查
git add <本次修改的文件>
git diff --cached
git commit -m "feat: describe the change"
git push -u origin feat/your-change
```

随后在 GitHub 创建 Pull Request，说明具体行为、检查结果及待验证项。修复可使用 `fix/` 前缀。每个分支围绕一个明确改动；不要强制推送 main，不要覆盖其他人的提交。合并前同步最新 main 并解决冲突。没有写权限时先 Fork 仓库，再从 Fork 提交 Pull Request。

仓库所有者可在 Settings → Collaborators 邀请团队成员。保护 main 的分支规则由所有者按团队需要启用；本次没有自动邀请用户或配置分支保护。

## 必须保留的架构约束

- Rust LocalService 和 SQLite 是唯一真实数据来源；复用 revision 事件、编辑草稿及 version 乐观锁。
- 已发布的 0001–0008 迁移不能修改；新结构新增下一版本迁移及适用的回滚逻辑，不删除数据库解决兼容问题。
- 任务、重复实例、提醒状态和审计记录按现有事务与生命周期规则写入；备份恢复统一在 Rust 服务处理。
- IPC 同时维护 Rust 注册、权限定义、分窗口 capability 和 TypeScript 契约；不开放任意 SQL 或文件路径访问。
- 数据、备份、导出、日志、工具链、签名私钥、GitHub 凭据和 `.env` 不进入提交。提交前检查 `git diff --cached`。

## 版本与发布

普通功能修改保持当前应用版本；仅在维护者明确决定时提高版本号。应用版本涉及 package.json/lock、Cargo.toml/lock、tauri.conf.json、release-config.json 和前端初始版本值，必须一致。

GitHub 更新识别独立递增的内部 build。**不要修改 `1.0.0+build` 更新协议前缀或删除 `TaskDock_1.0.0_x64-setup.exe` 兼容下载别名**，它们供已安装客户端识别新版，实际应用版本可为 1.2.1。

正式签名、构建和发布由持有原始私钥的维护者负责。协作者不需要获取私钥；不要生成新密钥覆盖现有更新通道。源码提交不会自动发布安装包，也没有配置自动发布工作流。发布步骤见 [GitHub 更新说明](docs/github-updates.md)。
