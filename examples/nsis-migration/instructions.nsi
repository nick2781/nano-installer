; One line for every instruction, attribute and header macro NSIS itself
; documents, so the migration guide's table is held to the language rather than
; to the one representative script beside it.
;
; This file is not meant to be compiled: the paths it names do not exist, some
; of its lines are deliberately in the wrong place (`!error` outside a guard,
; an uninstall section in the installer), and no product is installed by it.
; `scripts/check_nsi_migration.ps1 -FailOnUnknown` reads it line by line, so a
; construct NSIS has and the table does not is a failing check rather than a
; `unknown` line a reader has to interpret.

; --- Installer attributes -------------------------------------------------

Name "Instructions"
OutFile "instructions.exe"
InstallDir "$PROGRAMFILES\Instructions"
InstallDirRegKey HKLM "Software\Instructions" "InstallLocation"
RequestExecutionLevel admin
Icon "app.ico"
UninstallIcon "uninst.ico"
WindowIcon on
VIProductVersion "1.0.0.0"
VIFileVersion "1.0.0.0"
VIAddVersionKey "ProductName" "Instructions"
VIAddVersionKey "LegalCopyright" "(c) nobody"
ManifestDPIAware true
ManifestSupportedOS all
ManifestLongPathAware true
PEAddResource "extra.bmp" "#2" "#1337"
PERemoveResource "#Icon" "#200" ALL
SetCompressor /SOLID lzma
SetCompressorDictSize 8
SetCompress auto
SetDatablockOptimize on
FileBufSize 32
CRCCheck on
Unicode true
Target amd64-unicode
XPStyle on
SetOverwrite on
SetDateSave on
AllowSkipFiles on
AllowRootDirInstall true
ShowInstDetails show
ShowUninstDetails show
SetDetailsPrint both
SetDetailsView show
AutoCloseWindow false
SetAutoClose true
BringToFront
LockWindow on
HideWindow
Nop
SetPluginUnload alwaysoff
SilentInstall normal
SilentUnInstall normal
BrandingText "Instructions"
SetBrandingImage "branding.bmp"
AddBrandingImage left 100
BGFont "Arial"
BGGradient 000000 FFFFFF
CheckBitmap "check.bmp"
ChangeUI all "ui.exe"
InstProgressFlags smooth colored
InstallColors 000000 FFFFFF
SetFont "Arial" 9
SetCtlColors $0 000000 FFFFFF
CreateFont $1 "Arial" 9 700
LicenseBkColor FFFFFF
LicenseText "Read this"
LicenseData "licence.txt"
LicenseForceSelection checkbox
Caption "Instructions"
SubCaption 0 ": Setup"
CompletedText "Done"
ComponentText "Components"
DirText "Pick a folder"
DirVar $INSTDIR
DirVerify auto
FileErrorText "Cannot write"
InstallButtonText "Install"
DetailsButtonText "Details"
MiscButtonText "<" ">" "Cancel" "Close"
SpaceTexts "required" "available"
UninstallButtonText "Remove"
UninstallCaption "Uninstall Instructions"
UninstallSubCaption 0 ": Confirm"
UninstallText "This removes Instructions"
InstType "Full"
InstTypeSetText 0 "Full"
InstTypeGetText 0 $0
LoadLanguageFile "${NSISDIR}\Contrib\Language files\English.nlf"
LangString TEXT_ONE ${LANG_ENGLISH} "one"

; --- Compile-time --------------------------------------------------------

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "StrFunc.nsh"
!include "WordFunc.nsh"
!include "TextFunc.nsh"
!include "Sections.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "nsDialogs.nsh"
!include "InstallOptions.nsh"
!define APP_NAME "Instructions"
!undef APP_NAME
!addincludedir "include"
!searchparse "1.2.3" "." MAJOR "." MINOR
!searchreplace VERSION "1.2.3" "." "_"
!tempfile SCRATCH
!delfile "${SCRATCH}"
!appendfile "build.log" "packed$\r$\n"
!getdllversion "app.dll" DLLVERSION
!cd "$%TEMP%"
!verbose 2
!echo "building"
!pragma warning push
!ifdef NEVER_DEFINED
  !error "this line is never read"
