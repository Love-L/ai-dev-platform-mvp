$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:USERPROFILE\Desktop\ACE.lnk")
$Shortcut.TargetPath = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\src-tauri\target\release\ai-dev-platform-mvp.exe"
$Shortcut.WorkingDirectory = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp"
$Shortcut.Description = "ACE - AI Collaboration Engine"
$Shortcut.IconLocation = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\src-tauri\icons\icon.ico"
$Shortcut.Save()
Write-Host "Desktop shortcut created successfully!"
