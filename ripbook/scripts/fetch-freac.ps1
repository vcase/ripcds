param(
    [string]$Version = "1.1.7",
    [string]$Destination = "src-tauri/resources/freac"
)

$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "This script prepares the Windows fre:ac payload. Run it on Windows (or in the Windows CI job)."
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$destinationPath = Join-Path $repoRoot $Destination
$tempBase = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$tempRoot = Join-Path $tempBase "ripbook-freac"
$zipPath = Join-Path $tempRoot "freac.zip"
$extractPath = Join-Path $tempRoot "extract"

$url = "https://github.com/enzo1982/freac/releases/download/v$Version/freac-$Version-windows-x64.zip"
$expectedHash = "ef45665aae6c1c0eb4c0ecd8ecc6bed24f02f3cddf6cfd72d8e5c9bc858bf110"

Write-Host "Downloading fre:ac $Version from $url"
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
New-Item -ItemType Directory -Force -Path $destinationPath | Out-Null

Invoke-WebRequest -Uri $url -OutFile $zipPath
$downloadHash = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLowerInvariant()
if ($downloadHash -ne $expectedHash) {
    throw "fre:ac archive checksum mismatch. Expected $expectedHash, got $downloadHash."
}

if (Test-Path $extractPath) {
    Remove-Item -Recurse -Force $extractPath
}
Expand-Archive -Path $zipPath -DestinationPath $extractPath -Force

$freacFolder = Get-ChildItem -Path $extractPath -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "freaccmd.exe") } |
    Select-Object -First 1

if (-not $freacFolder) {
    throw "The fre:ac archive did not contain freaccmd.exe."
}

Get-ChildItem -Path $destinationPath -Force |
    Where-Object { $_.Name -ne "README.txt" } |
    Remove-Item -Recurse -Force

Get-ChildItem -Path $freacFolder.FullName -Force | Copy-Item -Destination $destinationPath -Recurse -Force

$hash = (Get-FileHash -Algorithm SHA256 (Join-Path $destinationPath "freaccmd.exe")).Hash
Write-Host "Bundled freaccmd.exe SHA256: $hash"
Write-Host "fre:ac files copied to $destinationPath"