!else
  !warning "this one is read"
!endif
!if 1 > 0
!endif
!ifndef NEVER_DEFINED
!endif
!addplugindir "plugins"
!system "echo build-time" = 0
!execute "echo build-time"
!packhdr "temp.dat" "upx --best temp.dat"
!finalize 'signtool sign /fd sha256 /a "%1"'
!uninstfinalize 'signtool sign /fd sha256 /a "%1"'
!macro Greet WHO
  DetailPrint "hello ${WHO}"
!macroend
!insertmacro Greet "world"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "licence.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_STARTMENU Application $StartMenuFolder
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_LICENSE
!insertmacro MUI_UNPAGE_COMPONENTS
!insertmacro MUI_UNPAGE_DIRECTORY
!insertmacro MUI_LANGUAGE "English"

; --- Pages, variables and sections ---------------------------------------

Var /GLOBAL MyVariable
Page license
Page components
Page directory
Page instfiles
PageCustom MyPageCreate MyPageLeave
PageEx instfiles
PageExEnd
UninstPage uninstConfirm
UninstPage instfiles
UninstPage custom un.MyPageCreate

Section "Main" SEC_MAIN
  SectionIn RO
  SectionInGroup 1
  SectionInstType 0
  AddSize 1024

  SetOutPath "$INSTDIR"
  CreateDirectory "$INSTDIR\sub"
  File "app.exe"
  File /r "payload\*.*"
  CopyFiles "$INSTDIR\app.exe" "$INSTDIR\app2.exe"
  Rename "$INSTDIR\app2.exe" "$INSTDIR\app3.exe"
  Delete "$INSTDIR\old.log"
  Delete /REBOOTOK "$INSTDIR\locked.dll"
  RMDir /r "$INSTDIR\old"
  SetFileAttributes "$INSTDIR\app.exe" NORMAL
  GetFileAttributes "$INSTDIR\app.exe" $0
  GetSize "$INSTDIR\app.exe" $1
  GetFileTime "$INSTDIR\app.exe" $2 $3
  GetFileTimeLocal "$INSTDIR\app.exe" $4 $5
  GetDLLVersion "$INSTDIR\app.dll" $6
  GetDLLVersionLocal "app.dll" $7
  ${GetFileVersion} "$INSTDIR\app.exe" $0
  GetFullPathName $8 "$INSTDIR\..\x"
  SearchPath $9 "cmd.exe"
  GetTempFileName $R0
  ${GetTime} "" "L" "Y M D" $R1 $R2 $R3
  ${GetDrives} "HDD" MyDriveCallback
  ${DriveSpace} "C:\" "/D=F" $R4
  ${GetRoot} "C:\Windows" $R5
  ${GetFileName} "C:\Windows\notepad.exe" $R6
  ${GetBaseName} "C:\Windows\notepad.exe" $R7
  FindFirst $R8 $R9 "$INSTDIR\*.*"
  FindNext $R8 $R9
  FindClose $R8

  WriteRegStr HKLM "Software\Instructions" "Path" "$INSTDIR"
  WriteRegExpandStr HKLM "Software\Instructions" "Data" "%TEMP%"
  WriteRegDWORD HKLM "Software\Instructions" "Number" 1
  WriteRegBin HKLM "Software\Instructions" "Blob" 0011
  WriteRegNone HKLM "Software\Instructions" "Empty"
  ReadRegStr $0 HKLM "Software\Instructions" "Path"
  ReadRegDWORD $1 HKLM "Software\Instructions" "Number"
  DeleteRegValue HKLM "Software\Instructions" "Number"
  DeleteRegKey HKLM "Software\Instructions"
  EnumRegKey $2 HKLM "Software" 0
  EnumRegValue $3 HKLM "Software\Instructions" 0
  SetRegView 64

  WriteINIStr "$INSTDIR\app.ini" "section" "key" "value"
  ReadINIStr $4 "$INSTDIR\app.ini" "section" "key"
  DeleteINISec "$INSTDIR\app.ini" "section"
  DeleteINIStr "$INSTDIR\app.ini" "section" "key"
  FlushINI "$INSTDIR\app.ini"

  FileOpen $5 "$INSTDIR\log.txt" w
  FileWrite $5 "line"
  FileWriteByte $5 "13"
  FileRead $5 $6
  FileReadByte $5 $7
  FileSeek $5 0 SET
  FileClose $5

  CreateShortCut "$DESKTOP\Instructions.lnk" "$INSTDIR\app.exe"
  WriteUninstaller "$INSTDIR\uninst.exe"
  RegDLL "$INSTDIR\shell.dll"
  UnRegDLL "$INSTDIR\shell.dll"

  Exec '"$INSTDIR\app.exe"'
  ExecWait '"$INSTDIR\app.exe" /silent' $8
  ExecShell "open" "https://example.test/"
  ExecShellWait "open" "$INSTDIR\readme.txt"

  DetailPrint "installing"
  LogText "installing"
  LogSet on
  Sleep 500
  MessageBox MB_OK "hello"
  GetInstDirError $9
  InitPluginsDir
  GetDlgItem $R0 $HWNDPARENT 1
  SendMessage $R1 0 0 0
  ShowWindow $R2 1
  EnableWindow $R3 1
  SetShellVarContext all
  SetSilent silent
  SetAutoClose true
  SetErrors
  GetErrorLevel $R4
  ClearErrors
  SetErrorLevel 0

  ExpandEnvStrings $R5 "%TEMP%"
  ReadEnvStr $R6 "PATH"
  ReadEnvStr $R7 "COMSPEC"

  ${If} $0 == "one"
    DetailPrint "one"
  ${ElseIf} $0 == "two"
    DetailPrint "two"
  ${Else}
    DetailPrint "other"
  ${EndIf}
  ${Unless} $0 == "three"
  ${EndUnless}
  ${IfNot} $0 == "four"
  ${EndIf}
  ${AndIf} $0 == "five"
  ${OrIf} $0 == "six"
  ${AndUnless} $0 == "seven"
  ${OrUnless} $0 == "eight"
  ${While} $0 == "nine"
  ${EndWhile}
  ${Do}
  ${Loop}
  ${Until} $0 == "ten"
  ${DoWhile} $0 == "eleven"
  ${For} $0 0 10
  ${Next}
  ${ForEach} $0 in "a b c"
  ${Next}
  ${Break}
  ${Continue}
  ${Select} $0
    ${Case} "one"
    ${CaseElse}
    ${Default}
  ${EndSelect}
  ${Switch} $0
    ${Case} "two"
  ${EndSwitch}

  Var /GLOBAL StackVar
  Push "value"
  Pop $StackVar
  Exch $StackVar
  StrCpy $StackVar "text"
  StrLen $R8 $StackVar
  StrCmp $StackVar "text" done same
  StrCmpS $StackVar "text" done same
  StrICmp $StackVar "TEXT" done same
  IntOp $R9 $R9 + 1
  IntCmp $R9 1 done same same
  IntCmpU $R9 1 done same same
  IntFmt $R9 "%05d" 3
  ${StrStr} $R9 "abcdef" "cd"
  ${StrRep} $R9 "a-b-c" "-" "+"
  ${StrLoc} $R9 "abcdef" "cd" ">"
  ${StrSort} $R9 "b a c" "" "" ""
  ${StrTrimNewLines} $R9 "text$\r$\n"
  ${WordFind} "a b c" " " "+1" $R9
  ${WordReplace} "a b c" " " "-" "+" $R9
  ${WordInsert} "a b c" " " 2 "x" $R9
  ${WordDelete} "a b c" " " 2 $R9
  ${TextCompare} "a" "b" "Same" "Diff" $R9
  ${LineFind} "in.txt" "/NUL" "1:-1" "Callback"
  ${LineRead} "in.txt" "1" $R9
  ${LineSum} "in.txt" $R9
  ${FileReadFromEnd} "in.txt" "Callback"
  ${FileJoin} "out.txt" "a.txt" "b.txt"
  ${VersionCompare} "1.0.0.0" "2.0.0.0" $R9
  ${VersionConvert} "1.2.3" "0123456789" $R9
  ${GetParameters} $R9
  ${GetOptions} $R9 "/D=" $R0
  ${AtLeastWin7} $R0
  ${IsWin10} $R0
  ${RunningX64} $R0
  ${DisableX64FSRedirection} $R0
  ${EnableX64FSRedirection} $R0
  ${GetParent} "$INSTDIR\x" $R0

  Goto done
  IfErrors failed done
  IfFileExists "$INSTDIR\app.exe" done failed
  IfAbort done failed
  IfRebootFlag done failed
  IfSilent done failed
  GetLabelAddress $R0 done
  GetFunctionAddress $R0 MyFunction
  GetCurrentAddress $R0
  SetRebootFlag true
  Reboot
  Call MyFunction
  Abort
  Quit

  done:
  same:
  failed:
  same:
