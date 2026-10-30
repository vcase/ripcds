; RipBook NSIS hooks.
;
; When the uninstaller's "Delete the application data" checkbox is selected,
; remove RipBook manifests, rip logs, and stale work folders from the book
; library. The library lives outside %APPDATA%, so Tauri's built-in app-data
; cleanup does not see it.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\resources\purge-installer-data.ps1"'
    Pop $0
  ${EndIf}
!macroend
