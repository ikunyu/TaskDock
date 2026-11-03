; Task data follows the installation directory; keep directory choice visible.
!define MUI_DIRECTORYPAGE_TEXT_TOP "任务数据库和备份将保存在安装目录的 data 文件夹。请安装到 D、F 等非系统盘的可写目录；升级时沿用原目录。"
; Keep first launch an explicit user choice on the installer finish page.
!define MUI_FINISHPAGE_RUN_NOTCHECKED
; Preserve the user's existing opt-in, but update a portable/previous install
; path. Never create an autostart entry when the user has not enabled one.
!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${If} $R0 != ""
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --background"
  ${EndIf}
!macroend
; No hook removes or modifies the application database, backups or migration ledger.
