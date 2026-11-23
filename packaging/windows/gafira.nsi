; Gafira Windows installer (spec 090): NSIS 3 and Modern UI 2.
;
; From the root of the repository, with -NOCD, after staging the executable
; as dist/windows-x64/Gafira.exe (packaging/windows/build.sh does it all):
;
;   makensis -NOCD -DAPP_VERSION=0.1.0-beta.3 -DAPP_NUMBERS=0.1.0.3 \
;            -DREPO_ROOT="$PWD" packaging/windows/gafira.nsi
;
; It is not signed: a certificate of our own would still say «unknown
; publisher». Signing is a decision of a later release.

Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef REPO_ROOT
  !error "Define REPO_ROOT, the root of the repository"
!endif
!ifndef APP_VERSION
  !error "Define APP_VERSION"
!endif
; Four numbers, as Windows wants them: the last one is of the beta.
!ifndef APP_NUMBERS
  !error "Define APP_NUMBERS"
!endif
; NSIS on Windows does not find files with forward slashes: SEP is the other there.
!ifndef SEP
  !define SEP "/"
!endif

!define LANG_ENGLISH 1033
!define LANG_SPANISH 1034

!define APP "Gafira"
!define EXE "Gafira.exe"
!define PUBLISHER "Gustavo Gutierrez"
!define KEY "Software\Gafira"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Gafira"
!define EXT ".gaf"
!define CLASS "Gafira.Project"
!define MIME "application/vnd.gafira.project+zip"
!define KIT_EXT ".gafkit"
!define KIT_CLASS "Gafira.BrandKit"
!define KIT_MIME "application/vnd.gafira.brandkit+zip"
!define ICONS "${REPO_ROOT}${SEP}assets${SEP}app-icon${SEP}windows"
!define PICTURES "${REPO_ROOT}${SEP}packaging${SEP}windows${SEP}assets"

Name "${APP} ${APP_VERSION}"
OutFile "${REPO_ROOT}${SEP}dist${SEP}Gafira-${APP_VERSION}-Setup-x64.exe"
InstallDir "$PROGRAMFILES64\Gafira"
InstallDirRegKey HKLM "${KEY}" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
BrandingText "${APP} ${APP_VERSION}"
ManifestDPIAware true

VIProductVersion "${APP_NUMBERS}"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "FileDescription" "${APP} Setup"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 ${PUBLISHER}. All rights reserved."

!define MUI_ICON "${ICONS}${SEP}gafira.ico"
!define MUI_UNICON "${ICONS}${SEP}gafira.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "${PICTURES}${SEP}header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "${PICTURES}${SEP}welcome.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${PICTURES}${SEP}welcome.bmp"
!define MUI_ABORTWARNING
!define MUI_LANGDLL_REGISTRY_ROOT "HKLM"
!define MUI_LANGDLL_REGISTRY_KEY "${KEY}"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Language"
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "$(RUN_TEXT)"
!define MUI_FINISHPAGE_RUN_FUNCTION RunGafira
!define MUI_WELCOMEPAGE_TEXT "$(WELCOME_TEXT)"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${REPO_ROOT}${SEP}LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Spanish"

LangString WELCOME_TEXT ${LANG_ENGLISH} "This wizard will install ${APP} ${APP_VERSION} on your computer.$\r$\n$\r$\nGafira is a design editor for social media posts, presentations, print and video. Projects are saved as .gaf files."
LangString WELCOME_TEXT ${LANG_SPANISH} "Este asistente instalará ${APP} ${APP_VERSION} en tu equipo.$\r$\n$\r$\nGafira es un editor de diseño para redes sociales, presentaciones, impresión y vídeo. Los proyectos se guardan como archivos .gaf."
LangString RUN_TEXT ${LANG_ENGLISH} "Open Gafira"
LangString RUN_TEXT ${LANG_SPANISH} "Abrir Gafira"
LangString UNINST_LINK ${LANG_ENGLISH} "Uninstall Gafira"
LangString UNINST_LINK ${LANG_SPANISH} "Desinstalar Gafira"
LangString PROJECT ${LANG_ENGLISH} "Gafira project"
LangString PROJECT ${LANG_SPANISH} "Proyecto de Gafira"
LangString BRANDKIT ${LANG_ENGLISH} "Gafira brand kit"
LangString BRANDKIT ${LANG_SPANISH} "Kit de marca de Gafira"
LangString X64_ONLY ${LANG_ENGLISH} "Gafira requires a 64-bit version of Windows."
LangString X64_ONLY ${LANG_SPANISH} "Gafira requiere una versión de Windows de 64 bits."

Function .onInit
  SetRegView 64
  SetShellVarContext all
  !insertmacro MUI_LANGDLL_DISPLAY
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "$(X64_ONLY)" /SD IDOK
    Abort
  ${EndIf}
FunctionEnd

Function un.onInit
  SetRegView 64
  SetShellVarContext all
  !insertmacro MUI_UNGETLANGUAGE
FunctionEnd

; Through Explorer: the application does not inherit the administrator of the
; installer.
Function RunGafira
  Exec '"$WINDIR\explorer.exe" "$INSTDIR\${EXE}"'
FunctionEnd

