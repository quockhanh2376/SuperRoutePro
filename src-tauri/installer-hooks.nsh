!macro NSIS_HOOK_PREINSTALL
  ; Force old version removal before new files are copied.
  ClearErrors
  ReadRegStr $R7 SHCTX "${UNINSTKEY}" "UninstallString"
  ReadRegStr $R8 SHCTX "${MANUPRODUCTKEY}" ""

  StrCmp "$R7" "" nsis_hook_done
  StrCmp "$R8" "" nsis_hook_done

  ; Run existing uninstaller in update/passive mode.
  StrCpy $R7 "$R7 /UPDATE /P _?=$R8"
  ExecWait '$R7' $R9

  ; Stop installation when old app still exists after uninstall attempt.
  StrCmp "$R9" "0" nsis_hook_done
  IfFileExists "$R8\${MAINBINARYNAME}.exe" 0 nsis_hook_done
  MessageBox MB_ICONEXCLAMATION|MB_OK "Cannot remove previous version automatically. Close the app and run installer again."
  Abort

nsis_hook_done:
!macroend
