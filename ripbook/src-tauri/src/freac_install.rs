#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use serde::Deserialize;
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;

use crate::models::FreacInstallResult;
#[cfg(windows)]
use crate::powershell;
#[cfg(windows)]
use crate::settings;

#[cfg(windows)]
const INSTALL_SCRIPT: &str = r##"
param(
    [string]$Version = "",
    [Parameter(Mandatory = $true)][string]$Destination,
    [string]$Architecture = "x64",
    [string]$UserAgent = "RipBook/0.1.0"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$headers = @{ "User-Agent" = $UserAgent }
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

if ([string]::IsNullOrWhiteSpace($Version) -or $Version.Trim().ToLowerInvariant() -eq "latest") {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/enzo1982/freac/releases/latest" -Headers $headers -TimeoutSec 30
} else {
    $tag = $Version.Trim()
    if (-not $tag.StartsWith("v")) { $tag = "v$tag" }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/enzo1982/freac/releases/tags/$tag" -Headers $headers -TimeoutSec 30
}

$cleanVersion = "$($release.tag_name)".TrimStart([char]'v')
$assetName = "freac-$cleanVersion-windows-$Architecture.zip"
$asset = @($release.assets) | Where-Object { $_.name -eq $assetName } | Select-Object -First 1

if (-not $asset) {
    $available = (@($release.assets) | Where-Object { $_.name -like "*-windows-*.zip" } | ForEach-Object { $_.name }) -join ", "
    throw "fre:ac $cleanVersion does not offer $assetName. Available Windows downloads: $available"
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("ripbook-freac-" + [Guid]::NewGuid().ToString("N"))
$zipPath = Join-Path $tempRoot "freac.zip"
$extractPath = Join-Path $tempRoot "extract"

New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
New-Item -ItemType Directory -Force -Path $Destination | Out-Null

try {
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath -Headers $headers -TimeoutSec 300
    $hash = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLowerInvariant()

    if ($cleanVersion -eq "1.1.7" -and $hash -ne "ef45665aae6c1c0eb4c0ecd8ecc6bed24f02f3cddf6cfd72d8e5c9bc858bf110") {
        throw "The fre:ac archive checksum did not match the expected value."
    }

    Expand-Archive -Path $zipPath -DestinationPath $extractPath -Force

    $exe = Get-ChildItem -Path $extractPath -Recurse -Filter "freaccmd.exe" | Select-Object -First 1
    if (-not $exe) { throw "The fre:ac archive did not contain freaccmd.exe." }

    $target = Join-Path $Destination $cleanVersion
    if (Test-Path $target) { Remove-Item -Recurse -Force $target }
    New-Item -ItemType Directory -Force -Path $target | Out-Null

    Get-ChildItem -Path $exe.Directory.FullName -Force | Copy-Item -Destination $target -Recurse -Force

    $finalPath = Join-Path $target "freaccmd.exe"
    if (-not (Test-Path $finalPath)) { throw "fre:ac was extracted, but freaccmd.exe was not installed." }

    [pscustomobject]@{
        version = $cleanVersion
        path = $finalPath
        message = "Installed fre:ac $cleanVersion from the official release (SHA256 $hash)."
    } | ConvertTo-Json -Compress
} finally {
    if (Test-Path $tempRoot) { Remove-Item -Recurse -Force $tempRoot -ErrorAction SilentlyContinue }
}
"##;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(windows)]
struct InstallPayload {
    version: String,
    path: String,
    #[serde(default)]
    message: String,
}

pub async fn install(
    app: &AppHandle,
    version: Option<String>,
) -> Result<FreacInstallResult, String> {
    #[cfg(not(windows))]
    {
        let _ = (app, version);
        Err("Downloading fre:ac is only supported on Windows.".to_string())
    }

    #[cfg(windows)]
    {
        let base = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Could not locate the app data folder: {error}"))?
            .join("freac");

        let architecture = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            "x86" => "i686",
            other => return Err(format!("Unsupported Windows architecture: {other}.")),
        };

        let requested = version.unwrap_or_default();
        let base_string = base.to_string_lossy().to_string();
        let output = powershell::run_script(
            INSTALL_SCRIPT,
            &[
                ("Version", requested.trim()),
                ("Destination", &base_string),
                ("Architecture", architecture),
                (
                    "UserAgent",
                    "RipBook/0.1.0 (https://github.com/vcase/ripcds)",
                ),
            ],
        )
        .await?;

        let payload: InstallPayload = serde_json::from_str(&output)
            .map_err(|error| format!("Could not read the fre:ac installer result: {error}"))?;

        if !Path::new(&payload.path).is_file() {
            return Err(format!(
                "fre:ac was downloaded, but freaccmd.exe was not found at {}.",
                payload.path
            ));
        }

        let mut app_settings = settings::load(app);
        app_settings.freaccmd_path = Some(payload.path.clone());
        settings::save(app, &app_settings)?;

        Ok(FreacInstallResult {
            version: payload.version,
            path: payload.path,
            message: if payload.message.is_empty() {
                "fre:ac was installed.".to_string()
            } else {
                payload.message
            },
        })
    }
}