SectionEnd

Section /o "Optional" SEC_OPTIONAL
SectionEnd

SectionGroup "Group"
  Section "Grouped" SEC_GROUPED
  SectionEnd
SectionGroupEnd

Section "Uninstall"
  Delete "$INSTDIR\app.exe"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKLM "Software\Instructions"
SectionEnd

SectionSetFlags ${SEC_MAIN} ${SF_SELECTED}
SectionGetFlags ${SEC_MAIN} $0
SectionSetText ${SEC_MAIN} "Main"
SectionGetText ${SEC_MAIN} $0
SectionSetSize ${SEC_MAIN} 1024
SectionGetSize ${SEC_MAIN} $0
SectionSetInstTypes ${SEC_MAIN} 1
SectionGetInstTypes ${SEC_MAIN} $0
SetCurInstType 0
GetCurInstType $0

; --- Functions and callbacks ---------------------------------------------

Function MyPageCreate
FunctionEnd

Function MyPageLeave
FunctionEnd

Function MyFunction
FunctionEnd

Function MyDriveCallback
FunctionEnd

Function .onInit
  ${If} ${AtLeastWinVista}
    DetailPrint "vista or later"
  ${EndIf}
FunctionEnd

Function .onGUIInit
FunctionEnd

Function .onInstSuccess
FunctionEnd

