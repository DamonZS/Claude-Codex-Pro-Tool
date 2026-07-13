Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."

Name "Claude Codex Pro"
OutFile "${ROOT}\dist\windows\claude-codex-pro-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Claude Codex Pro"
InstallDirRegKey HKCU "Software\Claude Codex Pro" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!define MUI_ICON "${ROOT}\apps\claude-codex-pro-manager\src-tauri\icons\icon.ico"
!define MUI_UNICON "${ROOT}\apps\claude-codex-pro-manager\src-tauri\icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"

  nsExec::ExecToLog 'taskkill /IM claude-codex-pro.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-manager.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-mcp.exe /F'
  Pop $0

  File "${ROOT}\dist\windows\app\claude-codex-pro.exe"
  File "${ROOT}\dist\windows\app\claude-codex-pro-manager.exe"
  File "${ROOT}\dist\windows\app\claude-codex-pro-mcp.exe"

  Delete "$DESKTOP\Claude Codex Pro.lnk"
  Delete "$DESKTOP\Claude Codex Pro 管理工具.lnk"
  Delete "$SMPROGRAMS\Claude Codex Pro\Claude Codex Pro.lnk"
  Delete "$SMPROGRAMS\Claude Codex Pro\Claude Codex Pro 管理工具.lnk"
  Delete "$SMPROGRAMS\Claude Codex Pro\卸载 Claude Codex Pro.lnk"
  RMDir "$SMPROGRAMS\Claude Codex Pro"

  CreateShortcut "$DESKTOP\Claude Codex Pro.lnk" "$INSTDIR\claude-codex-pro.exe" "" "$INSTDIR\claude-codex-pro.exe"
  CreateShortcut "$DESKTOP\Claude Codex Pro Manager.lnk" "$INSTDIR\claude-codex-pro-manager.exe" "" "$INSTDIR\claude-codex-pro-manager.exe"
  CreateDirectory "$SMPROGRAMS\Claude Codex Pro"
  CreateShortcut "$SMPROGRAMS\Claude Codex Pro\Claude Codex Pro.lnk" "$INSTDIR\claude-codex-pro.exe" "" "$INSTDIR\claude-codex-pro.exe"
  CreateShortcut "$SMPROGRAMS\Claude Codex Pro\Claude Codex Pro Manager.lnk" "$INSTDIR\claude-codex-pro-manager.exe" "" "$INSTDIR\claude-codex-pro-manager.exe"
  CreateShortcut "$SMPROGRAMS\Claude Codex Pro\Uninstall Claude Codex Pro.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\claude-codex-pro-manager.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Claude Codex Pro" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro" "DisplayName" "Claude Codex Pro"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro" "Publisher" "DamonZS"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro" "DisplayIcon" "$INSTDIR\claude-codex-pro-manager.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro" "UninstallString" "$INSTDIR\uninstall.exe"

  nsExec::ExecToLog '"$INSTDIR\claude-codex-pro.exe" --register-installation --app-version "${VERSION}"'
  Pop $0
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-manager.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-mcp.exe /F'
  Pop $0

  Delete "$DESKTOP\Claude Codex Pro.lnk"
  Delete "$DESKTOP\Claude Codex Pro 管理工具.lnk"
  Delete "$SMPROGRAMS\Claude Codex Pro\Claude Codex Pro.lnk"
  Delete "$SMPROGRAMS\Claude Codex Pro\Claude Codex Pro 管理工具.lnk"
  Delete "$SMPROGRAMS\Claude Codex Pro\卸载 Claude Codex Pro.lnk"
  RMDir "$SMPROGRAMS\Claude Codex Pro"

  Delete "$INSTDIR\claude-codex-pro.exe"
  Delete "$INSTDIR\claude-codex-pro-manager.exe"
  Delete "$INSTDIR\claude-codex-pro-mcp.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ClaudeCodexPro"
  DeleteRegKey HKCU "Software\Claude Codex Pro"
SectionEnd
