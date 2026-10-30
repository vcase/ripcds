use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::disc;
use crate::freac;
use crate::freac_install;
use crate::manifest;
use crate::models::{
    AppInfo, BookManifest, BookSummary, CreateBookRequest, DiscInfo, DiscStatus, DriveInfo,
    FreacInfo, FreacInstallResult, LogPayload, RipRequest, StatePayload,
};
use crate::settings;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Default)]
pub struct AppState {
    pub rip_cancel: tokio::sync::Mutex<Option<CancellationToken>>,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> Result<AppInfo, String> {
    let drives = detect_drives().unwrap_or_default();
    let saved = settings::load(&app);

    let default_audio_root = saved
        .audio_root
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_audio_root().to_string_lossy().to_string());

    let default_freaccmd_path = saved
        .freaccmd_path
        .filter(|value| Path::new(value).is_file())
        .unwrap_or_else(|| freac::default_freaccmd_path(&app));

    let default_drive = normalize_drive(&drives, saved.drive.as_deref());
    let freaccmd_found = Path::new(&default_freaccmd_path).is_file();

    Ok(AppInfo {
        platform: std::env::consts::OS.to_string(),
        default_audio_root,
        default_freaccmd_path,
        default_drive,
        freaccmd_found,
        drives,
    })
}

#[tauri::command]
pub fn list_drives() -> Result<Vec<DriveInfo>, String> {
    detect_drives()
}

#[tauri::command]
pub async fn lookup_disc_metadata(drive: String) -> Result<DiscInfo, String> {
    disc::lookup_metadata(&drive).await
}

#[tauri::command]
pub async fn install_freac(
    app: AppHandle,
    version: Option<String>,
) -> Result<FreacInstallResult, String> {
    freac_install::install(&app, version).await
}

#[tauri::command]
pub fn set_last_disc(
    app: AppHandle,
    book_dir: String,
    last_disc: u32,
) -> Result<BookManifest, String> {
    let mut manifest = manifest::load_manifest(&book_dir)?;

    if manifest.book_dir != book_dir {
        manifest.book_dir = book_dir.clone();
    }

    if manifest::set_last_disc(&mut manifest, last_disc)? {
        manifest::save_manifest(&manifest)?;
        settings::remember_book(&app, &manifest);
        emit_manifest(&app, &manifest);
    }

    Ok(manifest)
}

#[tauri::command]
pub fn list_books(root: String) -> Result<Vec<BookSummary>, String> {
    manifest::list_summaries(&root)
}

#[tauri::command]
pub fn create_book(app: AppHandle, request: CreateBookRequest) -> Result<BookManifest, String> {
    let manifest = manifest::create_or_open(&request)?;
    settings::remember_book(&app, &manifest);
    Ok(manifest)
}

#[tauri::command]
pub fn load_book(app: AppHandle, book_dir: String) -> Result<BookManifest, String> {
    let mut manifest = manifest::load_manifest(&book_dir)?;
    let mut changed = false;

    if manifest.book_dir != book_dir {
        manifest.book_dir = book_dir.clone();
        changed = true;
    }

    if let Some(root) = Path::new(&book_dir)
        .parent()
        .and_then(|parent| parent.parent())
    {
        let root = root.to_string_lossy().to_string();
        if manifest.audio_root != root {
            manifest.audio_root = root;
            changed = true;
        }
    }

    if manifest::reconcile(&mut manifest) {
        changed = true;
    }

    let saved = settings::load(&app);
    if !Path::new(&manifest.freaccmd_path).is_file()
        && let Some(saved_path) = saved
            .freaccmd_path
            .as_deref()
            .filter(|path| Path::new(path).is_file())
    {
        manifest.freaccmd_path = saved_path.to_string();
        changed = true;
    }

    if changed {
        manifest.updated_at = manifest::now_millis();
        manifest::save_manifest(&manifest)?;
    }

    settings::remember_book(&app, &manifest);
    Ok(manifest)
}

