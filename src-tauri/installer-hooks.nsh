!macro RepairShortcutIfExists SHORTCUT_PATH
  ${If} ${FileExists} "${SHORTCUT_PATH}"
    SetOutPath "$INSTDIR"
    CreateShortcut "${SHORTCUT_PATH}" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
    !insertmacro SetLnkAppUserModelId "${SHORTCUT_PATH}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro RepairShortcutIfExists "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    !insertmacro RepairShortcutIfExists "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif

  !insertmacro RepairShortcutIfExists "$DESKTOP\${PRODUCTNAME}.lnk"
!macroend
