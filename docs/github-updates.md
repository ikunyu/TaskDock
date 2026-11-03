# GitHub更新与发布

源码与更新仓库：https://github.com/ikunyu/TaskDock 。main 保存源码与开发文档，协作方式见[贡献指南](../CONTRIBUTING.md)。Releases 继续发布安装包、签名、latest.json和公开SHA256清单；用户数据、私钥和凭据不上传。

## 用户使用

旧版不含更新检查代码，需先安装一次支持更新的本版；随后启动时自动检查，有新构建会提示。也可在软件设置→关于与运行状态→检查更新。先下载，验证成功后点击安装并重启；先保存所有窗口内容。当前软件显示1.1.1，内部build递增识别更新。更新协议继续使用1.0.0+build及TaskDock_1.0.0_x64-setup.exe兼容别名，以便原有1.0.0更新器识别；兼容别名与当前版本安装器字节完全相同，签名校验完整构建号。不要单独改动协议前缀或删除兼容下载资产。TaskDock数据依然仅保存在本机data，更新不在设备间同步任务。

## 维护者发布

1. 更新release-notes.md。
2. 在构建电脑的项目目录执行 `npm run package:windows`；它递增release-config.json内部build，调用本地工具链编译、使用原签名密钥签署NSIS包、生成latest.json并离线验证签名和构建号。只重试同一未发布构建时可执行 `.\scripts\Build-WindowsRelease.ps1` 不加NextBuild。
3. 执行 `.\scripts\Publish-GitHubRelease.ps1`。先上传草稿，再设为最新公开Release；使用 `-Draft` 可只准备草稿。仅上传兼容别名TaskDock_1.0.0_x64-setup.exe及.sig、当前软件版本的安装包及.sig、latest.json、UPDATE-SHA256SUMS.txt；源码及用户数据不上传。发布前确保main分支至少有README初始化提交。
4. 查看仓库Releases和 `https://github.com/ikunyu/TaskDock/releases/latest/download/latest.json`。

签名私钥是.private/taskdock-updater.key，GitHub CLI授权配置在.private/gh；目录仅当前用户可访问，源码ZIP和Git忽略它。请离线妥善备份原私钥；不要上传GitHub，不要发送给其他设备或他人。丢失私钥后，新密钥签的包不能被已安装客户端信任，需要重新手动安装来更换信任根。客户端只带公钥，无GitHub令牌。更新签名不等于Windows代码签名证书。

发布脚本需要本机GitHub CLI已登录。工具在.tools/github-cli/bin/gh.exe，未安装时可用系统gh；第一次认证使用gh auth login，账号为仓库发布者。公共下载不需要其他设备登录GitHub。脚本优先沿用环境代理或Windows系统代理。

正式安装器用当前用户安装及被更新程序的当前目录，不改变任务data文件夹。失败后保留任务库；不要为了更新而删除data或备份。网络受限时可手动安装新包。尚未进行Windows原生下载/替换/重启验收。

## 本次1.0.1发布

此版本按用户要求提高可见版本，以便手动体验应用内更新；未更改数据库Schema、迁移或业务行为。release-config.json的appVersion、package.json/lock、Cargo.toml/lock、tauri.conf.json需保持一致；构建脚本读取appVersion选择本地安装器。build.rs与签名检查会检查版本一致性。后续同一公开版本的小修改仍可只递增build。

静态检查与产物（2026-11-03）：npm run typecheck、cargo fmt --manifest-path src-tauri/Cargo.toml -- --check、受影响文件Prettier检查通过。npm run package:windows 成功生成1.0.1 Release EXE与NSIS包，内部构建202611030002；node scripts/verify-update.mjs通过签名和安装包兼容别名一致性核对。没有运行TaskDock、安装器、业务测试或真实数据库操作。应用内检查、下载、安装和重启由用户后续体验，不能据构建结果宣称验收通过。

## 1.1.0发布（2026-11-03）

按用户要求将软件、EXE与NSIS安装包版本升为1.1.0，内部构建202611030003；未更改任务业务、数据库Schema或迁移。更新协议继续沿用1.0.0+build及旧下载文件名兼容别名，供1.0.0/1.0.1客户端更新。

npm run typecheck、Rust格式检查及受影响配置/TypeScript文件Prettier检查通过；npm run package:windows构建成功，node scripts/verify-update.mjs核对签名及兼容安装包一致性通过。未运行软件、安装器、自动化测试或真实数据库操作，应用内安装重启由用户体验。

## 1.1.1发布（2026-11-03）

悬浮小组件新增锁形按钮，展开/收起时均可锁定，锁定后整体鼠标穿透；托盘“解锁悬浮窗”与主窗口设置提供解除入口。锁定状态重启保留。不透明度范围改为20%～100%。新增Schema8迁移，保留任务、旧设置及备份记录，不修改0001–0007迁移。实现和待体验项目见[锁定说明](widget-lock.md)。

实际执行：npm run typecheck、cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked、Rust格式检查及受影响前端文件Prettier检查通过；npm run package:windows成功生成1.1.1 EXE与NSIS包，内部构建202611030004；签名及安装包兼容别名核对通过。没有执行npm test/cargo test、GUI操作、启动软件或真实数据库迁移。新增/调整回归用例仅完成静态编译。实际鼠标穿透、解锁、DPI、迁移、安装重启仍待用户体验。