#[tauri::command]
pub fn probe_freaccmd(path: String) -> FreacInfo {
    freac::probe_freaccmd(&path)
}

#[tauri::command]
pub async fn cancel_rip(state: State<'_, AppState>) -> Result<(), String> {
    let guard = state.rip_cancel.lock().await;
    match guard.as_ref() {
        Some(token) => {
            token.cancel();
            Ok(())
        }
        None => Err("No rip is currently running.".to_string()),
    }
}

#[tauri::command]
pub async fn eject_drive(drive: String) -> Result<(), String> {
    eject_drive_inner(&drive).await
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err(format!("Folder not found: {path}"));
    }

    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("explorer");
        command.arg(&path_buf);
        command.creation_flags(CREATE_NO_WINDOW);
        command
            .spawn()
            .map_err(|error| format!("Could not open the folder: {error}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_buf)
            .spawn()
            .map_err(|error| format!("Could not open the folder: {error}"))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_buf)
            .spawn()
            .map_err(|error| format!("Could not open the folder: {error}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Opening a folder is not supported on this platform.".to_string())
}

#[tauri::command]
pub async fn rip_disc(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RipRequest,
) -> Result<BookManifest, String> {
    let token = CancellationToken::new();
    {
        let mut active = state.rip_cancel.lock().await;
        if active.is_some() {
            return Err("A rip is already running.".to_string());
        }
        *active = Some(token.clone());
    }

    let result = rip_disc_inner(&app, &request, &token).await;

    if let Err(error) = &result {
        mark_failed(&app, &request, error);
    }

    *state.rip_cancel.lock().await = None;
    result
}

async fn rip_disc_inner(
    app: &AppHandle,
    request: &RipRequest,
    token: &CancellationToken,
) -> Result<BookManifest, String> {
    let mut manifest = manifest::load_manifest(&request.book_dir)?;

    if manifest.book_dir != request.book_dir {
        manifest.book_dir = request.book_dir.clone();
    }
    if let Some(root) = Path::new(&request.book_dir)
        .parent()
        .and_then(|parent| parent.parent())
    {
        manifest.audio_root = root.to_string_lossy().to_string();
    }

    if manifest::reconcile(&mut manifest) {
        manifest.updated_at = manifest::now_millis();
        manifest::save_manifest(&manifest)?;
    }

    let drive_selection = if request.drive.trim().is_empty() {
        manifest.drive.clone()
    } else {
        request.drive.trim().to_string()
    };
    let (drive_index, drive_letter) = resolve_drive(&drive_selection)?;

    let freac_path_string = if !request.freaccmd_path.trim().is_empty() {
        request.freaccmd_path.trim().to_string()
    } else if !manifest.freaccmd_path.trim().is_empty() {
        manifest.freaccmd_path.trim().to_string()
    } else {
        freac::find_freaccmd(app)
            .map(|path| path.to_string_lossy().to_string())
            .ok_or_else(|| "freaccmd.exe was not found. Choose its path in Settings.".to_string())?
    };

    let freac_path = PathBuf::from(&freac_path_string);
    if !freac_path.is_file() {
        return Err(format!(
            "freaccmd.exe was not found at {}. Choose its path in Settings.",
            freac_path.display()
        ));
    }

    let probe = freac::probe_freaccmd(&freac_path_string);
    if !probe.exists {
        return Err(probe.message);
    }

    if !manifest
        .discs
        .iter()
        .any(|disc| disc.index == request.disc_index)
    {
        return Err(format!(
            "Disc {} is not part of this book.",
            request.disc_index
        ));
    }

    manifest.drive = drive_letter.clone();
    manifest.freaccmd_path = freac_path_string.clone();

    {
        let disc = manifest
            .discs
            .iter_mut()
            .find(|disc| disc.index == request.disc_index)
            .ok_or_else(|| format!("Disc {} is not part of this book.", request.disc_index))?;
        disc.status = DiscStatus::Ripping;
        disc.error = None;
        disc.files.clear();
        disc.track_count = None;
        disc.expected_track_count = None;
        disc.completed_at = None;
    }

    manifest.updated_at = manifest::now_millis();
    manifest::save_manifest(&manifest)?;
    settings::remember_book(app, &manifest);
    emit_manifest(app, &manifest);
    emit_state(app, request.disc_index, "starting", "Preparing disc...");

    if token.is_cancelled() {
        return Err("Rip cancelled.".to_string());
    }

    let book_dir = PathBuf::from(&manifest.book_dir);
    let wip_dir = book_dir.join(format!("wip_cd_{:02}", request.disc_index));
    if wip_dir.exists() {
        fs::remove_dir_all(&wip_dir)
            .map_err(|e| format!("Could not clear the previous work folder: {e}"))?;
    }
    fs::create_dir_all(&wip_dir).map_err(|e| format!("Could not create the work folder: {e}"))?;

    let track_total = disc::read_toc(&drive_letter)
        .await
        .ok()
        .map(|info| info.track_count)
        .filter(|count| *count > 0);

    if let Some(total) = track_total {
        if let Some(disc) = manifest
            .discs
            .iter_mut()
            .find(|disc| disc.index == request.disc_index)
        {
            disc.expected_track_count = Some(total);
        }
        manifest.updated_at = manifest::now_millis();
        manifest::save_manifest(&manifest)?;
        emit_manifest(app, &manifest);
    }

    let log_path = book_dir.join(format!("rip_cd_{:02}.log", request.disc_index));
    let log_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .map_err(|e| format!("Could not create the rip log: {e}"))?;

    let args = freac::build_rip_args(&probe.version, &drive_index.to_string(), &wip_dir);
    let mut command = Command::new(&freac_path);
    if let Some(parent) = freac_path.parent() {
        command.current_dir(parent);
    }
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(windows)]
    command.as_std_mut().creation_flags(CREATE_NO_WINDOW);

    let rip_message = match track_total {
        Some(total) => format!("Ripping up to {total} tracks with fre:ac..."),
        None => "Ripping with fre:ac. This can take a while.".to_string(),
    };
    emit_state_progress(
        app,
        request.disc_index,
        "ripping",
        &rip_message,
        None,
        track_total,
        track_total.map(|_| 0),
    );

    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start freaccmd: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let log = Arc::new(Mutex::new(log_file));

    let stdout_task = tokio::spawn(pump_lines(
        stdout,
        app.clone(),
        request.disc_index,
        log.clone(),
    ));
    let stderr_task = tokio::spawn(pump_lines(
        stderr,
        app.clone(),
        request.disc_index,
        log.clone(),
    ));

    let progress_cancel = CancellationToken::new();
    let progress_task = tokio::spawn(watch_rip_progress(
        app.clone(),
        request.disc_index,
        wip_dir.clone(),
        track_total,
        progress_cancel.clone(),
    ));

    let status = tokio::select! {
        _ = token.cancelled() => {
            progress_cancel.cancel();
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            let _ = progress_task.await;
            return Err("Rip cancelled.".to_string());
        }
        result = child.wait() => result,
    }
    .map_err(|e| format!("Could not wait for freaccmd: {e}"))?;

    progress_cancel.cancel();
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let _ = progress_task.await;

    if !status.success() {
        let code = status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "an unknown error".to_string());
        return Err(format!(
            "freaccmd exited with {code}. See the rip log for details."
        ));
    }

    if token.is_cancelled() {
        return Err("Rip cancelled.".to_string());
    }

    emit_state_progress(
        app,
        request.disc_index,
        "installing",
        "Normalizing track names and installing files...",
        track_total,
        track_total,
        Some(100),
    );

    let files = normalize_and_install(&book_dir, request.disc_index, &wip_dir)?;
    let track_count = files.len() as u32;

    let expected_track_count = manifest
        .discs
        .iter()
        .find(|disc| disc.index == request.disc_index)
        .and_then(|disc| disc.expected_track_count);

    if let Some(expected) = expected_track_count
        && track_count < expected
    {
        return Err(format!(
            "freaccmd produced only {track_count} of the expected {expected} tracks. The disc was not marked complete; re-rip it."
        ));
    }

    {
        let disc = manifest
            .discs
            .iter_mut()
            .find(|disc| disc.index == request.disc_index)
            .ok_or_else(|| format!("Disc {} is not part of this book.", request.disc_index))?;
        disc.status = DiscStatus::Done;
        disc.files = files;
        disc.track_count = Some(track_count);
        disc.completed_at = Some(manifest::now_millis());
        disc.error = None;
    }

    manifest.updated_at = manifest::now_millis();
    manifest::save_manifest(&manifest)?;
    emit_manifest(app, &manifest);
    emit_state_progress(
        app,
        request.disc_index,
        "done",
        &format!("Installed {track_count} tracks."),
        Some(track_count),
        Some(track_count),
        Some(100),
    );

    if !drive_letter.is_empty()
        && let Err(error) = eject_drive_inner(&drive_letter).await
    {
        if let Some(disc) = manifest
            .discs
            .iter_mut()
            .find(|disc| disc.index == request.disc_index)
        {
            disc.error = Some(format!("Ripped successfully, but eject failed: {error}"));
        }
        manifest.updated_at = manifest::now_millis();
        let _ = manifest::save_manifest(&manifest);
        emit_manifest(app, &manifest);
    }

    Ok(manifest)
}

