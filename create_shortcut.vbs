Set WshShell = WScript.CreateObject("WScript.Shell")
Set shortcut = WshShell.CreateShortcut(WshShell.SpecialFolders("Desktop") & "\ACE.lnk")
shortcut.TargetPath = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\src-tauri\target\release\ai-dev-platform-mvp.exe"
shortcut.WorkingDirectory = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp"
shortcut.Description = "ACE - AI Collaboration Engine"
shortcut.IconLocation = "E:\orca\AI自动化开发流程\单机版\ai-dev-platform-mvp\src-tauri\target\release\ai-dev-platform-mvp.exe"
shortcut.Save
WScript.Echo "Desktop shortcut created successfully!"
