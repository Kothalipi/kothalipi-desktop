//! Kothalipi: keep the app's data private to the signed-in user.
//!
//! The settings file holds the user's API keys and the recordings folder holds
//! their voice, so on a shared Mac or Linux box no other account should be able
//! to open either. The data folder is made owner-only (0700) and the files
//! inside owner-only (0600). Windows already keeps the profile folder private.

use tauri::{AppHandle, Manager};

#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)) {
        log::warn!("Could not restrict {}: {}", path.display(), e);
    }
}

#[cfg(unix)]
fn lock_down(dir: &std::path::Path, depth: u32) {
    set_mode(dir, 0o700);
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(t) if t.is_dir() => lock_down(&path, depth - 1),
            Ok(t) if t.is_file() => set_mode(&path, 0o600),
            _ => {} // never follow symlinks
        }
    }
}

/// Restrict the app data folder and what is in it. Cheap; run at startup.
pub fn harden(app: &AppHandle) {
    #[cfg(unix)]
    if let Ok(dir) = app.path().app_data_dir() {
        if dir.is_dir() {
            lock_down(&dir, 2);
        }
    }
    #[cfg(not(unix))]
    let _ = app;
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn data_folder_and_files_become_owner_only() {
        let dir = std::env::temp_dir().join(format!("kothalipi-sec-{}", std::process::id()));
        let rec = dir.join("recordings");
        std::fs::create_dir_all(&rec).unwrap();
        let f = dir.join("settings_store.json");
        let w = rec.join("a.wav");
        std::fs::write(&f, "{}").unwrap();
        std::fs::write(&w, "x").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        lock_down(&dir, 2);
        let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&rec), 0o700);
        assert_eq!(mode(&f), 0o600);
        assert_eq!(mode(&w), 0o600);
        std::fs::remove_dir_all(&dir).ok();
    }
}