fn mark_failed(app: &AppHandle, request: &RipRequest, error: &str) {
    let Ok(mut manifest) = manifest::load_manifest(&request.book_dir) else {
        return;
    };

    let Some(disc) = manifest
        .discs
        .iter_mut()
        .find(|disc| disc.index == request.disc_index)
    else {
        return;
    };

    if disc.status != DiscStatus::Ripping {
        return;
    }

    if error == "Rip cancelled." {
        disc.status = DiscStatus::Pending;
        disc.error = Some("Rip cancelled. You can retry this disc.".to_string());
    } else {
        disc.status = DiscStatus::Failed;
        disc.error = Some(error.to_string());
    }

    manifest.updated_at = manifest::now_millis();
    let _ = manifest::save_manifest(&manifest);
    emit_manifest(app, &manifest);
    emit_state(app, request.disc_index, "failed", error);
}

async fn watch_rip_progress(
    app: AppHandle,
    disc: u32,
    wip_dir: PathBuf,
    track_total: Option<u32>,
    cancel: CancellationToken,
) {
    let mut last_count = 0u32;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_millis(600)) => {}
        }

        let count = count_audio_files(&wip_dir);
        if count == 0 || count == last_count {
            continue;
        }
        last_count = count;

        let (message, percent) = match track_total {
            Some(total) if total > 0 => {
                let done = count.min(total);
                let percent = (done * 100 / total).min(99);
                (format!("Ripped {done} of {total} tracks..."), Some(percent))
            }
            _ => (
                format!(
                    "Ripped {count} track{} so far...",
                    if count == 1 { "" } else { "s" }
                ),
                None,
            ),
        };

        emit_state_progress(
            &app,
            disc,
            "ripping",
            &message,
            Some(count),
            track_total,
            percent,
        );
    }
}

