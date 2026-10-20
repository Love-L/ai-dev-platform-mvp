# PowerShell script to create desktop shortcut
$WshShell = New-Object -ComObject WScript.Shell
$DesktopPath = [System.Environment]::GetFolderPath('Desktop')
$ShortcutPath = Join-Path $DesktopPath "AI研发协作平台.lnk"
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\start.bat"
$Shortcut.WorkingDirectory = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp"
$Shortcut.Description = "AI研发协作平台 - 单机版 v2.0"
$Shortcut.Save()
Write-Host "Desktop shortcut created successfully!"
Write-Host "Location: $ShortcutPath"
