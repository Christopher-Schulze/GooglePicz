; NSIS installer script for GooglePicz
!include "MUI2.nsh"

!define APP_NAME "GooglePicz"
!ifndef APP_VERSION
!define APP_VERSION "0.1.0"
!endif

Name "${APP_NAME} ${APP_VERSION}"
OutFile "${APP_NAME}Setup.exe"
InstallDir "$PROGRAMFILES\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallDir"

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Main"
  SetOutPath "$INSTDIR"
  File "..\target\release\googlepicz.exe"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\googlepicz.exe"
  WriteRegStr HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_NAME} ${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\googlepicz.exe"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$INSTDIR\Uninstall.exe"
  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
  RMDir "$INSTDIR"
SectionEnd

