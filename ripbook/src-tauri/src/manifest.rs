use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use atomicwrites::{AllowOverwrite, AtomicFile};

use crate::models::{BookManifest, BookSummary, CreateBookRequest, DiscState, DiscStatus};

pub const MANIFEST_FILE: &str = ".ripbook.json";

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn manifest_path(book_dir: &Path) -> PathBuf {
    book_dir.join(MANIFEST_FILE)
}

pub fn load_manifest(book_dir: &str) -> Result<BookManifest, String> {
    let dir = Path::new(book_dir);
    let path = manifest_path(dir);
    let data =
        fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let manifest: BookManifest =
        serde_json::from_str(&data).map_err(|e| format!("Invalid manifest: {e}"))?;
    Ok(manifest)
}

pub fn save_manifest(manifest: &BookManifest) -> Result<(), String> {
    let dir = Path::new(&manifest.book_dir);
    fs::create_dir_all(dir).map_err(|e| format!("Could not create book folder: {e}"))?;
    let path = manifest_path(dir);
    let data = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("Could not serialize manifest: {e}"))?;
    AtomicFile::new(&path, AllowOverwrite)
        .write(|file| file.write_all(&data))
        .map_err(|e| format!("Could not save manifest: {e}"))
}

pub fn reconcile(manifest: &mut BookManifest) -> bool {
    let book_dir = PathBuf::from(&manifest.book_dir);
    let mut changed = false;

    for disc in &mut manifest.discs {
        match disc.status {
            DiscStatus::Ripping => {
                disc.status = DiscStatus::Pending;
                disc.error = Some(
                    "Previous rip was interrupted. Insert this disc again and retry.".to_string(),
                );
                changed = true;
            }
            DiscStatus::Done => {
                if !disc_files_valid(&book_dir, disc) {
                    disc.status = DiscStatus::Pending;
                    disc.files.clear();
                    disc.track_count = None;
                    disc.completed_at = None;
                    disc.error = Some(
                        "Some files for this disc are missing or empty. Re-rip it.".to_string(),
                    );
                    changed = true;
                } else if disc.track_count != Some(disc.files.len() as u32) {
                    disc.track_count = Some(disc.files.len() as u32);
                    changed = true;
                }
            }
            DiscStatus::Pending | DiscStatus::Failed => {}
        }
    }

    changed
}

fn disc_files_valid(book_dir: &Path, disc: &DiscState) -> bool {
    if disc.files.is_empty() {
        return false;
    }

    disc.files.iter().all(|name| {
        let path = book_dir.join(name);
        path.is_file() && fs::metadata(&path).map(|m| m.len() > 1024).unwrap_or(false)
    })
}

pub fn create_or_open(request: &CreateBookRequest) -> Result<BookManifest, String> {
    let author = request.author.trim();
    let title = request.title.trim();

    if author.is_empty() {
        return Err("Author is required.".to_string());
    }
    if title.is_empty() {
        return Err("Book title is required.".to_string());
    }
    if request.first_disc == 0 {
        return Err("The first disc must be 1 or greater.".to_string());
    }
    if request.last_disc < request.first_disc {
        return Err("The last disc must be greater than or equal to the first disc.".to_string());
    }
    if request.last_disc > 999 {
        return Err("Disc numbers are limited to 999.".to_string());
    }

    let audio_root = PathBuf::from(request.audio_root.trim());
    if audio_root.as_os_str().is_empty() {
        return Err("Choose an audiobook library folder.".to_string());
    }

    fs::create_dir_all(&audio_root)
        .map_err(|e| format!("Could not create audiobook library: {e}"))?;

    let author_dir = audio_root.join(sanitize_component(author));
    let book_dir = author_dir.join(sanitize_component(title));
    fs::create_dir_all(&book_dir).map_err(|e| format!("Could not create book folder: {e}"))?;

    let book_dir_string = book_dir.to_string_lossy().to_string();
    let audio_root_string = audio_root.to_string_lossy().to_string();
    let path = manifest_path(&book_dir);

    let mut manifest = if path.is_file() {
        load_manifest(&book_dir_string)?
    } else {
        BookManifest::new(
            author,
            title,
            &book_dir_string,
            &audio_root_string,
            request.first_disc,
            request.last_disc,
            &request.drive,
            &request.freaccmd_path,
        )
    };

    manifest.author = author.to_string();
    manifest.title = title.to_string();
    manifest.book_dir = book_dir_string;
    manifest.audio_root = audio_root_string;
    manifest.drive = request.drive.clone();
    manifest.freaccmd_path = request.freaccmd_path.clone();
    manifest.first_disc = manifest.first_disc.min(request.first_disc);
    manifest.last_disc = manifest.last_disc.max(request.last_disc);

    for index in request.first_disc..=request.last_disc {
        if !manifest.discs.iter().any(|disc| disc.index == index) {
            manifest.discs.push(DiscState::pending(index));
        }
    }

    manifest.discs.sort_by_key(|disc| disc.index);
    import_existing_files(&mut manifest);
    reconcile(&mut manifest);
    manifest.updated_at = now_millis();
    save_manifest(&manifest)?;

    Ok(manifest)
}

