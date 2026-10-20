@echo off
chcp 65001 >nul
echo ========================================
echo AI研发协作平台 - 安装程序
echo ========================================
echo.

:: 获取当前目录
set "CURRENT_DIR=%~dp0"

:: 获取桌面路径
set "DESKTOP=%USERPROFILE%\Desktop"

:: 获取开始菜单路径
set "START_MENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs"

echo [1/3] 创建桌面快捷方式...

:: 创建桌面快捷方式
echo Set WshShell = CreateObject("WScript.Shell") > "%TEMP%\create_shortcut.vbs"
echo Set Shortcut = WshShell.CreateShortcut("%DESKTOP%\AI研发协作平台.lnk") >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.TargetPath = "%CURRENT_DIR%start.bat" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.WorkingDirectory = "%CURRENT_DIR%" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.Description = "AI研发协作平台 - 单机版 v2.0" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.IconLocation = "%CURRENT_DIR%resources\icon.ico,0" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.WindowStyle = 1 >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.Save >> "%TEMP%\create_shortcut.vbs"

cscript //nologo "%TEMP%\create_shortcut.vbs"
del "%TEMP%\create_shortcut.vbs"

echo       桌面快捷方式创建完成！

echo.
echo [2/3] 创建开始菜单快捷方式...

:: 创建开始菜单文件夹
if not exist "%START_MENU%\AI研发协作平台" (
    mkdir "%START_MENU%\AI研发协作平台"
)

:: 创建开始菜单快捷方式
echo Set WshShell = CreateObject("WScript.Shell") > "%TEMP%\create_shortcut.vbs"
echo Set Shortcut = WshShell.CreateShortcut("%START_MENU%\AI研发协作平台\AI研发协作平台.lnk") >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.TargetPath = "%CURRENT_DIR%start.bat" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.WorkingDirectory = "%CURRENT_DIR%" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.Description = "AI研发协作平台 - 单机版 v2.0" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.IconLocation = "%CURRENT_DIR%resources\icon.ico,0" >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.WindowStyle = 1 >> "%TEMP%\create_shortcut.vbs"
echo Shortcut.Save >> "%TEMP%\create_shortcut.vbs"

cscript //nologo "%TEMP%\create_shortcut.vbs"
del "%TEMP%\create_shortcut.vbs"

echo       开始菜单快捷方式创建完成！

echo.
echo [3/3] 创建卸载程序...

:: 创建卸载脚本
echo @echo off > "%CURRENT_DIR%uninstall.bat"
echo chcp 65001 ^>nul >> "%CURRENT_DIR%uninstall.bat"
echo echo ======================================== >> "%CURRENT_DIR%uninstall.bat"
echo echo AI研发协作平台 - 卸载程序 >> "%CURRENT_DIR%uninstall.bat"
echo echo ======================================== >> "%CURRENT_DIR%uninstall.bat"
echo echo. >> "%CURRENT_DIR%uninstall.bat"
echo echo 正在删除快捷方式... >> "%CURRENT_DIR%uninstall.bat"
echo del "%%USERPROFILE%%\Desktop\AI研发协作平台.lnk" 2^>nul >> "%CURRENT_DIR%uninstall.bat"
echo rmdir /s /q "%%APPDATA%%\Microsoft\Windows\Start Menu\Programs\AI研发协作平台" 2^>nul >> "%CURRENT_DIR%uninstall.bat"
echo echo. >> "%CURRENT_DIR%uninstall.bat"
echo echo 卸载完成！ >> "%CURRENT_DIR%uninstall.bat"
echo echo. >> "%CURRENT_DIR%uninstall.bat"
echo pause >> "%CURRENT_DIR%uninstall.bat"

echo       卸载程序创建完成！

echo.
echo ========================================
echo 安装完成！
echo ========================================
echo.
echo 已创建以下快捷方式:
echo   - 桌面: %DESKTOP%\AI研发协作平台.lnk
echo   - 开始菜单: %START_MENU%\AI研发协作平台\AI研发协作平台.lnk
echo.
echo 如需卸载，请运行 uninstall.bat
echo.
pause
