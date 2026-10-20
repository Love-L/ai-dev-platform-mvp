@echo off
chcp 65001 >nul
echo ========================================
echo AI研发协作平台 - 卸载程序
echo ========================================
echo.

:: 获取开始菜单路径
set "START_MENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs"

echo [1/2] 删除桌面快捷方式...
del "%USERPROFILE%\Desktop\AI研发协作平台.lnk" 2>nul
if exist "%USERPROFILE%\Desktop\AI研发协作平台.lnk" (
    echo       [警告] 无法删除桌面快捷方式，请手动删除
) else (
    echo       桌面快捷方式已删除
)

echo.
echo [2/2] 删除开始菜单快捷方式...
rmdir /s /q "%START_MENU%\AI研发协作平台" 2>nul
if exist "%START_MENU%\AI研发协作平台" (
    echo       [警告] 无法删除开始菜单快捷方式，请手动删除
) else (
    echo       开始菜单快捷方式已删除
)

echo.
echo ========================================
echo 卸载完成！
echo ========================================
echo.
echo 提示: 应用程序文件未被删除。
echo 如需完全卸载，请手动删除应用程序文件夹。
echo.
pause
