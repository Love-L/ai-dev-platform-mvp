@echo off
chcp 65001 >nul

REM 获取exe的短路径
set "EXE_PATH=E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\src-tauri\target\release\ai-dev-platform-mvp.exe"
set "ICON_PATH=E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\src-tauri\icons\icon.ico"

REM 创建桌面快捷方式（使用PowerShell避免中文路径问题）
powershell -Command ^
  "$WshShell = New-Object -ComObject WScript.Shell; ^
   $Shortcut = $WshShell.CreateShortcut([System.IO.Path]::Combine([System.Environment]::GetFolderPath('Desktop'), 'ACE.lnk')); ^
   $Shortcut.TargetPath = '%EXE_PATH%'; ^
   $Shortcut.WorkingDirectory = '%~dp0'; ^
   $Shortcut.Description = 'ACE - AI Collaboration Engine'; ^
   $Shortcut.IconLocation = '%ICON_PATH%'; ^
   $Shortcut.Save()"

echo Desktop shortcut created successfully!
