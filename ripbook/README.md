# RipBook

RipBook is a small Tauri desktop app for turning multi-disc audiobooks into a
resumable queue of `cdNN_tNN.mp3` files. It is the GUI successor to the
`ripBook.bat` script in the repository root.

## What it does

- Prompts for author, title, first disc, and last disc.
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
- Detects both the fre:ac 1.0 (`-track` / `-t <timeout>`) and fre:ac 1.1+
  (`--track` / `--timeout`) command-line syntaxes.
- Saves a `rip_cdNN.log` next to each book.
- Remembers the last library folder, drive, and fre:ac path.
- Ejects the disc after a successful install.

RipBook does **not** write ID3 tags or playlists. The normalized filenames are
the ordering mechanism, just like the original batch script.

## Requirements

For running the Windows build:

- Windows 10 or 11.
- An optical drive that can read audio CDs.
- `freaccmd.exe` from fre:ac. Release builds produced by the GitHub Actions
  workflow bundle the fre:ac 1.1.7 portable Windows build, so the installer is
  self-contained. For local development, install fre:ac or point RipBook at a
  portable copy.

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
Windows. A fake ripper is not included; the UI can still be opened and the
manifest/import/resume paths can be exercised with test folders.

## Build the Windows installer

The supported path is GitHub Actions:

1. Push the repository to GitHub.
2. Open **Actions → Build Windows installer → Run workflow**.
3. Download the `ripbook-windows-x64` artifact.

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

- changes an interrupted `ripping` disc back to `pending` and clears the stale
  work folder on the next attempt;
- verifies every `done` disc's files exist and are non-empty;
- keeps `failed` discs for an explicit retry;
- imports existing `cdNN_tNN.*` files for discs that are still pending.

Resume is **per disc**, not per track. If the app stops in the middle of disc 4,
it re-rips disc 4. This keeps the implementation simple and avoids relying on
partial MP3 files.

Last-used library, drive, and fre:ac path are stored separately in the OS app
config directory (`%APPDATA%\com.ripbook.desktop\settings.json` on Windows).
They are convenience defaults only; each book's manifest remains authoritative.

## Limitations

- Resume is per disc, not per track.
- The drive selector uses Windows' optical-drive enumeration order. On a
  multi-drive machine, verify the drive letter shown next to the selected index.
- Files are encoded with fre:ac's default LAME MP3 encoder. There is no bitrate
  or output-format picker yet.
- RipBook does not write ID3 tags or playlists. It relies on the `cdNN_tNN`
  filenames for ordering.
- There is no TOC fingerprint yet. The app trusts the user to insert the disc
  shown in the UI.
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
src-tauri/src/freac.rs    fre:ac discovery, version detection, argument builder
src-tauri/src/manifest.rs manifest, atomic writes, resume/import logic
src-tauri/src/settings.rs last-used library/drive/fre:ac path
src-tauri/src/models.rs   shared data types
src-tauri/resources/freac optional bundled fre:ac payload
scripts/fetch-freac.ps1  downloads the portable fre:ac Windows build
../.github/workflows/    Windows NSIS build (repository root)
```
