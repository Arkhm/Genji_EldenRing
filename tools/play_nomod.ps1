# Backs up your saves and starts Elden Ring offline through me3 with the Genji save, WITHOUT genji.dll.
# Use it to check whether a problem comes from our mod or from Elden Ring itself.
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

& "$PSScriptRoot\backup_saves.ps1"

Write-Host "Starting Elden Ring through me3 WITHOUT the Genji mod (offline, save ER0000.genji.sl2)."
me3 launch --profile "$root\profile\genji-nomod.me3"
