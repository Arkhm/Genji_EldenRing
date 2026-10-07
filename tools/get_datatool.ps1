# Downloads the latest DataTool release (overtools/OWLib, official GitHub releases) into third_party\datatool\.
# DataTool must match the installed Overwatch build; run this again after an Overwatch update once a new
# DataTool release is out. Needs the GitHub CLI (gh).
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$third = Join-Path $root "third_party"
$zip = Join-Path $third "toolchain-release.zip"
$dest = Join-Path $third "datatool"

$release = gh api repos/overtools/OWLib/releases/latest | ConvertFrom-Json
Write-Host "Latest DataTool: $($release.tag_name), published $($release.published_at)"
Write-Host (($release.body -split "`n" | Select-String "Supports Overwatch") -join "`n")

New-Item -ItemType Directory -Force $third | Out-Null
gh release download $release.tag_name --repo overtools/OWLib --pattern toolchain-release.zip --dir $third --clobber
if ($LASTEXITCODE -ne 0) { throw "download failed" }
Write-Host ("SHA-256: " + (Get-FileHash $zip -Algorithm SHA256).Hash)

if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
Expand-Archive $zip -DestinationPath $dest
Write-Host "Unpacked to $dest"
