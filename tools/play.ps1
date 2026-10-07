# Backs up your saves, builds genji.dll, and starts Elden Ring offline through me3 with the Genji profile.
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

& "$PSScriptRoot\backup_saves.ps1"
& "$PSScriptRoot\build.ps1"

Write-Host "Starting Elden Ring through me3 (offline, save ER0000.genji.sl2). The log is logs\genji.log."
me3 launch --profile "$root\profile\genji.me3"