fn count_audio_files(dir: &Path) -> u32 {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    let path = entry.path();
                    path.is_file()
                        && is_audio_file(&path)
                        && fs::metadata(&path)
                            .map(|metadata| metadata.len() > 0)
                            .unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0)
}

async fn pump_lines<R>(reader: Option<R>, app: AppHandle, disc: u32, log: Arc<Mutex<File>>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let Some(reader) = reader else {
        return;
    };

    let mut reader = tokio::io::BufReader::new(reader);
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        let read = match reader.read_until(b'\n', &mut buffer).await {
            Ok(read) => read,
            Err(_) => break,
        };
        if read == 0 {
            break;
        }

        let line = String::from_utf8_lossy(&buffer)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if line.is_empty() {
            continue;
        }

        if let Ok(mut file) = log.lock() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }

        let _ = app.emit("rip-log", LogPayload { disc, line });
    }
}

fn normalize_and_install(
    book_dir: &Path,
    disc_index: u32,
    wip_dir: &Path,
) -> Result<Vec<String>, String> {
    let entries =
        fs::read_dir(wip_dir).map_err(|e| format!("Could not read the work folder: {e}"))?;

    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_audio_file(path))
        .collect();

    if candidates.is_empty() {
        return Err(
            "freaccmd finished, but it did not produce any audio files. Check the rip log."
                .to_string(),
        );
    }

    let mut numbered: Vec<(u32, PathBuf)> = candidates
        .iter()
        .filter_map(|path| extract_track_number(path).map(|track| (track, path.clone())))
        .collect();

    let all_numbered = numbered.len() == candidates.len();
    let unique_numbers = numbered
        .iter()
        .map(|(track, _)| *track)
        .collect::<HashSet<_>>()
        .len()
        == numbered.len()
        && numbered.iter().all(|(track, _)| *track >= 1);

    let track_numbers: Option<Vec<u32>> = if all_numbered && unique_numbers {
        numbered.sort_by_key(|(track, _)| *track);
        candidates = numbered.iter().map(|(_, path)| path.clone()).collect();
        Some(numbered.iter().map(|(track, _)| *track).collect())
    } else {
        candidates.sort_by(|a, b| {
            modified_time(a)
                .cmp(&modified_time(b))
                .then_with(|| a.file_name().cmp(&b.file_name()))
        });
        None
    };

    let mut installed = Vec::with_capacity(candidates.len());
    let mut seen_expected = HashSet::new();

    for (offset, source) in candidates.iter().enumerate() {
        let track_number = track_numbers
            .as_ref()
            .and_then(|numbers| numbers.get(offset).copied())
            .unwrap_or((offset + 1) as u32);

        if !seen_expected.insert(track_number) {
            return Err(format!(
                "Track number {track_number} appeared more than once. Check the ripped filenames."
            ));
        }

        let extension = source
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy().to_ascii_lowercase()))
            .unwrap_or_else(|| ".mp3".to_string());

        let target_name = format!("cd{disc_index:02}_t{track_number:02}{extension}",);
        let target = book_dir.join(&target_name);

        move_file(source, &target)?;
        installed.push(target_name);
    }

    let _ = fs::remove_dir_all(wip_dir);
    Ok(installed)
}