pub fn set_last_disc(manifest: &mut BookManifest, last_disc: u32) -> Result<bool, String> {
    if last_disc == 0 {
        return Err("The number of discs must be at least 1.".to_string());
    }
    if last_disc > 999 {
        return Err("Disc numbers are limited to 999.".to_string());
    }
    if last_disc < manifest.first_disc {
        return Err(format!(
            "The last disc cannot be before the first disc ({}).",
            manifest.first_disc
        ));
    }

    let protected = manifest
        .discs
        .iter()
        .filter(|disc| disc.status != DiscStatus::Pending)
        .map(|disc| disc.index)
        .max()
        .unwrap_or(manifest.first_disc.saturating_sub(1));

    if last_disc < protected {
        return Err(format!(
            "Disc {protected} already has saved files or progress. Re-rip or keep that disc before reducing the book to {last_disc} discs."
        ));
    }

    let mut changed = manifest.last_disc != last_disc;

    manifest.last_disc = last_disc;
    let before = manifest.discs.len();
    manifest
        .discs
        .retain(|disc| disc.index >= manifest.first_disc && disc.index <= last_disc);

    if manifest.discs.len() != before {
        changed = true;
    }

    for index in manifest.first_disc..=last_disc {
        if !manifest.discs.iter().any(|disc| disc.index == index) {
            manifest.discs.push(DiscState::pending(index));
            changed = true;
        }
    }

    manifest.discs.sort_by_key(|disc| disc.index);

    if changed {
        import_existing_files(manifest);
        reconcile(manifest);
        manifest.updated_at = now_millis();
    }

    Ok(changed)
}

fn import_existing_files(manifest: &mut BookManifest) {
    let book_dir = PathBuf::from(&manifest.book_dir);
    let entries = match fs::read_dir(&book_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut by_disc: BTreeMap<u32, Vec<(u32, String)>> = BTreeMap::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().is_file() {
            continue;
        }
        if let Some((disc, track)) = parse_track_file(&name) {
            by_disc.entry(disc).or_default().push((track, name));
        }
    }

    for disc in &mut manifest.discs {
        if disc.status != DiscStatus::Pending {
            continue;
        }

        let Some(mut files) = by_disc.remove(&disc.index) else {
            continue;
        };

        files.sort_by_key(|(track, _)| *track);
        disc.status = DiscStatus::Done;
        disc.files = files.into_iter().map(|(_, name)| name).collect();
        disc.track_count = Some(disc.files.len() as u32);
        disc.completed_at = Some(now_millis());
        disc.error = Some(
            "Imported from existing files. Re-rip this disc if the track list looks incomplete."
                .to_string(),
        );
    }
}

fn parse_track_file(name: &str) -> Option<(u32, u32)> {
    let stem = Path::new(name)
        .file_stem()?
        .to_string_lossy()
        .to_ascii_lowercase();
    let rest = stem.strip_prefix("cd")?;
    let (disc, track_part) = rest.split_once("_t")?;
    let disc: u32 = disc.parse().ok()?;
    let track: u32 = track_part.parse().ok()?;
    Some((disc, track))
}

