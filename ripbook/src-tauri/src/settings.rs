use std::fs;
use std::io::Write;
use std::path::PathBuf;

use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub audio_root: Option<String>,
    #[serde(default)]
    pub audio_roots: Vec<String>,
    #[serde(default)]
    pub freaccmd_path: Option<String>,
    #[serde(default)]
    pub drive: Option<String>,
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|directory| directory.join("settings.json"))
}

pub fn load(app: &AppHandle) -> AppSettings {
    let Some(path) = settings_path(app) else {
        return AppSettings::default();
    };
    let Ok(data) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let Some(path) = settings_path(app) else {
        return Ok(());
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create the settings folder: {error}"))?;
    }

    let data = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("Could not serialize settings: {error}"))?;

    AtomicFile::new(&path, AllowOverwrite)
        .write(|file| file.write_all(&data))
        .map_err(|error| format!("Could not save settings: {error}"))
}

pub fn remember_book(app: &AppHandle, manifest: &crate::models::BookManifest) {
    let mut settings = load(app);

    settings.audio_root = Some(manifest.audio_root.clone());
    if !manifest.audio_root.trim().is_empty()
        && !settings
            .audio_roots
            .iter()
            .any(|root| root.eq_ignore_ascii_case(&manifest.audio_root))
    {
        settings.audio_roots.push(manifest.audio_root.clone());
    }
    settings.freaccmd_path = Some(manifest.freaccmd_path.clone());
    settings.drive = Some(manifest.drive.clone());

    let _ = save(app, &settings);
}