fn extract_track_number(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    if let Ok(number) = stem.parse::<u32>() {
        return Some(number);
    }

    let lower = stem.to_ascii_lowercase();
    if let Some(index) = lower.rfind("track ") {
        let rest = &stem[index + 6..];
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(number) = digits.parse::<u32>() {
            return Some(number);
        }
    }

    let digits: String = stem
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

fn is_audio_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .as_deref(),
        Some("mp3")
            | Some("flac")
            | Some("m4a")
            | Some("aac")
            | Some("ogg")
            | Some("opus")
            | Some("wav")
            | Some("wma")
            | Some("aiff")
            | Some("aif")
    )
}

fn modified_time(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn move_file(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_file(target)
            .map_err(|e| format!("Could not replace {}: {e}", target.display()))?;
    }

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, target)
                .map_err(|e| format!("Could not copy {}: {e}", source.display()))?;
            fs::remove_file(source)
                .map_err(|e| format!("Could not remove {}: {e}", source.display()))?;
            Ok(())
        }
    }
}

fn default_audio_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("AudioBooks")
}

fn drive_matches(info: &DriveInfo, value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }

    let no_colon = value.trim_end_matches(':');
    info.letter.eq_ignore_ascii_case(value)
        || info
            .letter
            .trim_end_matches(':')
            .eq_ignore_ascii_case(no_colon)
        || info.index.to_string() == value
}

