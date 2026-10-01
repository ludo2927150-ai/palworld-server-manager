<# Crée un raccourci « Palworld Manager » sur le Bureau qui lance Lancer-Palworld-Manager.bat.
   Usage : powershell -ExecutionPolicy Bypass -File .\scripts\creer-raccourci.ps1 #>
$root = Split-Path -Parent $PSScriptRoot
$desktop = [Environment]::GetFolderPath("Desktop")
$lnk = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $desktop "Palworld Manager.lnk"))
$lnk.TargetPath = Join-Path $root "Lancer-Palworld-Manager.bat"
$lnk.WorkingDirectory = $root
$lnk.IconLocation = (Join-Path $root "src-tauri\icons\icon.ico")
$lnk.Save()
Write-Host "Raccourci créé sur le Bureau : Palworld Manager" -ForegroundColor Green
