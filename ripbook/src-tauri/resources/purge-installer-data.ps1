$ErrorActionPreference = "SilentlyContinue"

$settingsFiles = @(
    (Join-Path (Join-Path $env:APPDATA "com.ripbook.desktop") "settings.json"),
    (Join-Path (Join-Path $env:LOCALAPPDATA "com.ripbook.desktop") "settings.json")
)

$roots = New-Object System.Collections.Generic.List[string]

foreach ($settingsFile in $settingsFiles) {
    if (-not (Test-Path -LiteralPath $settingsFile)) { continue }

    try {
        $settings = Get-Content -LiteralPath $settingsFile -Raw | ConvertFrom-Json

        if ($settings.audioRoot) {
            $roots.Add([string]$settings.audioRoot)
        }

        foreach ($root in @($settings.audioRoots)) {
            if ($root) { $roots.Add([string]$root) }
        }
    } catch {
    }
}

$roots.Add((Join-Path $env:USERPROFILE "AudioBooks"))

$uniqueRoots = @($roots | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Sort-Object -Unique)

foreach ($root in $uniqueRoots) {
    Write-Output "Removing RipBook state under $root"

    Get-ChildItem -LiteralPath $root -Recurse -Force -File -Filter ".ripbook.json" -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue }

    Get-ChildItem -LiteralPath $root -Recurse -Force -File -Filter "rip_cd_*.log" -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue }

    Get-ChildItem -LiteralPath $root -Recurse -Force -Directory -Filter "wip_cd_*" -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue }
}
