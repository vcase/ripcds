use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static SCRIPT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run an embedded PowerShell script and return its trimmed standard output.
///
/// The script is written to a uniquely named temporary file so that scripts
/// containing braces, quotes, or here-strings do not need shell escaping.
pub async fn run_script(script: &str, args: &[(&str, &str)]) -> Result<String, String> {
    let path = std::env::temp_dir().join(format!(
        "ripbook-{}-{}.ps1",
        std::process::id(),
        SCRIPT_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    fs::write(&path, script)
        .map_err(|error| format!("Could not write the PowerShell helper: {error}"))?;

    let mut command = tokio::process::Command::new("powershell");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    for (name, value) in args {
        command.arg(format!("-{name}")).arg(value);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().await;
    let _ = fs::remove_file(&path);

    let output = output.map_err(|error| format!("Could not run PowerShell: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "PowerShell exited with an error.".to_string()
        };

        return Err(detail);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
