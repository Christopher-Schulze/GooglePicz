; NSIS installer script for GooglePicz
!include "MUI2.nsh"

!define APP_NAME "GooglePicz"

; These version components must be provided by the packager via /D options
!ifndef APP_VERSION_MAJOR
!error "APP_VERSION_MAJOR not defined"
!endif
!ifndef APP_VERSION_MINOR
!error "APP_VERSION_MINOR not defined"
!endif
!ifndef APP_VERSION_PATCH
!error "APP_VERSION_PATCH not defined"
!endif
!define APP_VERSION "${APP_VERSION_MAJOR}.${APP_VERSION_MINOR}.${APP_VERSION_PATCH}"

!define BUILD_DIR "..\\target\\release"
RequestExecutionLevel admin

Name "${APP_NAME} ${APP_VERSION}"
OutFile "..\\target\\windows\\${APP_NAME}-${APP_VERSION}-Setup.exe"
InstallDir "$PROGRAMFILES\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallDir"

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

VIProductVersion "${APP_VERSION_MAJOR}.${APP_VERSION_MINOR}.${APP_VERSION_PATCH}"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} Installer"

Section "Main"
  SetOutPath "$INSTDIR"
  File "${BUILD_DIR}\googlepicz.exe"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\googlepicz.exe"
  WriteRegStr HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_NAME} ${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Uninstall" SEC_UNINSTALL
  Delete "$INSTDIR\googlepicz.exe"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$INSTDIR\Uninstall.exe"
  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
  RMDir "$INSTDIR"
SectionEnd

