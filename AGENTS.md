# TaskDock 项目位置与接手约定

- 当前项目根目录：E:\vibecoding\tododock。旧 F:\tododock 已不存在，E:\tododock 也不是项目根目录。
- 执行工具时显式使用当前E盘根目录作为workdir；先检查实际工作目录以及package.json、src-tauri/tauri.conf.json，不从旧聊天cwd猜测位置。
- 开始接手先读 HANDOFF.md 和 docs/path-migration-handoff.md，再核对 release-config.json 与Git状态。后续若再次迁移，以实际目录和用户最新说明为准，并更新路径交接。
- 用户可见的本地文件链接使用当前E盘绝对路径；仓库内文档和脚本尽量使用相对路径，避免重新写死失效盘符。
- 历史阶段报告中的F盘路径仅作历史证据，不能用作当前执行目录或交付链接。不得通过重建F盘目录来掩盖路径错误。
- 当前检查到应用1.2.2、Schema8；不按旧会话1.1.1信息回退代码或版本，不修改已应用迁移、不删除真实用户数据。
- 开发目录与运行数据目录不同；数据由实际EXE路径决定，不能为了改文档路径而搬动或重建数据库。
