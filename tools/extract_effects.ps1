# The effect reader (docs/DESIGN.md 5l, part 6): every effect Genji's entities use, with its parts, timings,
# materials and textures, into D:\GenjiBridgeAssets\fx (outside the repository).
#
# It adds our mode (tools\fx_reader\ExtractGenjiEffects.cs) to DataTool's own source, builds it with the .NET 10 SDK and
# runs it on the same offline copy of Overwatch as tools\extract_genji.ps1. Read-only on the Overwatch files; never
# goes online (--online=false).
#
# Needs, once (done 2026-10-13):
#   - the .NET 10 SDK in D:\GenjiBridgeAssets\tools\dotnet (Microsoft's dotnet-install.ps1, -InstallDir there);
#   - DataTool's source at the commit of our DataTool release (v2.24.1, df522f0c), with its submodules:
#       git clone https://github.com/overtools/OWLib.git D:\GenjiBridgeAssets\tools\OWLib-src
#       git -C D:\GenjiBridgeAssets\tools\OWLib-src checkout df522f0c
#       git -C D:\GenjiBridgeAssets\tools\OWLib-src submodule update --init --recursive
param(
    [string]$Overwatch = "D:\SteamDepotDownloads\app_2357570\Overwatch_2.24.1",
    [string]$Hero = "Genji",
    [string]$Assets = "D:\GenjiBridgeAssets"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$dotnet = Join-Path $Assets "tools\dotnet\dotnet.exe"
$src = Join-Path $Assets "tools\OWLib-src"
$built = Join-Path $Assets "tools\datatool-built"
$out = Join-Path $Assets "fx"
$logs = Join-Path $root "logs"
if (-not (Test-Path $dotnet)) { throw "No .NET SDK at $dotnet (see the top of this script)." }
if (-not (Test-Path "$src\DataTool\DataTool.csproj")) { throw "No DataTool source at $src (see the top of this script)." }

Copy-Item (Join-Path $PSScriptRoot "fx_reader\ExtractGenjiEffects.cs") "$src\DataTool\ToolLogic\Extract\ExtractGenjiEffects.cs" -Force
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:DOTNET_NOLOGO = "1"
Write-Host "Building DataTool with the effect reader ..."
& $dotnet build "$src\DataTool\DataTool.csproj" -c Release -o $built -v quiet -nologo "-p:WarningLevel=0" | Out-File (Join-Path $logs "fx-build.txt") -Encoding utf8
if ($LASTEXITCODE -ne 0) { Get-Content (Join-Path $logs "fx-build.txt") | Select-String "error" | Select-Object -First 20; throw "build failed (logs\fx-build.txt)" }

if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force $out, $logs | Out-Null
Write-Host "Reading $Hero's effects into $out ..."
$env:GENJI_FX_HERO = $Hero
# Our .NET is a local copy, not installed system-wide: tell the built DataTool where it is.
$env:DOTNET_ROOT = Join-Path $Assets "tools\dotnet"
$p = Start-Process -FilePath (Join-Path $built "DataTool.exe") -ArgumentList ('"{0}" extract-genji-effects "{1}" --online=false' -f $Overwatch, $out) `
    -NoNewWindow -Wait -PassThru -RedirectStandardOutput (Join-Path $logs "fx-run.txt") -RedirectStandardError (Join-Path $logs "fx-run.err.txt")
Get-Content (Join-Path $logs "fx-run.txt") | Select-String "GenjiFX" | ForEach-Object { Write-Host $_ }
if ($p.ExitCode -ne 0) { throw "DataTool exited with $($p.ExitCode) (logs\fx-run.txt, logs\fx-run.err.txt)" }
