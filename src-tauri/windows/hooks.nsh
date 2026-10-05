;; Runs after the uninstaller removed Sarab's files, registry keys and shortcuts.
;; Start with Windows lives outside the install folder, so remove it here; otherwise Windows
;; would try to start a deleted sarab.exe at every sign-in.
;; `sarab` works in any new terminal. Sarab edits PATH itself: NSIS strings stop at 1024
;; characters and would cut a long PATH short.
!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$INSTDIR\sarab.exe" --add-to-path'
!macroend

;; Before the files go, while sarab.exe can still run. An update keeps the PATH entry.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    ExecWait '"$INSTDIR\sarab.exe" --remove-from-path'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Sarab"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Sarab"
  ${EndIf}
!macroend
