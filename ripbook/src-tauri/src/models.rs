use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscStatus {
    Pending,
    Ripping,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscState {
    pub index: u32,
    pub status: DiscStatus,
    #[serde(default)]
    pub track_count: Option<u32>,
    #[serde(default)]
    pub expected_track_count: Option<u32>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub completed_at: Option<u64>,
}

impl DiscState {
    pub fn pending(index: u32) -> Self {
        Self {
            index,
            status: DiscStatus::Pending,
            track_count: None,
            expected_track_count: None,
            files: Vec::new(),
            error: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookManifest {
    pub schema_version: u32,
    pub author: String,
    pub title: String,
    pub book_dir: String,
    pub audio_root: String,
    pub first_disc: u32,
    pub last_disc: u32,
    pub drive: String,
    pub freaccmd_path: String,
    pub discs: Vec<DiscState>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl BookManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        author: &str,
        title: &str,
        book_dir: &str,
        audio_root: &str,
        first_disc: u32,
        last_disc: u32,
        drive: &str,
        freaccmd_path: &str,
    ) -> Self {
        let now = crate::manifest::now_millis();
        Self {
            schema_version: 1,
            author: author.to_string(),
            title: title.to_string(),
            book_dir: book_dir.to_string(),
            audio_root: audio_root.to_string(),
            first_disc,
            last_disc,
            drive: drive.to_string(),
            freaccmd_path: freaccmd_path.to_string(),
            discs: (first_disc..=last_disc).map(DiscState::pending).collect(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookRequest {
    pub author: String,
    pub title: String,
    pub first_disc: u32,
    pub last_disc: u32,
    pub audio_root: String,
    pub drive: String,
    pub freaccmd_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RipRequest {
    pub book_dir: String,
    pub disc_index: u32,
    pub drive: String,
    pub freaccmd_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSummary {
    pub book_dir: String,
    pub author: String,
    pub title: String,
    pub done_discs: usize,
    pub total_discs: usize,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub index: u32,
    pub letter: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub platform: String,
    pub default_audio_root: String,
    pub default_freaccmd_path: String,
    pub default_drive: String,
    pub freaccmd_found: bool,
    pub drives: Vec<DriveInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreacInfo {
    pub path: String,
    pub exists: bool,
    pub version: String,
    pub supports_timeout: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPayload {
    pub disc: u32,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatePayload {
    pub disc: u32,
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscInfo {
    pub drive: String,
    pub track_count: u32,
    pub first_track: u32,
    pub last_track: u32,
    pub disc_id: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub disc_number: Option<u32>,
    pub disc_count: Option<u32>,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreacInstallResult {
    pub version: String,
    pub path: String,
    pub message: String,
}
