use std::path::{Path, PathBuf};

fn backup_dir_of(deck_file: &Path) -> PathBuf {
    let base = deck_file
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(".keynote-backups")
}

fn stamp() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs().to_string(),
        Err(_) => "0".into(),
    }
}

/// Save a timestamped `.bak` copy beside the deck. Best-effort; returns backup path.
pub fn backup_file(deck_file: &Path) -> Option<PathBuf> {
    let content = std::fs::read(deck_file).ok()?;
    let dir = backup_dir_of(deck_file);
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let stem = deck_file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "deck".into());
    let dest = dir.join(format!("{stem}-{}.bak", stamp()));
    std::fs::write(&dest, content).ok()?;
    // Recovery snapshot mirror (crash safety net, Markdown + boundaries only).
    mirror_recovery(deck_file);
    Some(dest)
}

fn recovery_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(xdg).join("keynote/recovery")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/state/keynote/recovery")
    } else {
        std::env::temp_dir().join("keynote-recovery")
    }
}

fn mirror_recovery(deck_file: &Path) {
    let dir = recovery_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(content) = std::fs::read(deck_file) {
        let name = deck_file
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "deck.md".into());
        let dest = dir.join(format!("{name}.{}.snap", stamp()));
        let _ = std::fs::write(dest, content);
    }
}

pub fn list_backups(deck_file: &Path) -> Vec<PathBuf> {
    let dir = backup_dir_of(deck_file);
    let mut out = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|x| x.path()))
                .filter(|p| p.extension().map(|e| e == "bak").unwrap_or(false))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.sort();
    out
}
