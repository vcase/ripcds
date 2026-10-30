use serde::Deserialize;

use crate::models::DiscInfo;
#[cfg(windows)]
use crate::powershell;

#[cfg(windows)]
const USER_AGENT: &str = "RipBook/0.1.0 (https://github.com/vcase/ripcds)";

#[cfg(windows)]
const DISC_SCRIPT: &str = r##"
param(
    [Parameter(Mandatory = $true)][string]$Drive,
    [string]$Lookup = "false",
    [string]$Action = "info",
    [string]$UserAgent = "RipBook/0.1.0"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

if (-not $Drive.EndsWith(":")) { $Drive = "${Drive}:" }
$devicePath = "\\.\$Drive"

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public static class RipBookDisc
{
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr CreateFileW(string lpFileName, uint dwDesiredAccess, uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition, uint dwFlagsAndAttributes, IntPtr hTemplateFile);

    [DllImport("kernel32.dll", EntryPoint = "DeviceIoControl", SetLastError = true)]
    private static extern bool DeviceIoControlRead(IntPtr hDevice, uint dwIoControlCode, IntPtr lpInBuffer, uint nInBufferSize, byte[] lpOutBuffer, uint nOutBufferSize, out uint lpBytesReturned, IntPtr lpOverlapped);

    [DllImport("kernel32.dll", EntryPoint = "DeviceIoControl", SetLastError = true)]
    private static extern bool DeviceIoControlWrite(IntPtr hDevice, uint dwIoControlCode, byte[] lpInBuffer, uint nInBufferSize, IntPtr lpOutBuffer, uint nOutBufferSize, out uint lpBytesReturned, IntPtr lpOverlapped);

    [DllImport("kernel32.dll", EntryPoint = "DeviceIoControl", SetLastError = true)]
    private static extern bool DeviceIoControlInOut(IntPtr hDevice, uint dwIoControlCode, byte[] lpInBuffer, uint nInBufferSize, byte[] lpOutBuffer, uint nOutBufferSize, out uint lpBytesReturned, IntPtr lpOverlapped);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr hObject);

    private const uint GENERIC_READ = 0x80000000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint OPEN_EXISTING = 3;
    private const uint IOCTL_CDROM_READ_TOC = 0x00024000;
    private const uint IOCTL_CDROM_READ_TOC_EX = 0x00024054;
    private const uint IOCTL_STORAGE_MEDIA_REMOVAL = 0x002D4804;
    private const uint IOCTL_STORAGE_EJECT_MEDIA = 0x002D4808;

    private static IntPtr OpenDevice(string devicePath)
    {
        IntPtr handle = CreateFileW(devicePath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, 0, IntPtr.Zero);
        if (handle == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());

        return handle;
    }

    public static byte[] ReadToc(string devicePath)
    {
        IntPtr handle = OpenDevice(devicePath);

        try
        {
            byte[] buffer = new byte[804];
            uint returned;
            if (!DeviceIoControlRead(handle, IOCTL_CDROM_READ_TOC, IntPtr.Zero, 0, buffer, (uint)buffer.Length, out returned, IntPtr.Zero))
                throw new Win32Exception(Marshal.GetLastWin32Error());

            return buffer;
        }
        finally
        {
            CloseHandle(handle);
        }
    }

    public static string[] ReadCdText(string devicePath)
    {
        IntPtr handle = OpenDevice(devicePath);

        try
        {
            byte[] input = new byte[] { 0x05, 0x00, 0x00, 0x00 };
            byte[] buffer = new byte[4 + 18 * 256];
            uint returned;

            if (!DeviceIoControlInOut(handle, IOCTL_CDROM_READ_TOC_EX, input, (uint)input.Length, buffer, (uint)buffer.Length, out returned, IntPtr.Zero))
                throw new Win32Exception(Marshal.GetLastWin32Error());

            return ParseCdText(buffer, Math.Min(buffer.Length, (int)returned));
        }
        finally
        {
            CloseHandle(handle);
        }
    }

    private static string[] ParseCdText(byte[] buffer, int size)
    {
        List<KeyValuePair<int, int>> titlePacks = new List<KeyValuePair<int, int>>();
        List<KeyValuePair<int, int>> performerPacks = new List<KeyValuePair<int, int>>();

        for (int offset = 4; offset + 18 <= size; offset += 18)
        {
            int packType = buffer[offset];
            int trackNumber = buffer[offset + 1] & 0x7F;
            int sequenceNumber = buffer[offset + 2];
            int blockNumber = (buffer[offset + 3] >> 4) & 0x07;

            if (blockNumber != 0 || trackNumber != 0) continue;

            if (packType == 0x80) titlePacks.Add(new KeyValuePair<int, int>(sequenceNumber, offset));
            else if (packType == 0x81) performerPacks.Add(new KeyValuePair<int, int>(sequenceNumber, offset));
        }

        titlePacks.Sort((left, right) => left.Key.CompareTo(right.Key));
        performerPacks.Sort((left, right) => left.Key.CompareTo(right.Key));

        return new string[]
        {
            DecodeCdText(buffer, titlePacks),
            DecodeCdText(buffer, performerPacks)
        };
    }

    private static string DecodeCdText(byte[] buffer, List<KeyValuePair<int, int>> packs)
    {
        List<byte> bytes = new List<byte>();

        foreach (KeyValuePair<int, int> pack in packs)
        {
            for (int i = 4; i < 16; i++) bytes.Add(buffer[pack.Value + i]);
        }

        int end = bytes.IndexOf(0);
        if (end < 0) end = bytes.Count;

        return Encoding.GetEncoding(28591).GetString(bytes.ToArray(), 0, end).Trim();
    }

    public static void Eject(string devicePath)
    {
        IntPtr handle = OpenDevice(devicePath);

        try
        {
            byte[] allowRemoval = new byte[] { 0 };
            uint returned;

            DeviceIoControlWrite(handle, IOCTL_STORAGE_MEDIA_REMOVAL, allowRemoval, (uint)allowRemoval.Length, IntPtr.Zero, 0, out returned, IntPtr.Zero);

            if (!DeviceIoControlWrite(handle, IOCTL_STORAGE_EJECT_MEDIA, null, 0, IntPtr.Zero, 0, out returned, IntPtr.Zero))
                throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        finally
        {
            CloseHandle(handle);
        }
    }
}
"@

if ($Action -eq "eject") {
    $primaryError = $null

    try {
        [RipBookDisc]::Eject($devicePath)
        [pscustomobject]@{ message = "Ejected $Drive." } | ConvertTo-Json -Compress
        return
    } catch {
        $primaryError = $_.Exception.Message
    }

    try {
        $root = $Drive + "\"
        (New-Object -ComObject Shell.Application).Namespace(17).ParseName($root).InvokeVerb("Eject")
        [pscustomobject]@{ message = "Ejected $Drive." } | ConvertTo-Json -Compress
        return
    } catch {
        throw "Could not eject ${Drive}. $primaryError $($_.Exception.Message)"
    }
}

$buffer = [RipBookDisc]::ReadToc($devicePath)
$length = (([int]$buffer[0]) -shl 8) -bor ([int]$buffer[1])
if ($length -lt 2) { throw "The drive returned an empty table of contents." }

$firstTrack = [int]$buffer[2]
$lastTrack = [int]$buffer[3]
$tracks = New-Object System.Collections.Generic.List[object]
$leadout = 0
$entryCount = [Math]::Min(100, [Math]::Floor(($length - 2) / 8))

for ($i = 0; $i -lt $entryCount; $i++) {
    $base = 4 + ($i * 8)
    $number = [int]$buffer[$base + 2]
    $control = [int]$buffer[$base + 1]
    $minute = [int]$buffer[$base + 5]
    $second = [int]$buffer[$base + 6]
    $frame = [int]$buffer[$base + 7]
    $offset = ($minute * 4500) + ($second * 75) + $frame

    if ($number -eq 170) {
        $leadout = $offset
        break
    }

    if ($number -ge 1 -and $number -le 99 -and (($control -band 0x04) -eq 0)) {
        $tracks.Add([pscustomobject]@{ number = $number; offset = $offset })
    }
}

if ($leadout -le 0) { throw "The drive did not return a lead-out sector." }

$trackCount = $tracks.Count
if ($trackCount -le 0) { throw "No audio tracks were found on this disc." }

$hashText = "{0:X2}{1:X2}" -f $firstTrack, $lastTrack

for ($trackNumber = 0; $trackNumber -le 99; $trackNumber++) {
    if ($trackNumber -eq 0) {
        $offset = $leadout
    } else {
        $offset = 0
        foreach ($track in $tracks) {
            if ($track.number -eq $trackNumber) {
                $offset = $track.offset
                break
            }
        }
    }

    $hashText += "{0:X8}" -f $offset
}

$sha1 = [System.Security.Cryptography.SHA1]::Create()
$discId = [Convert]::ToBase64String($sha1.ComputeHash([System.Text.Encoding]::ASCII.GetBytes($hashText))).Replace("+", ".").Replace("/", "_").Replace("=", "-")
$sha1.Dispose()

$artist = $null
$title = $null
$discNumber = $null
$discCount = $null
$source = "CD table of contents"
$message = "Found $trackCount audio track$(if ($trackCount -eq 1) { '' } else { 's' })."
$lookupError = $null
$cdTextArtist = $null
$cdTextTitle = $null

if ($Lookup -eq "true") {
    try {
        $cdText = [RipBookDisc]::ReadCdText($devicePath)
        if ($cdText -and $cdText.Count -ge 2) {
            if (-not [string]::IsNullOrWhiteSpace($cdText[0])) { $cdTextTitle = $cdText[0].Trim() }
            if (-not [string]::IsNullOrWhiteSpace($cdText[1])) { $cdTextArtist = $cdText[1].Trim() }
        }
    } catch {
    }

    try {
        $uri = "https://musicbrainz.org/ws/2/discid/${discId}?fmt=json&inc=artist-credits"
        $response = Invoke-RestMethod -Uri $uri -Headers @{ "User-Agent" = $UserAgent } -TimeoutSec 15

        $release = $null
        foreach ($candidate in @($response.releases)) {
            if (-not $candidate.media) { continue }

            $matches = $false
            foreach ($medium in @($candidate.media)) {
                foreach ($disc in @($medium.discs)) {
                    if ($disc.id -eq $discId) { $matches = $true; break }
                }
                if ($matches) { break }
            }

            if ($matches) {
                $release = $candidate
                break
            }
        }

        if (-not $release) {
            foreach ($candidate in @($response.releases)) {
                if ($candidate.media) {
                    $release = $candidate
                    break
                }
            }
        }

        if ($release) {
            $artist = (($release.'artist-credit' | ForEach-Object { "$($_.name)$($_.joinphrase)" }) -join "").Trim()
            $title = "$($release.title)".Trim()

            $medium = $null
            foreach ($candidate in @($release.media)) {
                foreach ($disc in @($candidate.discs)) {
                    if ($disc.id -eq $discId) { $medium = $candidate; break }
                }
                if ($medium) { break }
            }
            if (-not $medium -and $release.media) { $medium = @($release.media)[0] }

            if ($medium) { $discNumber = $medium.position }
            $discCount = @($release.media).Count
            $source = "MusicBrainz"

            if ($discNumber -and $discCount) {
                $message = "MusicBrainz identifies this as disc $discNumber of $discCount."
            } else {
                $message = "Matched this disc in MusicBrainz."
            }
        } else {
            $message = "No MusicBrainz release matched this disc. Enter the details manually."
        }
    } catch {
        $lookupError = $_.Exception.Message
        $message = "MusicBrainz lookup failed: $lookupError"
    }

    $usedCdText = $false
    if (-not $artist -and $cdTextArtist) {
        $artist = $cdTextArtist
        $usedCdText = $true
    }
    if (-not $title -and $cdTextTitle) {
        $title = $cdTextTitle
        $usedCdText = $true
    }

    if ($usedCdText) {
        if ($source -eq "MusicBrainz") {
            $source = "MusicBrainz + CD-Text"
        } else {
            $source = "CD-Text"
        }

        if (-not $discNumber) {
            $message = "Read title and author from the disc's CD-Text."
        }
    } elseif ($source -eq "CD table of contents") {
        if ($lookupError) {
            $message = "MusicBrainz lookup failed ($lookupError), and no CD-Text metadata was found. Enter the details manually."
        } else {
            $message = "No MusicBrainz release or CD-Text metadata was found. Enter the details manually."
        }
    }
}

[pscustomobject]@{
    firstTrack = $firstTrack
    lastTrack = $lastTrack
    trackCount = $trackCount
    discId = $discId
    artist = $artist
    title = $title
    discNumber = $discNumber
    discCount = $discCount
    source = $source
    message = $message
} | ConvertTo-Json -Compress
"##;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(windows), allow(dead_code))]
struct ScriptDiscInfo {
    #[serde(default)]
    first_track: Option<u32>,
    #[serde(default)]
    last_track: Option<u32>,
    #[serde(default)]
    track_count: Option<u32>,
    #[serde(default)]
    disc_id: Option<String>,
    #[serde(default)]
    artist: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    disc_number: Option<u32>,
    #[serde(default)]
    disc_count: Option<u32>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

pub async fn read_toc(drive: &str) -> Result<DiscInfo, String> {
    inspect(drive, false).await
}

pub async fn lookup_metadata(drive: &str) -> Result<DiscInfo, String> {
    inspect(drive, true).await
}

#[cfg_attr(not(windows), allow(dead_code))]
pub async fn eject(drive: &str) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = drive;
        Err("Ejecting a disc through the Windows driver is only supported on Windows.".to_string())
    }

    #[cfg(windows)]
    {
        powershell::run_script(
            DISC_SCRIPT,
            &[
                ("Drive", drive),
                ("Action", "eject"),
                ("Lookup", "false"),
                ("UserAgent", USER_AGENT),
            ],
        )
        .await?;

        Ok(())
    }
}

