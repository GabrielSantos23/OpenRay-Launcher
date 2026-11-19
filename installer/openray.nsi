Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma

!ifndef VERSION
  !error "Pass /DVERSION=x.y.z"
!endif
!ifndef SOURCE
  !error "Pass /DSOURCE=<folder with openray.exe, LICENSE and extensions>"
!endif
!ifndef OUTFILE
  !define OUTFILE "openray-windows-x64-setup.exe"
!endif

!define PRODUCT "Openray"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Openray"
!define RUNKEY "Software\Microsoft\Windows\CurrentVersion\Run"

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

Name "${PRODUCT}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Openray"
RequestExecutionLevel user
BrandingText "${PRODUCT} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${PRODUCT} Setup"
VIAddVersionKey "LegalCopyright" "MIT License"

!define MUI_ICON "..\app\assets\app-icons\icon.ico"
!define MUI_UNICON "..\app\assets\app-icons\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\openray.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "--show"

!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Var Restart

Function .onInit
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/R" $R1
  ${IfNot} ${Errors}
    StrCpy $Restart 1
  ${EndIf}
  ; The Tauri updater launches the installer with /P (passive) and /UPDATE, and expects it to finish unattended.
  ClearErrors
  ${GetOptions} $R0 "/P" $R1
  ${IfNot} ${Errors}
    SetSilent silent
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/UPDATE" $R1
  ${IfNot} ${Errors}
    SetSilent silent
  ${EndIf}
FunctionEnd

Function RemoveTauriInstall
  ReadRegStr $0 HKCU "${UNINSTKEY}" "MainBinaryName"
  ${If} $0 == ""
    Return
  ${EndIf}
  ReadRegStr $1 HKCU "${UNINSTKEY}" "InstallLocation"
  ${If} $1 == ""
  ${OrIfNot} ${FileExists} "$1\uninstall.exe"
    DeleteRegKey HKCU "${UNINSTKEY}"
    Return
  ${EndIf}
  DetailPrint "Removing the previous Openray (Tauri) from $1"
  ; /UPDATE makes the Tauri uninstaller keep the app data in %APPDATA%\dev.openray.app, which this version reads.
  ExecWait '"$1\uninstall.exe" /S /UPDATE _?=$1'
  Delete "$1\uninstall.exe"
  RMDir "$1"
  DeleteRegKey HKCU "${UNINSTKEY}"
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  ${If} ${FileExists} "$DESKTOP\${PRODUCT}.lnk"
    Delete "$DESKTOP\${PRODUCT}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCT}.lnk" "$INSTDIR\openray.exe"
  ${EndIf}
FunctionEnd

Section "Openray"
  nsExec::Exec 'taskkill /F /IM openray.exe'
  Sleep 500
  ReadRegStr $2 HKCU "${RUNKEY}" "${PRODUCT}"
  Call RemoveTauriInstall

  SetOutPath "$INSTDIR"
  File "${SOURCE}\openray.exe"
  File "${SOURCE}\LICENSE"
  SetOutPath "$INSTDIR\extensions"
  File /r "${SOURCE}\extensions\*"
  SetOutPath "$INSTDIR"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\openray.exe"

  ; Open at Login stays on: the old value pointed at the Tauri exe the uninstaller just removed.
  ${If} $2 != ""
    WriteRegStr HKCU "${RUNKEY}" "${PRODUCT}" '"$INSTDIR\openray.exe"'
  ${EndIf}

  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCT}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${PRODUCT}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" '"$INSTDIR\openray.exe"'
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTKEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
SectionEnd

Function .onInstSuccess
  ${If} $Restart == 1
    Exec '"$INSTDIR\openray.exe" --show'
  ${EndIf}
FunctionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /F /IM openray.exe'
  Sleep 500
  Delete "$INSTDIR\openray.exe"
  Delete "$INSTDIR\openray.old.exe"
  Delete "$INSTDIR\LICENSE"
  RMDir /r "$INSTDIR\extensions"
  RMDir /r "$INSTDIR\.update"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  ReadRegStr $0 HKCU "${RUNKEY}" "${PRODUCT}"
  ${If} $0 == '"$INSTDIR\openray.exe"'
    DeleteRegValue HKCU "${RUNKEY}" "${PRODUCT}"
  ${EndIf}
  DeleteRegKey HKCU "${UNINSTKEY}"
SectionEnd
