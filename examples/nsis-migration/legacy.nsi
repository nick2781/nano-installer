; legacy.nsi -- the NSIS script that examples/nsis-migration/migrated/ was
; migrated from. It is representative rather than runnable: there is no `app\`,
; `docs\`, `samples\` or icon next to it, and it installs nothing. Its job is to
; carry one of every construct a real NSIS installer uses, so that
; scripts/check_nsi_migration.ps1 has something to classify and a reader can see
; each construct next to the row it maps to in docs/en/MIGRATION_FROM_NSIS.md.

!include "MUI2.nsh"
!include "LogicLib.nsh"

!define APP_NAME "LegacyApp"
!define APP_VERSION "3.4.1"
!define APP_PUBLISHER "Example Company"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

Name "${APP_NAME}"
OutFile "LegacyApp_Setup.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "${UNINST_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
SetRegView 64
ManifestDPIAware true
Icon "assets\app.ico"
UninstallIcon "assets\uninst.ico"
VIProductVersion "3.4.1.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "CompanyName" "${APP_PUBLISHER}"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "LegalCopyright" "Copyright 2026 Example Company"

Var StartMenuFolder

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "license.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

Function .onInit
  SetShellVarContext all
  ReadRegStr $0 HKLM "${UNINST_KEY}" "InstallLocation"
  ${If} $0 != ""
    StrCpy $INSTDIR $0
  ${EndIf}
FunctionEnd

Section "!Core files" SEC_CORE
  SectionIn RO
  SetOutPath "$INSTDIR"
  File /r "app\*.*"
  WriteUninstaller "$INSTDIR\uninst.exe"
  CreateDirectory "$INSTDIR\logs"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_NAME}.exe"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_NAME}.exe"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\uninst.exe"
  WriteRegStr HKLM "Software\${APP_NAME}" "InstallPath" "$INSTDIR"
  WriteRegExpandStr HKLM "Software\${APP_NAME}" "DataDir" "$APPDATA\${APP_NAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" "$INSTDIR\uninst.exe"
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1
  DetailPrint "Running the bundled migration tool"
  nsExec::ExecToLog '"$INSTDIR\tools\migrate.exe" --quiet'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "The migration tool failed with code $0."
    Abort
  ${EndIf}
  ExecWait '"$INSTDIR\vc_redist.x64.exe" /install /quiet /norestart' $1
  ExecShell "open" "https://example.test/legacyapp/first-run"
SectionEnd

Section /o "Documentation" SEC_DOCS
  SetOutPath "$INSTDIR\docs"
  File /r "docs\*.*"
  WriteINIStr "$INSTDIR\docs\index.ini" "docs" "installed" "1"
SectionEnd

Section /o "Sample projects" SEC_SAMPLES
  SetOutPath "$INSTDIR\samples"
  File /r "samples\*.*"
SectionEnd

Section "Uninstall"
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}"
  ${If} $0 != ""
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}"
  ${EndIf}
  RMDir /r "$INSTDIR\docs"
  RMDir /r "$INSTDIR\samples"
  Delete "$INSTDIR\*.log"
  RMDir /r "$INSTDIR"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  RMDir /r "$SMPROGRAMS\${APP_NAME}"
  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegKey HKCU "Software\${APP_NAME}"
SectionEnd

; Signing is the pipeline's step here too, and NSIS only hands the finished
; files over -- the same two hooks the builder carries.
!finalize 'signtool sign /fd sha256 /a "%1"'
!uninstfinalize 'signtool sign /fd sha256 /a "%1"'
