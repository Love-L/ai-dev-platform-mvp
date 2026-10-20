; ============================================================
; ACE 自定义 NSIS 安装钩子（tauri.conf.json > bundle.windows.nsis.installerHooks）
; 职责：把 $INSTDIR\tools（内含 wecom-cli.exe 等便携工具）注册到
;       当前用户 PATH（HKCU\Environment），卸载时移除。
; 约束：
;   - 仅用 NSIS 内置命令 + System 插件，不 !include 第三方 .nsh，
;     避免与模板内置库重复定义。
;   - 卸载段只能 Call 以 "un." 开头的函数，故用宏成对生成
;     install/uninstall 两套函数（标签加前缀保证唯一）。
;   - 含中文注释，文件必须保存为 UTF-8 with BOM。
; ============================================================

; ---- 全局临时变量（顶层声明，安装/卸载共用）----
Var AcePath     ; 读出的当前用户 Path
Var AceDir      ; 待注册目录
Var AceFound    ; 查找结果 0/1
Var AceRest     ; 令牌解析：剩余字符串
Var AceIdx      ; 令牌解析：扫描下标
Var AceCh       ; 令牌解析：当前字符
Var AceToken    ; 令牌解析：当前令牌
Var AceOut      ; 重建后的 Path
Var AceSkip     ; 令牌解析：消费长度

; ------------------------------------------------------------
; 广播 WM_SETTINGCHANGE("Environment")，让新启动的进程感知 PATH 变更
; ------------------------------------------------------------
!macro AceBroadcastEnvChange
  System::Call 'user32::SendMessageTimeout(i 0xffff, i 0x1A, i 0, w "Environment", i 2, i 5000, i 0)'
!macroend

; ------------------------------------------------------------
; 成对函数体：
;   ${PFX} = ""    → 安装侧函数（AcePathRead/AcePathHasDir/...）
;   ${PFX} = "un." → 卸载侧函数（un.AcePathRead/...）
;   ${LBL} = 标签前缀，保证两侧标签唯一
; ------------------------------------------------------------
!macro AcePathFunctions PFX LBL
Function ${PFX}AcePathRead
  ReadRegStr $AcePath HKCU "Environment" "Path"
  ClearErrors
FunctionEnd

Function ${PFX}AcePathHasDir
  StrCpy $AceFound 0
  StrCpy $AceRest "$AcePath;"
${LBL}has_loop:
  StrCmp $AceRest "" ${LBL}has_done
  StrCpy $AceIdx 0
${LBL}has_scan:
  StrCpy $AceCh $AceRest 1 $AceIdx
  StrCmp $AceCh "" ${LBL}has_done
  StrCmp $AceCh ";" ${LBL}has_sep
  IntOp $AceIdx $AceIdx + 1
  Goto ${LBL}has_scan
${LBL}has_sep:
  StrCpy $AceToken $AceRest $AceIdx
  StrCmp $AceToken $AceDir ${LBL}has_found
  IntOp $AceSkip $AceIdx + 1
  StrCpy $AceRest $AceRest "" $AceSkip
  Goto ${LBL}has_loop
${LBL}has_found:
  StrCpy $AceFound 1
${LBL}has_done:
FunctionEnd

Function ${PFX}AcePathAdd
  Call ${PFX}AcePathRead
  Call ${PFX}AcePathHasDir
  StrCmp $AceFound 1 ${LBL}add_done
  StrCmp $AcePath "" ${LBL}add_empty
  StrCpy $AcePath "$AcePath;$AceDir"
  Goto ${LBL}add_write
${LBL}add_empty:
  StrCpy $AcePath "$AceDir"
${LBL}add_write:
  WriteRegExpandStr HKCU "Environment" "Path" $AcePath
  !insertmacro AceBroadcastEnvChange
${LBL}add_done:
FunctionEnd

Function ${PFX}AcePathRemove
  Call ${PFX}AcePathRead
  StrCmp $AcePath "" ${LBL}rm_done
  StrCpy $AceOut ""
  StrCpy $AceRest "$AcePath;"
${LBL}rm_loop:
  StrCmp $AceRest "" ${LBL}rm_write
  StrCpy $AceIdx 0
${LBL}rm_scan:
  StrCpy $AceCh $AceRest 1 $AceIdx
  StrCmp $AceCh "" ${LBL}rm_last
  StrCmp $AceCh ";" ${LBL}rm_sep
  IntOp $AceIdx $AceIdx + 1
  Goto ${LBL}rm_scan
${LBL}rm_sep:
  StrCpy $AceToken $AceRest $AceIdx
  StrCmp $AceToken "" ${LBL}rm_consume
  StrCmp $AceToken $AceDir ${LBL}rm_consume
  StrCmp $AceOut "" ${LBL}rm_first
  StrCpy $AceOut "$AceOut;"
${LBL}rm_first:
  StrCpy $AceOut "$AceOut$AceToken"
${LBL}rm_consume:
  IntOp $AceSkip $AceIdx + 1
  StrCpy $AceRest $AceRest "" $AceSkip
  Goto ${LBL}rm_loop
${LBL}rm_last:
  StrCmp $AceRest "" ${LBL}rm_write
  StrCmp $AceRest $AceDir ${LBL}rm_write
  StrCmp $AceOut "" ${LBL}rm_last_first
  StrCpy $AceOut "$AceOut;"
${LBL}rm_last_first:
  StrCpy $AceOut "$AceOut$AceRest"
${LBL}rm_write:
  StrCmp $AceOut "" ${LBL}rm_delete
  WriteRegExpandStr HKCU "Environment" "Path" $AceOut
  Goto ${LBL}rm_broadcast
${LBL}rm_delete:
  DeleteRegValue HKCU "Environment" "Path"
${LBL}rm_broadcast:
  !insertmacro AceBroadcastEnvChange
${LBL}rm_done:
FunctionEnd
!macroend

!insertmacro AcePathFunctions "" "i_"
!insertmacro AcePathFunctions "un." "u_"

; ------------------------------------------------------------
; 四个钩子宏（Tauri 模板会全部 !insertmacro，必须全部定义）
; ------------------------------------------------------------
!macro NSIS_HOOK_PREINSTALL
  ; 安装前置逻辑：无需操作
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "[ACE] Register tools dir in user PATH: $INSTDIR\tools"
  StrCpy $AceDir "$INSTDIR\tools"
  Call AcePathAdd
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "[ACE] Remove tools dir from user PATH: $INSTDIR\tools"
  StrCpy $AceDir "$INSTDIR\tools"
  Call un.AcePathRemove
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; 卸载后清理：无残留需处理
!macroend
