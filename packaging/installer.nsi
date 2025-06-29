; NSIS installer script for GooglePicz
OutFile "GooglePiczSetup.exe"
InstallDir "$PROGRAMFILES\GooglePicz"
Section "Main"
  SetOutPath "$INSTDIR"
  File "..\target\release\googlepicz.exe"
  CreateShortCut "$DESKTOP\GooglePicz.lnk" "$INSTDIR\googlepicz.exe"
SectionEnd

