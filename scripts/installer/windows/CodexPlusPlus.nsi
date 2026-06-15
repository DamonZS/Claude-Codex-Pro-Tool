Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."

Name "Codex++"
OutFile "${ROOT}\dist\windows\CodexPlusPlus-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Codex++"
InstallDirRegKey HKCU "Software\Codex++" "InstallDir"
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

  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-plus.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-plus-manager.exe /F'
  Pop $0

  File "${ROOT}\dist\windows\app\claude-codex-pro-plus.exe"
  File "${ROOT}\dist\windows\app\claude-codex-pro-plus-manager.exe"

  Delete "$DESKTOP\Codex++ 管理工具.lnk"
  Delete "$SMPROGRAMS\Codex++\Codex++ 管理工具.lnk"

  CreateShortcut "$DESKTOP\Codex++.lnk" "$INSTDIR\claude-codex-pro-plus.exe" "" "$INSTDIR\claude-codex-pro-plus.exe"
  CreateShortcut "$DESKTOP\Codex++ 管理工具.lnk" "$INSTDIR\claude-codex-pro-plus-manager.exe" "" "$INSTDIR\claude-codex-pro-plus-manager.exe"
  CreateDirectory "$SMPROGRAMS\Codex++"
  CreateShortcut "$SMPROGRAMS\Codex++\Codex++.lnk" "$INSTDIR\claude-codex-pro-plus.exe" "" "$INSTDIR\claude-codex-pro-plus.exe"
  CreateShortcut "$SMPROGRAMS\Codex++\Codex++ 管理工具.lnk" "$INSTDIR\claude-codex-pro-plus-manager.exe" "" "$INSTDIR\claude-codex-pro-plus-manager.exe"
  CreateShortcut "$SMPROGRAMS\Codex++\卸载 Codex++.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\claude-codex-pro-plus-manager.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Codex++" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++" "DisplayName" "Codex++"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++" "Publisher" "DamonZS"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++" "DisplayIcon" "$INSTDIR\claude-codex-pro-plus-manager.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-plus.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM claude-codex-pro-plus-manager.exe /F'
  Pop $0

  Delete "$DESKTOP\Codex++.lnk"
  Delete "$DESKTOP\Codex++ 管理工具.lnk"
  Delete "$SMPROGRAMS\Codex++\Codex++.lnk"
  Delete "$SMPROGRAMS\Codex++\Codex++ 管理工具.lnk"
  Delete "$SMPROGRAMS\Codex++\卸载 Codex++.lnk"
  RMDir "$SMPROGRAMS\Codex++"

  Delete "$INSTDIR\claude-codex-pro-plus.exe"
  Delete "$INSTDIR\claude-codex-pro-plus-manager.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex++"
  DeleteRegKey HKCU "Software\Codex++"
SectionEnd
