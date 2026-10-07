# Builds genji.dll and copies it to build\genji.dll, where the me3 profile loads it from.
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

Push-Location "$root\genji"
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Force "$root\build" | Out-Null
try {
    Copy-Item "$root\genji\target\release\genji.dll" "$root\build\genji.dll" -Force
    Copy-Item "$root\genji\target\release\genji.pdb" "$root\build\genji.pdb" -Force
} catch {
    throw "Could not copy genji.dll into build\. Is Elden Ring still running? Close it and build again."
}
$dll = Get-Item "$root\build\genji.dll"
Write-Host ("Built {0} ({1:N0} bytes, {2})" -f $dll.FullName, $dll.Length, $dll.LastWriteTime)
