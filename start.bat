@echo off
chcp 65001 >nul
title AI研发协作平台

:: 切换到脚本所在目录
cd /d "%~dp0"

:: 启动应用
echo ========================================
echo AI研发协作平台 - 启动中...
echo ========================================
echo.

:: 检查可执行文件是否存在
if exist "ai-dev-platform.exe" (
    echo 正在启动应用...
    start "" "ai-dev-platform.exe"
) else (
    echo [错误] 未找到应用程序文件！
    echo 请确保 ai-dev-platform.exe 存在于当前目录。
    echo.
    pause
    exit /b 1
)
