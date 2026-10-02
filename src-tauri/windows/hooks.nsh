;; Runs after the uninstaller removed Sarab's files, registry keys and shortcuts.
;; Start with Windows lives outside the install folder, so remove it here; otherwise Windows
;; would try to start a deleted sarab.exe at every sign-in.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Sarab"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Sarab"
  ${EndIf}
!macroend
