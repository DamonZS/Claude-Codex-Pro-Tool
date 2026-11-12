Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."
!define UNKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro"

; 过渡版：不再安装 CCP，而是下载并安装 Jianmu，成功后静默卸载旧 CCP。
; 文件名保持 claude-codex-pro-${VERSION}-windows-x64-setup.exe，旧 CCP 的更新器才会接受。
VIProductVersion "${VERSION}.0.0"
VIAddVersionKey "ProductName" "Claude Codex Pro"
VIAddVersionKey "FileDescription" "Claude Codex Pro 迁移到 Jianmu"
VIAddVersionKey "CompanyName" "TopoReduce"
VIAddVersionKey "LegalCopyright" "© 2026 TopoReduce"
VIAddVersionKey "FileVersion" "${VERSION}.0.0"
VIAddVersionKey "ProductVersion" "${VERSION}"

Name "Claude Codex Pro → Jianmu"
OutFile "${ROOT}\dist\windows\claude-codex-pro-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\jianmu"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "${ROOT}\apps\claude-codex-pro-manager\src-tauri\icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Migrate"
  InitPluginsDir
  File "/oname=$PLUGINSDIR\migrate.ps1" "migrate-to-jianmu.ps1"

  DetailPrint "正在下载并校验 Jianmu / Downloading and verifying Jianmu..."
  nsExec::ExecToStack 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\migrate.ps1" -Dest "$LOCALAPPDATA\Programs\jianmu\jianmu.exe"'
  Pop $0
  Pop $1
  StrCmp $0 "0" migrated
    MessageBox MB_OK|MB_ICONSTOP "迁移到 Jianmu 失败，已保留现有 Claude Codex Pro。$\r$\n原因：$1$\r$\n请从 https://github.com/DamonZS/Jianmu/releases 手动下载 Jianmu。"
    DetailPrint "失败：$1"
    Abort
  migrated:

  DetailPrint "Jianmu 已安装，正在卸载 Claude Codex Pro..."
  ReadRegStr $R0 HKCU "${UNKEY}" "UninstallString"
  ReadRegStr $R1 HKCU "${UNKEY}" "InstallLocation"
  StrCmp $R0 "" done_uninstall
  IfFileExists "$R0" 0 done_uninstall
    ExecWait '"$R0" /S _?=$R1'
    Delete "$R0"
    RMDir "$R1"
  done_uninstall:
  DeleteRegKey HKCU "${UNKEY}"

  Exec '"$LOCALAPPDATA\Programs\jianmu\jianmu.exe"'
SectionEnd
