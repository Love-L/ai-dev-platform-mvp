@echo off
chcp 65001 >nul
echo ========================================
echo AI研发协作平台 - 创建桌面快捷方式
echo ========================================
echo.

:: 获取桌面路径
set "DESKTOP=%USERPROFILE%\Desktop"

:: 获取当前目录
set "CURRENT_DIR=%~dp0"

:: 创建VBScript来创建快捷方式
echo Set WshShell = CreateObject("WScript.Shell") > "%TEMP%\create_shortcut.vbs"
echo Set Shortcut = WshShell.CreateShortcut("%DESKTOP%\AI研发协作平台.lnk") >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.TargetPath = "%CURRENT_DIR%start.bat" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.WorkingDirectory = "%CURRENT_DIR%" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.Description = "AI研发协作平台 - 单机版 v2.0" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.IconLocation = "%CURRENT_DIR%resources\icon.ico,0" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.WindowStyle = 1 >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.Save >> "%TEMP%\create_shortcut.vbs"

:: 执行VBScript
cscript //nologo "%TEMP%\create_shortcut.vbs"

:: 清理临时文件
del "%TEMP%\create_shortcut.vbs"

echo.
echo ========================================
echo 桌面快捷方式创建成功！
echo ========================================
echo.
echo 快捷方式位置: %DESKTOP%\AI研发协作平台.lnk
echo.
pause