async fn inspect(drive: &str, lookup: bool) -> Result<DiscInfo, String> {
    #[cfg(not(windows))]
    {
        let _ = (drive, lookup);
        Err("Reading disc metadata is only supported on Windows.".to_string())
    }

    #[cfg(windows)]
    {
        let output = powershell::run_script(
            DISC_SCRIPT,
            &[
                ("Drive", drive),
                ("Action", "info"),
                ("Lookup", if lookup { "true" } else { "false" }),
                ("UserAgent", USER_AGENT),
            ],
        )
        .await?;

        let info: ScriptDiscInfo = serde_json::from_str(&output)
            .map_err(|error| format!("Could not read the disc information: {error}"))?;

        let track_count = info.track_count.unwrap_or(0);
        if track_count == 0 {
            return Err(info
                .message
                .unwrap_or_else(|| "No audio tracks were found on this disc.".to_string()));
        }

        Ok(DiscInfo {
            drive: drive.to_string(),
            track_count,
            first_track: info.first_track.unwrap_or(1),
            last_track: info.last_track.unwrap_or(track_count),
            disc_id: info.disc_id,
            artist: info.artist.filter(|value| !value.trim().is_empty()),
            title: info.title.filter(|value| !value.trim().is_empty()),
            disc_number: info.disc_number,
            disc_count: info.disc_count,
            source: info
                .source
                .unwrap_or_else(|| "CD table of contents".to_string()),
            message: info.message.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_disc_script_output() {
        let info: ScriptDiscInfo = serde_json::from_str(
            r#"{"firstTrack":1,"lastTrack":12,"trackCount":12,"discId":"abc-","artist":"Author","title":"Book","discNumber":2,"discCount":3,"source":"MusicBrainz","message":"ok"}"#,
        )
        .unwrap();

        assert_eq!(info.track_count, Some(12));
        assert_eq!(info.artist.as_deref(), Some("Author"));
        assert_eq!(info.disc_number, Some(2));
        assert_eq!(info.disc_count, Some(3));
    }
}
