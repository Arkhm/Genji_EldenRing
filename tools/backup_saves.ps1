# Copies every Elden Ring save (and its .bak) into backups\saves\<date-time>\. Read-only on the save folder.
# Restore: close the game, then copy the files from a backup folder back into %APPDATA%\EldenRing\<steam id>\.
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$src = Join-Path $env:APPDATA "EldenRing"
if (-not (Test-Path $src)) { throw "No save folder at $src" }

$dst = Join-Path $root ("backups\saves\" + (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Force $dst | Out-Null
Copy-Item "$src\*" $dst -Recurse

$files = Get-ChildItem $dst -Recurse -File
Write-Host ("Backed up {0} files ({1:N1} MB) to {2}" -f $files.Count, (($files | Measure-Object Length -Sum).Sum / 1MB), $dst)
$files | ForEach-Object { Write-Host ("  {0}  {1:N0} bytes" -f $_.FullName.Substring($dst.Length + 1), $_.Length) }
