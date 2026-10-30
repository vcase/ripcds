use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::{AppHandle, Manager};

use crate::models::FreacInfo;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    #[cfg(not(windows))]
    let _ = command;
}

pub fn candidate_paths(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        for base in [
            resource_dir.join("freac"),
            resource_dir.join("resources").join("freac"),
        ] {
            candidates.push(base.join("freaccmd.exe"));
            candidates.push(base.join("freaccmd"));
        }
    }

    if let Ok(data_dir) = app.path().app_data_dir() {
        let mut downloads = fs::read_dir(data_dir.join("freac"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path().join("freaccmd.exe"))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();

        downloads.sort();
        downloads.reverse();
        candidates.extend(downloads);
    }

    #[cfg(windows)]
    {
        if let Ok(program_files) = env::var("ProgramFiles") {
            candidates.push(
                PathBuf::from(program_files)
                    .join("freac")
                    .join("freaccmd.exe"),
            );
        }
        if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
            candidates.push(
                PathBuf::from(program_files_x86)
                    .join("freac")
                    .join("freaccmd.exe"),
            );
        }
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("freac")
                    .join("freaccmd.exe"),
            );
        }
        candidates.push(PathBuf::from(r"C:\Program Files\freac\freaccmd.exe"));
        candidates.push(PathBuf::from(r"C:\Program Files (x86)\freac\freaccmd.exe"));
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/freac.app/Contents/MacOS/freaccmd",
        ));
        candidates.push(PathBuf::from(
            "/Applications/freac.app/Contents/Resources/freaccmd",
        ));
    }

    if let Some(path_var) = env::var_os("PATH") {
        for directory in env::split_paths(&path_var) {
            candidates.push(directory.join("freaccmd.exe"));
            candidates.push(directory.join("freaccmd"));
        }
    }

    candidates
}

pub fn find_freaccmd(app: &AppHandle) -> Option<PathBuf> {
    candidate_paths(app).into_iter().find(|path| path.is_file())
}

pub fn default_freaccmd_path(app: &AppHandle) -> String {
    if let Some(path) = find_freaccmd(app) {
        return path.to_string_lossy().to_string();
    }

    #[cfg(windows)]
    {
        return r"C:\Program Files\freac\freaccmd.exe".to_string();
    }

    #[cfg(target_os = "macos")]
    {
        return "/Applications/freac.app/Contents/MacOS/freaccmd".to_string();
    }

    #[allow(unreachable_code)]
    "freaccmd.exe".to_string()
}

pub fn probe_freaccmd(path: &str) -> FreacInfo {
    let path_buf = PathBuf::from(path);
    if !path_buf.is_file() {
        return FreacInfo {
            path: path.to_string(),
            exists: false,
            version: "unknown".to_string(),
            supports_timeout: false,
            message: "freaccmd.exe was not found at this path.".to_string(),
        };
    }

    let mut command = Command::new(&path_buf);
    if let Some(parent) = path_buf.parent() {
        command.current_dir(parent);
    }
    hide_console(&mut command);

    match command.output() {
        Ok(output) => {
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            let version = if text.contains("--timeout") || text.contains("--track=") {
                "1.1+"
            } else if text.contains("-track") {
                "1.0"
            } else {
                "unknown"
            };

            FreacInfo {
                path: path.to_string(),
                exists: true,
                version: version.to_string(),
                supports_timeout: version == "1.1+",
                message: match version {
                    "1.1+" => "Detected the fre:ac 1.1+ command line syntax.".to_string(),
                    "1.0" => "Detected the fre:ac 1.0 command line syntax.".to_string(),
                    _ => "Found freaccmd, but could not determine its CLI version. Assuming 1.1+."
                        .to_string(),
                },
            }
        }
        Err(error) => FreacInfo {
            path: path.to_string(),
            exists: true,
            version: "unknown".to_string(),
            supports_timeout: false,
            message: format!("Found freaccmd but could not run it: {error}"),
        },
    }
}

pub fn build_rip_args(version: &str, drive: &str, wip_dir: &Path) -> Vec<String> {
    let wip_dir = wip_dir.to_string_lossy().to_string();
    let drive = if drive.trim().is_empty() {
        "0"
    } else {
        drive.trim()
    };

    if version == "1.0" {
        vec![
            "-track".to_string(),
            "all".to_string(),
            "-t".to_string(),
            "600".to_string(),
            "-p".to_string(),
            "<track>".to_string(),
            "-e".to_string(),
            "LAME".to_string(),
            "-cd".to_string(),
            drive.to_string(),
            "-d".to_string(),
            wip_dir,
        ]
    } else {
        vec![
            "--track=all".to_string(),
            "--timeout=600".to_string(),
            "--pattern=<track>".to_string(),
            "--encoder=LAME".to_string(),
            format!("--drive={drive}"),
            "-d".to_string(),
            wip_dir,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_modern_args() {
        let args = build_rip_args("1.1+", "0", Path::new(r"C:\books\book\wip_cd_01"));
        assert!(args.contains(&"--track=all".to_string()));
        assert!(args.contains(&"--timeout=600".to_string()));
        assert!(args.contains(&"--pattern=<track>".to_string()));
        assert!(args.contains(&"--drive=0".to_string()));
        assert!(!args.contains(&"-t".to_string()));
    }

    #[test]
    fn builds_legacy_args() {
        let args = build_rip_args("1.0", "1", Path::new(r"C:\books\book\wip_cd_01"));
        assert!(args.contains(&"-track".to_string()));
        assert!(args.contains(&"-t".to_string()));
        assert!(args.contains(&"600".to_string()));
        assert!(args.contains(&"-cd".to_string()));
        assert!(args.contains(&"1".to_string()));
        assert!(!args.iter().any(|arg| arg == "--timeout=600"));
    }
}
