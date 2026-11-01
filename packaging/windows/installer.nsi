; Claudium-Setup.exe: installs the folder CI stages (Claudium-Launcher.exe, game\, runtime\)
; for the current user, with Start menu and desktop shortcuts. No admin rights needed.
;   makensis /DSRC=<staged folder> /DOUT=<setup exe> installer.nsi
Unicode true
!include "MUI2.nsh"

Name "Claudium"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\Claudium"
InstallDirRegKey HKCU "Software\Claudium" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ICON "${SRC}\Claudium-Launcher.ico"
!define MUI_UNICON "${SRC}\Claudium-Launcher.ico"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\Claudium-Launcher.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Claudium"
  SetOutPath "$INSTDIR"
  ; A newer game replaces the old one; the launcher's data (accounts, prepared game) is
  ; elsewhere, in %APPDATA%\claudium-launcher, and stays.
  RMDir /r "$INSTDIR\game"
  RMDir /r "$INSTDIR\runtime"
  File /r "${SRC}\*.*"
  WriteRegStr HKCU "Software\Claudium" "InstallDir" "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$SMPROGRAMS\Claudium.lnk" "$INSTDIR\Claudium-Launcher.exe"
  CreateShortcut "$DESKTOP\Claudium.lnk" "$INSTDIR\Claudium-Launcher.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claudium" "DisplayName" "Claudium"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claudium" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claudium" "DisplayIcon" "$INSTDIR\Claudium-Launcher.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claudium" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claudium" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\Claudium.lnk"
  Delete "$DESKTOP\Claudium.lnk"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Claudium"
  DeleteRegKey HKCU "Software\Claudium"
SectionEnd
