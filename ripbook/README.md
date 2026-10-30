# RipBook

RipBook is a small Tauri desktop app for turning multi-disc audiobooks into a
resumable queue of `cdNN_tNN.mp3` files. It is the GUI successor to the
`ripBook.bat` script in the repository root.

## What it does

- Prompts for author, title, first disc, and last disc.
- Can read the inserted disc's table of contents, its CD-Text, and look up the
  disc ID on MusicBrainz to pre-fill author, title, and (when MusicBrainz knows
  the release) the total number of discs. There is a **Read from disc** button
  next to Author, and RipBook also tries this automatically when Create is
  pressed with a blank author or title. Audio CDs themselves do not carry a
  “disc N of M” field, so the disc range can still be entered or corrected
  manually.
- Lets you update the number of discs in a book later, so a book started with
  the default single-disc range can be extended without re-ripping anything.
- Creates `%USERPROFILE%\AudioBooks\<Author>\<Book>` (or another library folder).
- Rips one inserted disc at a time with `freaccmd.exe`.
- Stores each disc in a temporary `wip_cd_NN` folder, renames the tracks to
  `cdNN_tNN.mp3`, then installs them into the book folder.
- Writes a `.ripbook.json` manifest in the book folder after every state change.
- Resumes at the first pending or failed disc after the app is closed, the
  machine reboots, or a rip is cancelled.
- Verifies completed discs on load and marks missing/empty files as pending.
- Imports existing `cdNN_tNN.*` files, so books ripped with the old batch script
  can be opened and continued.
- Shows separate book progress and current-disc progress while a rip is running.
- Remembers the last library folder, drive, and fre:ac path.
- Refreshes the optical-drive list on window focus or with the Refresh button,
  so a drive connected after startup can be used without restarting RipBook.
- Lets you choose a target disc (including a completed one) before re-ripping.
- Can download the latest fre:ac release, or a specific version, from the
  official fre:ac GitHub releases page into RipBook's app-data folder.
- Detects both the fre:ac 1.0 (`-track` / `-t <timeout>`) and fre:ac 1.1+
  (`--track` / `--timeout`) command-line syntaxes.
- Saves a `rip_cdNN.log` next to each book.
- Ejects the disc after a successful install.
- When the Windows uninstaller's **Delete the application data** option is
  selected, also removes RipBook manifests and rip logs from known library
  roots. Ripped audio files are left in place.

RipBook does **not** write ID3 tags or playlists. The normalized filenames are
the ordering mechanism, just like the original batch script.

## Requirements

For running the Windows build:

- Windows 10 or 11.
- An optical drive that can read audio CDs.
- `freaccmd.exe` from fre:ac. Release builds produced by the GitHub Actions
  workflow bundle the fre:ac 1.1.7 portable Windows build, so the installer is
  self-contained. The app can also download the latest release (or a specific
  version) from the official fre:ac GitHub releases; for local development you
  can install fre:ac or point RipBook at a portable copy.

For building:

- Node.js 22 or newer.
- Rust stable.
- On macOS: the Tauri prerequisites for macOS development.
- To build the Windows installer: a Windows machine or the included GitHub
  Actions workflow. Tauri's Windows installer toolchain (NSIS, WebView2 build
  support) is not available on macOS.

## Development on macOS

The UI and the resume/state logic can be developed on macOS. CD ripping itself
requires Windows and fre:ac.

```bash
cd ripbook
npm install
npm run tauri dev
```

Useful checks:

```bash
npm run build
cd src-tauri && cargo test
```

The macOS development build will show a warning that CD ripping requires
Windows. Disc TOC/metadata reading and the in-app fre:ac downloader also require
Windows. A fake ripper is not included; the UI can still be opened and the
manifest/import/resume paths can be exercised with test folders.

## Build the Windows installer

The supported path is GitHub Actions:

1. Push the repository to GitHub.
2. Open **Actions → Build Windows installer → Run workflow**.
3. Download the `ripbook-windows-x64-<run number>` artifact.

Pushing a `v*` tag also creates a GitHub release and uploads the installer to it.

The workflow:

1. Installs Node and Rust on a `windows-latest` runner.
2. Runs the Rust test suite.
3. Runs `scripts/fetch-freac.ps1`, which downloads the fre:ac 1.1.7 Windows x64
   portable zip and copies it into `src-tauri/resources/freac`.
