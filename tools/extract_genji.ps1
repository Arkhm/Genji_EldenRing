# Step 2: extract Genji's model, animations, ability effects and voice lines from YOUR Overwatch install into
# owassets\raw\ (gitignored, never committed). Read-only on the Overwatch folder. DataTool runs with
# --online=false so it never downloads anything from Blizzard. Overwatch does not need to be running.
#
# Every DataTool output goes to logs\datatool-*.txt; a file count summary goes to logs\extract-summary.txt.
param(
    [string]$Overwatch = "D:\SteamLibrary\steamapps\common\Overwatch",
    [string]$Skin = "Overwatch 2"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$dt = Join-Path $root "third_party\datatool\DataTool.exe"
$out = Join-Path $root "owassets\raw"
$logs = Join-Path $root "logs"
if (-not (Test-Path $dt)) { throw "DataTool not found. Run tools\get_datatool.ps1 first." }
New-Item -ItemType Directory -Force $out, $logs | Out-Null

function Invoke-DataTool([string]$name, [string[]]$arguments) {
    $log = Join-Path $logs "datatool-$name.txt"
    $errLog = Join-Path $logs "datatool-$name.err.txt"
    Write-Host "DataTool $name ..."
    $sw = [Diagnostics.Stopwatch]::StartNew()
    # A separate process, so DataTool writing to stderr doesn't count as a PowerShell error.
    $quoted = (@($Overwatch) + $arguments + @("--online=false") | ForEach-Object { '"' + $_ + '"' }) -join " "
    $p = Start-Process -FilePath $dt -ArgumentList $quoted -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError $errLog
    $code = $p.ExitCode
    Write-Host ("  exit {0}, {1:N0}s, log {2}" -f $code, $sw.Elapsed.TotalSeconds, $log)
    if (Select-String -Path $log, $errLog -Pattern "does not support this version of Overwatch" -Quiet) {
        throw "This DataTool doesn't support the installed Overwatch build yet. Wait for a new release, then run tools\get_datatool.ps1."
    }
    return $code
}

Invoke-DataTool "list-heroes" @("list-heroes") | Out-Null
Invoke-DataTool "list-unlocks" @("list-unlocks") | Out-Null
Invoke-DataTool "list-abilities" @("list-abilities") | Out-Null
Invoke-DataTool "extract-unlocks" @("extract-unlocks", $out, "Genji|skin=$Skin") | Out-Null
Invoke-DataTool "extract-abilities" @("extract-abilities", $out, "Genji") | Out-Null
Invoke-DataTool "extract-hero-voice" @("extract-hero-voice", $out, "Genji") | Out-Null

# Summary: what came out, by folder and file type.
$summary = Join-Path $logs "extract-summary.txt"
$files = Get-ChildItem $out -Recurse -File
$lines = @()
$lines += "Extracted {0} files, {1:N1} MB, into {2}" -f $files.Count, (($files | Measure-Object Length -Sum).Sum / 1MB), $out
$lines += ""
$lines += "By type:"
$lines += $files | Group-Object Extension | Sort-Object Count -Descending | ForEach-Object { "  {0,-14} {1,6}" -f $_.Name, $_.Count }
$lines += ""
$lines += "By folder (two levels):"
$lines += $files | Group-Object { ($_.DirectoryName.Substring($out.Length).TrimStart('\') -split '\\' | Select-Object -First 2) -join '\' } |
    Sort-Object Name | ForEach-Object { "  {0,-60} {1,6}" -f $_.Name, $_.Count }
$lines += ""
$lines += "Names containing 'first' or '1p' (first-person candidates):"
$lines += $files | Where-Object { $_.FullName -match "first|\\1p|_1p|1st" } | Select-Object -First 50 | ForEach-Object { "  " + $_.FullName.Substring($out.Length) }
$lines | Set-Content $summary -Encoding utf8
$lines | Select-Object -First 40 | ForEach-Object { Write-Host $_ }
Write-Host "Full summary: $summary"