pub fn list_summaries(root: &str) -> Result<Vec<BookSummary>, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();

    let authors = fs::read_dir(&root).map_err(|e| format!("Could not read library: {e}"))?;
    for author_entry in authors.flatten() {
        if !author_entry.path().is_dir() {
            continue;
        }
        let books = match fs::read_dir(author_entry.path()) {
            Ok(books) => books,
            Err(_) => continue,
        };
        for book_entry in books.flatten() {
            if !book_entry.path().is_dir() {
                continue;
            }
            if !manifest_path(&book_entry.path()).is_file() {
                continue;
            }
            if let Ok(mut manifest) = load_manifest(&book_entry.path().to_string_lossy()) {
                reconcile(&mut manifest);
                summaries.push(BookSummary {
                    book_dir: book_entry.path().to_string_lossy().to_string(),
                    author: manifest.author,
                    title: manifest.title,
                    done_discs: manifest
                        .discs
                        .iter()
                        .filter(|disc| disc.status == DiscStatus::Done)
                        .count(),
                    total_discs: manifest.discs.len(),
                    updated_at: manifest.updated_at,
                });
            }
        }
    }

    summaries.sort_by_key(|summary| std::cmp::Reverse(summary.updated_at));
    Ok(summaries)
}

pub fn sanitize_component(input: &str) -> String {
    let mut value: String = input
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .take(120)
        .collect();

    value = value.trim_end_matches([' ', '.']).to_string();
    if value.is_empty() {
        return "Untitled".to_string();
    }

    let upper = value.to_ascii_uppercase();
    let base = upper.split('.').next().unwrap_or("");
    let reserved = matches!(base, "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base.as_bytes()[3].is_ascii_digit());

    if reserved {
        value.insert(0, '_');
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BookManifest;

    fn temp_book() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let book = root.path().join("Author").join("Book");
        fs::create_dir_all(&book).unwrap();
        (root, book)
    }

    #[test]
    fn sanitizes_windows_invalid_characters() {
        assert_eq!(sanitize_component("A/B: C?"), "A_B_ C_");
        assert_eq!(sanitize_component("  Book.  "), "Book");
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component("COM1.txt"), "_COM1.txt");
        assert_eq!(sanitize_component(".."), "Untitled");
    }

    #[test]
    fn parses_normalized_track_names() {
        assert_eq!(parse_track_file("cd01_t02.mp3"), Some((1, 2)));
        assert_eq!(parse_track_file("CD12_T10.flac"), Some((12, 10)));
        assert_eq!(parse_track_file("wip_cd_01"), None);
        assert_eq!(parse_track_file("rip_cd_01.log"), None);
    }

    #[test]
    fn reconcile_marks_missing_done_files_as_pending() {
        let (_root, book) = temp_book();
        let mut manifest = BookManifest::new(
            "Author",
            "Book",
            &book.to_string_lossy(),
            &book.parent().unwrap().parent().unwrap().to_string_lossy(),
            1,
            1,
            "0",
            "freaccmd.exe",
        );
        manifest.discs[0].status = DiscStatus::Done;
        manifest.discs[0].files = vec!["cd01_t01.mp3".to_string()];
        manifest.discs[0].track_count = Some(1);

        assert!(reconcile(&mut manifest));
        assert_eq!(manifest.discs[0].status, DiscStatus::Pending);
        assert!(manifest.discs[0].files.is_empty());
    }

    #[test]
    fn reconcile_keeps_valid_done_files() {
        let (_root, book) = temp_book();
        fs::write(book.join("cd01_t01.mp3"), vec![0u8; 2048]).unwrap();

        let mut manifest = BookManifest::new(
            "Author",
            "Book",
            &book.to_string_lossy(),
            &book.parent().unwrap().parent().unwrap().to_string_lossy(),
            1,
            1,
            "0",
            "freaccmd.exe",
        );
        manifest.discs[0].status = DiscStatus::Done;
        manifest.discs[0].files = vec!["cd01_t01.mp3".to_string()];
        manifest.discs[0].track_count = Some(1);

        assert!(!reconcile(&mut manifest));
        assert_eq!(manifest.discs[0].status, DiscStatus::Done);
    }

    #[test]
    fn create_book_imports_existing_batch_output() {
        let root = tempfile::tempdir().unwrap();
        let book = root.path().join("Author").join("Book");
        fs::create_dir_all(&book).unwrap();
        fs::write(book.join("cd01_t01.mp3"), vec![0u8; 2048]).unwrap();
        fs::write(book.join("cd01_t02.mp3"), vec![0u8; 2048]).unwrap();

        let manifest = create_or_open(&CreateBookRequest {
            author: "Author".to_string(),
            title: "Book".to_string(),
            first_disc: 1,
            last_disc: 2,
            audio_root: root.path().to_string_lossy().to_string(),
            drive: "0".to_string(),
            freaccmd_path: "freaccmd.exe".to_string(),
        })
        .unwrap();

        assert_eq!(manifest.discs.len(), 2);
        assert_eq!(manifest.discs[0].status, DiscStatus::Done);
        assert_eq!(manifest.discs[0].track_count, Some(2));
        assert_eq!(manifest.discs[1].status, DiscStatus::Pending);
    }

    #[test]
    fn create_book_extends_an_existing_range() {
        let root = tempfile::tempdir().unwrap();
        let request = |first_disc, last_disc| CreateBookRequest {
            author: "Author".to_string(),
            title: "Book".to_string(),
            first_disc,
            last_disc,
            audio_root: root.path().to_string_lossy().to_string(),
            drive: "0".to_string(),
            freaccmd_path: "freaccmd.exe".to_string(),
        };

        let first = create_or_open(&request(1, 2)).unwrap();
        assert_eq!(first.discs.len(), 2);

        let extended = create_or_open(&request(3, 4)).unwrap();
        assert_eq!(extended.discs.len(), 4);
        assert_eq!(extended.first_disc, 1);
        assert_eq!(extended.last_disc, 4);
        assert!(
            extended
                .discs
                .iter()
                .all(|disc| disc.status == DiscStatus::Pending)
        );
    }

    #[test]
    fn manifest_round_trips() {
        let (_root, book) = temp_book();
        let manifest = BookManifest::new(
            "Author",
            "Book",
            &book.to_string_lossy(),
            &book.parent().unwrap().parent().unwrap().to_string_lossy(),
            1,
            3,
            "0",
            "freaccmd.exe",
        );

        save_manifest(&manifest).unwrap();
        let loaded = load_manifest(&book.to_string_lossy()).unwrap();
        assert_eq!(loaded.title, "Book");
        assert_eq!(loaded.discs.len(), 3);
        assert!(
            loaded
                .discs
                .iter()
                .all(|disc| disc.status == DiscStatus::Pending)
        );
        assert!(manifest.discs[0].completed_at.is_none());
    }

    #[test]
    fn set_last_disc_adds_pending_discs() {
        let (_root, book) = temp_book();
        let mut manifest = BookManifest::new(
            "Author",
            "Book",
            &book.to_string_lossy(),
            &book.parent().unwrap().parent().unwrap().to_string_lossy(),
            1,
            1,
            "0",
            "freaccmd.exe",
        );

        assert!(set_last_disc(&mut manifest, 3).unwrap());
        assert_eq!(manifest.last_disc, 3);
        assert_eq!(manifest.discs.len(), 3);
        assert!(
            manifest
                .discs
                .iter()
                .all(|disc| disc.status == DiscStatus::Pending)
        );
    }

    #[test]
    fn set_last_disc_removes_only_pending_discs() {
        let (_root, book) = temp_book();
        let mut manifest = BookManifest::new(
            "Author",
            "Book",
            &book.to_string_lossy(),
            &book.parent().unwrap().parent().unwrap().to_string_lossy(),
            1,
            4,
            "0",
            "freaccmd.exe",
        );
        manifest.discs[0].status = DiscStatus::Done;
        manifest.discs[0].files = vec!["cd01_t01.mp3".to_string()];
        manifest.discs[1].status = DiscStatus::Done;
        manifest.discs[1].files = vec!["cd02_t01.mp3".to_string()];
        fs::write(book.join("cd01_t01.mp3"), vec![0u8; 2048]).unwrap();
        fs::write(book.join("cd02_t01.mp3"), vec![0u8; 2048]).unwrap();

        assert!(set_last_disc(&mut manifest, 2).unwrap());
        assert_eq!(manifest.discs.len(), 2);
        assert_eq!(manifest.discs[0].status, DiscStatus::Done);

        let error = set_last_disc(&mut manifest, 1).unwrap_err();
        assert!(error.contains("already has saved files"));
    }
}
