!macro STOP_REPAIR_AND_ROUTE_SERVICE label_prefix
  ; Terminate any running Repair Broker helper and stop the route service cleanly.
  ClearErrors
  ExecWait '"$SYSDIR\taskkill.exe" /F /IM SuperRouteRepairBroker.exe'
  ClearErrors
  ExecWait '"$SYSDIR\sc.exe" stop SuperRouteProRouteService'

  ; Poll until SuperRouteProRouteService is no longer running (up to 10 seconds).
  StrCpy $R0 0
${label_prefix}_wait_loop:
  IntCmp $R0 20 ${label_prefix}_wait_done ${label_prefix}_wait_check ${label_prefix}_wait_done
${label_prefix}_wait_check:
  ClearErrors
  ExecWait '"$SYSDIR\cmd.exe" /C ""$SYSDIR\sc.exe" query SuperRouteProRouteService | "$SYSDIR\findstr.exe" /I /C:"RUNNING" /C:"STOP_PENDING" /C:"START_PENDING" >nul 2>&1"' $R1
  StrCmp "$R1" "0" 0 ${label_prefix}_wait_done
  Sleep 500
  IntOp $R0 $R0 + 1
  Goto ${label_prefix}_wait_loop
${label_prefix}_wait_done:
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; Stop the repair broker and route service before replacing installed files during update.
  !insertmacro STOP_REPAIR_AND_ROUTE_SERVICE preinstall

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

!macro NSIS_HOOK_PREUNINSTALL
  ; Stop the repair broker and remove the route service before deleting installed binaries.
  !insertmacro STOP_REPAIR_AND_ROUTE_SERVICE preuninstall
  ClearErrors
  ExecWait '"$SYSDIR\sc.exe" delete SuperRouteProRouteService'
!macroend
