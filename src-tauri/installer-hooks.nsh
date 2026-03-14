!define SRP_REPAIR_SERVICE_NAME "SuperRouteRepairService"
!define SRP_REPAIR_SERVICE_DISPLAY_NAME "Super Route Pro Repair Service"

!macro SRP_STOP_REPAIR_SERVICE
  ClearErrors
  ExecWait '"$SYSDIR\sc.exe" stop "${SRP_REPAIR_SERVICE_NAME}"' $R0
!macroend

!macro SRP_DELETE_REPAIR_SERVICE
  ClearErrors
  ExecWait '"$SYSDIR\sc.exe" delete "${SRP_REPAIR_SERVICE_NAME}"' $R0
!macroend

!macro SRP_CREATE_REPAIR_SERVICE
  ClearErrors
  ExecWait '"$SYSDIR\sc.exe" create "${SRP_REPAIR_SERVICE_NAME}" binPath= "\"$INSTDIR\SuperRouteRepairService.exe\"" start= auto DisplayName= "${SRP_REPAIR_SERVICE_DISPLAY_NAME}"' $R0
  ExecWait '"$SYSDIR\sc.exe" description "${SRP_REPAIR_SERVICE_NAME}" "Privileged repair actions for Super Route Pro."' $R1
!macroend

!macro SRP_START_REPAIR_SERVICE
  ClearErrors
  ExecWait '"$SYSDIR\sc.exe" start "${SRP_REPAIR_SERVICE_NAME}"' $R0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro SRP_STOP_REPAIR_SERVICE
  !insertmacro SRP_DELETE_REPAIR_SERVICE

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

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro SRP_STOP_REPAIR_SERVICE
  !insertmacro SRP_DELETE_REPAIR_SERVICE
  !insertmacro SRP_CREATE_REPAIR_SERVICE
  !insertmacro SRP_START_REPAIR_SERVICE
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro SRP_STOP_REPAIR_SERVICE
  !insertmacro SRP_DELETE_REPAIR_SERVICE
!macroend
