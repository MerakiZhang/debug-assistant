use anyhow::bail;
use chrono::Local;
use std::path::PathBuf;

pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    if text.trim().is_empty() {
        bail!("No log to copy");
    }

    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}

pub fn save_log(kind: &str, text: &str) -> anyhow::Result<PathBuf> {
    if text.trim().is_empty() {
        bail!("No log to save");
    }

    std::fs::create_dir_all("logs")?;
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let path = PathBuf::from(format!("logs/{}-{}.log", kind, timestamp));
    std::fs::write(&path, text)?;
    Ok(path)
}