fn normalize_drive(drives: &[DriveInfo], saved: Option<&str>) -> String {
    let saved = saved.map(str::trim).filter(|value| !value.is_empty());

    if let Some(value) = saved
        && let Some(found) = drives.iter().find(|drive| drive_matches(drive, value))
    {
        return found.letter.clone();
    }

    if let Some(first) = drives.first() {
        return first.letter.clone();
    }

    saved.unwrap_or("0").to_string()
}

fn resolve_drive(drive: &str) -> Result<(u32, String), String> {
    let drives = detect_drives().unwrap_or_default();
    let value = drive.trim();

    if let Some(found) = drives.iter().find(|info| drive_matches(info, value)) {
        return Ok((found.index, found.letter.clone()));
    }

    if let Ok(index) = value.parse::<u32>() {
        if let Some(found) = drives.iter().find(|info| info.index == index) {
            return Ok((found.index, found.letter.clone()));
        }

        if drives.is_empty() {
            return Ok((index, value.to_string()));
        }

        return Err(format!(
            "Drive {index} is no longer available. Refresh the drive list and choose a drive."
        ));
    }

    Err(format!(
        "Drive {value} is not currently detected. Refresh the drive list and choose a drive."
    ))
}

fn detect_drives() -> Result<Vec<DriveInfo>, String> {
    #[cfg(windows)]
    {
        let script = r#"
$i = 0
Get-CimInstance Win32_CDROMDrive | ForEach-Object {
  "{0}`t{1}`t{2}" -f $i, $_.Drive, $_.Name
  $i++
}
"#;

        let mut command = std::process::Command::new("powershell");
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        command.creation_flags(CREATE_NO_WINDOW);

        let output = command
            .output()
            .map_err(|e| format!("Could not query optical drives: {e}"))?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut drives = Vec::new();

        for line in text.lines() {
            let mut parts = line.trim().splitn(3, '\t');
            let Some(index) = parts.next() else {
                continue;
            };
            let Some(letter) = parts.next() else {
                continue;
            };
            let name = parts.next().unwrap_or("CD/DVD drive");
            let Ok(index) = index.trim().parse::<u32>() else {
                continue;
            };
            drives.push(DriveInfo {
                index,
                letter: letter.trim().to_string(),
                name: name.trim().to_string(),
            });
        }

        Ok(drives)
    }

    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn drive_letter_for_eject(drive: &str) -> Result<String, String> {
    let value = drive.trim();
    let upper = value.to_ascii_uppercase();

    if upper.len() == 1 && upper.as_bytes()[0].is_ascii_uppercase() {
        return Ok(format!("{upper}:"));
    }

    if upper.len() == 2 && upper.ends_with(':') && upper.as_bytes()[0].is_ascii_uppercase() {
        return Ok(upper);
    }

    resolve_drive(value).map(|(_, letter)| letter)
}

async fn eject_drive_inner(drive: &str) -> Result<(), String> {
    #[cfg(not(windows))]
    let _ = drive;

    #[cfg(windows)]
    {
        let letter = drive_letter_for_eject(drive)?;
        disc::eject(&letter).await?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("drutil")
            .arg("eject")
            .output()
            .await
            .map_err(|e| format!("Could not run drutil: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let output = Command::new("eject")
            .output()
            .await
            .map_err(|e| format!("Could not run eject: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Ejecting a disc is not supported on this platform.".to_string())
}

fn emit_manifest(app: &AppHandle, manifest: &BookManifest) {
    let _ = app.emit("book-updated", manifest.clone());
}

fn emit_state(app: &AppHandle, disc: u32, stage: &str, message: &str) {
    emit_state_progress(app, disc, stage, message, None, None, None);
}

fn emit_state_progress(
    app: &AppHandle,
    disc: u32,
    stage: &str,
    message: &str,
    track: Option<u32>,
    track_total: Option<u32>,
    percent: Option<u32>,
) {
    let _ = app.emit(
        "rip-state",
        StatePayload {
            disc,
            stage: stage.to_string(),
            message: message.to_string(),
            track,
            track_total,
            percent,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_track_numbers_from_freac_names() {
        assert_eq!(extract_track_number(Path::new("01.mp3")), Some(1));
        assert_eq!(extract_track_number(Path::new("12.mp3")), Some(12));
        assert_eq!(
            extract_track_number(Path::new("Audio CD 0 - Track 7.mp3")),
            Some(7)
        );
        assert_eq!(
            extract_track_number(Path::new("03 - Chapter One.mp3")),
            Some(3)
        );
        assert_eq!(extract_track_number(Path::new("Chapter.mp3")), None);
    }

    #[test]
    fn normalizes_and_installs_in_numeric_order() {
        let book = tempfile::tempdir().unwrap();
        let wip = book.path().join("wip_cd_01");
        fs::create_dir_all(&wip).unwrap();

        fs::write(wip.join("10.mp3"), b"ten").unwrap();
        fs::write(wip.join("2.mp3"), b"two").unwrap();
        fs::write(wip.join("1.mp3"), b"one").unwrap();
        fs::write(wip.join("notes.txt"), b"not audio").unwrap();

        let files = normalize_and_install(book.path(), 1, &wip).unwrap();

        assert_eq!(
            files,
            vec![
                "cd01_t01.mp3".to_string(),
                "cd01_t02.mp3".to_string(),
                "cd01_t10.mp3".to_string()
            ]
        );
        assert_eq!(fs::read(book.path().join("cd01_t01.mp3")).unwrap(), b"one");
        assert!(!wip.exists());
        assert!(!book.path().join("notes.txt").exists());
    }

    #[test]
    fn supports_three_digit_disc_numbers() {
        let book = tempfile::tempdir().unwrap();
        let wip = book.path().join("wip_cd_100");
        fs::create_dir_all(&wip).unwrap();
        fs::write(wip.join("1.mp3"), b"one").unwrap();

        let files = normalize_and_install(book.path(), 100, &wip).unwrap();
        assert_eq!(files, vec!["cd100_t01.mp3".to_string()]);
    }

    #[test]
    fn normalizes_saved_drive_selection_to_a_letter() {
        let drives = vec![
            DriveInfo {
                index: 0,
                letter: "D:".to_string(),
                name: "Drive one".to_string(),
            },
            DriveInfo {
                index: 1,
                letter: "E:".to_string(),
                name: "Drive two".to_string(),
            },
        ];

        assert_eq!(normalize_drive(&drives, Some("0")), "D:");
        assert_eq!(normalize_drive(&drives, Some("e")), "E:");
        assert_eq!(normalize_drive(&drives, Some("missing")), "D:");
        assert_eq!(normalize_drive(&[], Some("E:")), "E:");
    }

    #[test]
    fn counts_non_empty_audio_files_for_progress() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("01.mp3"), b"audio").unwrap();
        fs::write(dir.path().join("02.flac"), b"").unwrap();
        fs::write(dir.path().join("notes.txt"), b"not audio").unwrap();

        assert_eq!(count_audio_files(dir.path()), 1);
    }

    #[test]
    fn normalizes_drive_letters_for_eject() {
        assert_eq!(drive_letter_for_eject("d").unwrap(), "D:");
        assert_eq!(drive_letter_for_eject("e:").unwrap(), "E:");
        assert_eq!(drive_letter_for_eject(" D: ").unwrap(), "D:");
    }
}