Section "Gafira"
  SectionIn RO
  SetRegView 64
  SetShellVarContext all

  SetOutPath "$INSTDIR"
  File "${REPO_ROOT}${SEP}dist${SEP}windows-x64${SEP}${EXE}"
  File "${ICONS}${SEP}gafira-project.ico"
  File "${ICONS}${SEP}gafira-brandkit.ico"
  SetOutPath "$INSTDIR\legal"
  File /oname=LICENSE.txt "${REPO_ROOT}${SEP}LICENSE"
  File "${REPO_ROOT}${SEP}THIRD_PARTY_NOTICES.md"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Gafira"
  CreateShortcut "$SMPROGRAMS\Gafira\Gafira.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0
  CreateShortcut "$SMPROGRAMS\Gafira\$(UNINST_LINK).lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\Gafira.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0

  WriteRegStr HKLM "${KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${EXE},0"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKLM "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  SectionGetSize 0 $0
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "EstimatedSize" "$0"
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1

  ; The .gaf files: their type, their icon and what opens them.
  WriteRegStr HKLM "Software\Classes\${EXT}" "" "${CLASS}"
  WriteRegStr HKLM "Software\Classes\${EXT}" "Content Type" "${MIME}"
  WriteRegStr HKLM "Software\Classes\${EXT}\OpenWithProgids" "${CLASS}" ""
  WriteRegStr HKLM "Software\Classes\${CLASS}" "" "$(PROJECT)"
  WriteRegStr HKLM "Software\Classes\${CLASS}" "Content Type" "${MIME}"
  WriteRegStr HKLM "Software\Classes\${CLASS}\DefaultIcon" "" "$INSTDIR\gafira-project.ico,0"
  WriteRegStr HKLM "Software\Classes\${CLASS}\shell\open\command" "" '"$INSTDIR\${EXE}" "%1"'
  ; The .gafkit brand kits (spec 094): opening one imports it.
  WriteRegStr HKLM "Software\Classes\${KIT_EXT}" "" "${KIT_CLASS}"
  WriteRegStr HKLM "Software\Classes\${KIT_EXT}" "Content Type" "${KIT_MIME}"
  WriteRegStr HKLM "Software\Classes\${KIT_EXT}\OpenWithProgids" "${KIT_CLASS}" ""
  WriteRegStr HKLM "Software\Classes\${KIT_CLASS}" "" "$(BRANDKIT)"
  WriteRegStr HKLM "Software\Classes\${KIT_CLASS}" "Content Type" "${KIT_MIME}"
  WriteRegStr HKLM "Software\Classes\${KIT_CLASS}\DefaultIcon" "" "$INSTDIR\gafira-brandkit.ico,0"
  WriteRegStr HKLM "Software\Classes\${KIT_CLASS}\shell\open\command" "" '"$INSTDIR\${EXE}" "%1"'
  WriteRegStr HKLM "Software\Classes\Applications\${EXE}\SupportedTypes" "${KIT_EXT}" ""
  WriteRegStr HKLM "${KEY}\Capabilities\FileAssociations" "${KIT_EXT}" "${KIT_CLASS}"
  WriteRegStr HKLM "Software\Classes\Applications\${EXE}" "FriendlyAppName" "${APP}"
  WriteRegStr HKLM "Software\Classes\Applications\${EXE}\SupportedTypes" "${EXT}" ""
  WriteRegStr HKLM "Software\Classes\Applications\${EXE}\shell\open\command" "" '"$INSTDIR\${EXE}" "%1"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}" "" "$INSTDIR\${EXE}"
  ; «Aplicaciones predeterminadas» of Windows.
  WriteRegStr HKLM "${KEY}\Capabilities" "ApplicationName" "${APP}"
  WriteRegStr HKLM "${KEY}\Capabilities" "ApplicationDescription" "${APP}"
  WriteRegStr HKLM "${KEY}\Capabilities\FileAssociations" "${EXT}" "${CLASS}"
  WriteRegStr HKLM "Software\RegisteredApplications" "${APP}" "${KEY}\Capabilities"
  ; Explorer shows it at once.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  SetRegView 64
  SetShellVarContext all

  Delete "$DESKTOP\Gafira.lnk"
  Delete "$SMPROGRAMS\Gafira\Gafira.lnk"
  ; In the language it was installed in, or another.
  Delete "$SMPROGRAMS\Gafira\Uninstall Gafira.lnk"
  Delete "$SMPROGRAMS\Gafira\Desinstalar Gafira.lnk"
  RMDir "$SMPROGRAMS\Gafira"

  ; What was installed, and no more: the projects and the settings stay.
  Delete "$INSTDIR\legal\LICENSE.txt"
  Delete "$INSTDIR\legal\THIRD_PARTY_NOTICES.md"
  RMDir "$INSTDIR\legal"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\gafira-project.ico"
  Delete "$INSTDIR\gafira-brandkit.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  ; The type of the files, if it is still the one of Gafira.
  ReadRegStr $0 HKLM "Software\Classes\${EXT}" ""
  ${If} $0 == "${CLASS}"
    DeleteRegKey HKLM "Software\Classes\${EXT}"
  ${Else}
    DeleteRegValue HKLM "Software\Classes\${EXT}\OpenWithProgids" "${CLASS}"
  ${EndIf}
  DeleteRegKey HKLM "Software\Classes\${CLASS}"
  ReadRegStr $0 HKLM "Software\Classes\${KIT_EXT}" ""
  ${If} $0 == "${KIT_CLASS}"
    DeleteRegKey HKLM "Software\Classes\${KIT_EXT}"
  ${Else}
    DeleteRegValue HKLM "Software\Classes\${KIT_EXT}\OpenWithProgids" "${KIT_CLASS}"
  ${EndIf}
  DeleteRegKey HKLM "Software\Classes\${KIT_CLASS}"
  DeleteRegKey HKLM "Software\Classes\Applications\${EXE}"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}"
  DeleteRegValue HKLM "Software\RegisteredApplications" "${APP}"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
  DeleteRegKey HKLM "${KEY}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