Function .onInstFailed
FunctionEnd

Function .onGUIEnd
FunctionEnd

Function .onMouseOverSection
FunctionEnd

Function .onSelChange
FunctionEnd

Function .onRebootFailed
FunctionEnd

Function un.onInit
FunctionEnd

Function un.onUninstSuccess
FunctionEnd

Function un.onUninstFailed
FunctionEnd

Function un.onGUIEnd
FunctionEnd

; --- Plugins -------------------------------------------------------------

Section "Plugins" SEC_PLUGINS
  nsExec::Exec "cmd.exe"
  nsExec::ExecToLog "cmd.exe"
  nsExec::ExecToStack "cmd.exe"
  nsExec::ExecShellEx "open" "https://example.test/"
  nsProcess::_FindProcess "app.exe"
  Pop $0
  nsProcess::_KillProcess "app.exe"
  Pop $0
  EnvVar::set "INSTRUCTIONS" "1"
  EnvVar::unset "INSTRUCTIONS"
  nsDialogs::Create 1018
  Pop $0
  nsDialogs::Show
  InstallOptions::dialog "$PLUGINSDIR\options.ini"
  Pop $0
  UAC::RunElevated
  Pop $0
  System::Call "kernel32::GetTickCount()i.R0"
  nsisXML::create
SectionEnd