4. Runs `npm run build:windows`.
5. Uploads the NSIS installer.

To build manually on a Windows machine:

```powershell
cd ripbook
npm ci
pwsh -File scripts/fetch-freac.ps1
npm run build:windows
```

The installer is written to:

```text
src-tauri/target/release/bundle/nsis/
```

If you do not want to bundle fre:ac, skip the fetch script and let the user
choose an installed `freaccmd.exe` in the app's settings.

## Resume state

The manifest is stored in the book folder, not in the app's data directory, so
moving or backing up the book folder preserves its progress.

```text
<Book>/
  .ripbook.json
  cd01_t01.mp3
  cd01_t02.mp3
  ...
  cd02_t01.mp3
  rip_cd_01.log
  rip_cd_02.log
```

Disc states are `pending`, `ripping`, `done`, and `failed`.

On load, RipBook:

- changes an interrupted `ripping` disc to `failed` so partial files cannot be
  mistaken for a completed disc, and clears the stale work folder on the next
  attempt;
- verifies every `done` disc's files exist, are non-empty, and meet the expected
  track count recorded from the disc TOC;
- keeps `failed` discs for an explicit retry;
- imports existing `cdNN_tNN.*` files for discs that are still pending, but not
  when the disc has a known expected track count that the files do not meet.

Resume is **per disc**, not per track. If the app stops in the middle of disc 4,
it re-rips disc 4. This keeps the implementation simple and avoids relying on
partial MP3 files.

Last-used library, drive, and fre:ac path are stored separately in the OS app
config directory (`%APPDATA%\com.ripbook.desktop\settings.json` on Windows).
They are convenience defaults only; each book's manifest remains authoritative.

## Limitations

- Resume is per disc, not per track.
- Audio CDs do not contain a standard total-disc-count field. RipBook can
  suggest the count from a MusicBrainz match, but otherwise the user must enter
  the range up front; it can be extended later from the book screen.
- Metadata pre-fill depends on a matching MusicBrainz release or CD-Text on the
  disc. Discs with neither must be entered manually.
- The drive selector uses the drive letter reported by Windows. It refreshes on
  window focus and with the Refresh button; a drive connected while RipBook is
  running appears after one of those refreshes.
- Files are encoded with fre:ac's default LAME MP3 encoder. There is no bitrate
  or output-format picker yet.
- RipBook does not write ID3 tags or playlists. It relies on the `cdNN_tNN`
  filenames for ordering.
- There is no TOC fingerprint yet. The app trusts the user to insert the disc
  shown in the UI.
- The in-app fre:ac downloader targets the official Windows x64/ARM64 portable
  ZIP releases. It does not install the fre:ac GUI or use a package manager.
- A hard crash or forced quit during a rip may leave a `freaccmd.exe` process
  running; close it before retrying that disc.

## Packaging and signing

The default NSIS build is a normal one-click Windows installer. For distribution
to other people:

- Sign the installer with a Windows code-signing certificate; otherwise
  SmartScreen will show an “Unknown publisher” warning.
- fre:ac is GPLv2. Release builds that bundle fre:ac must comply with the GPL:
  include its license and source-code offer. The fetch script copies fre:ac's
  `COPYING` and `Readme` into the bundled folder. fre:ac's source is available
  at <https://github.com/enzo1982/freac>.
- RipBook uses the system WebView2 runtime, which is included with Windows 11
  and normally present on Windows 10.

## Project layout

```text
src/                     React UI
src-tauri/src/commands.rs Tauri commands, ripping, cancellation, eject
src-tauri/src/disc.rs     CD TOC reader and MusicBrainz metadata lookup
src-tauri/src/freac.rs    fre:ac discovery, version detection, argument builder
src-tauri/src/freac_install.rs downloads/installs fre:ac releases
src-tauri/src/manifest.rs manifest, atomic writes, resume/import logic
src-tauri/src/powershell.rs embedded PowerShell script runner
src-tauri/src/settings.rs last-used library/drive/fre:ac path
src-tauri/src/models.rs   shared data types
src-tauri/resources/freac optional bundled fre:ac payload
src-tauri/resources/purge-installer-data.ps1 uninstall-time manifest cleanup
src-tauri/windows/nsis-hooks.nsh NSIS installer/uninstaller hooks
scripts/fetch-freac.ps1  downloads the portable fre:ac Windows build
../.github/workflows/    Windows NSIS build (repository root)
```
