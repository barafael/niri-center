//! Auto-backups taken before every save.
//!
//! Inspired by NiriMod's `backup.py` (same idea, clean-room implementation):
//! before any write lands on disk, the
//! pristine on-disk content is copied into the state directory with a timestamped
//! name, and stale backups are pruned. This is the safety net *behind* the
//! history, which undoes changes; backups survive a history that was cleared.

use std::path::{Path, PathBuf};

/// Write `content` into the backup directory with a millisecond-timestamped
/// name. Returns the backup path.
pub fn write_backup(dir: &Path, file_name: &str, content: &str) -> std::io::Result<PathBuf> {
    let nanos = crate::clock::now_nanos();
    let safe = file_name.replace('/', "_");
    let path = dir.join(format!("{nanos}_{safe}"));
    std::fs::create_dir_all(dir)?;
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Keep only the newest `keep` backups in `dir` (timestamp prefix sorts them).
pub fn prune(dir: &Path, keep: usize) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    entries.sort();
    let excess = entries.len().saturating_sub(keep);
    // one backup that cannot go does not keep the newer excess ones
    let mut first_failure = None;
    for old in entries.iter().take(excess) {
        if let Err(failure) = std::fs::remove_file(old) {
            first_failure.get_or_insert(failure);
        }
    }
    first_failure.map_or(Ok(()), Err)
}

/// One stored backup: provenance + location.
#[derive(Debug)]
pub struct Backup {
    pub path: PathBuf,
    /// Config file this backup belongs to (`config.kdl`, `keybinds.kdl`, …).
    pub display_name: String,
    pub timestamp_nanos: u128,
}

/// List backups, newest last.
pub fn list_backups(dir: &Path) -> Vec<Backup> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            let (nano, file) = name.split_once('_')?;
            let timestamp_nanos = nano.parse().unwrap_or(0);
            let display_name = file.to_string();
            Some(Backup {
                path,
                display_name,
                timestamp_nanos,
            })
        })
        .collect()
}

/// Newest backup content for `file_name`, if any.
pub fn restore_latest(dir: &Path, file_name: &str) -> Option<String> {
    let mut candidates: Vec<Backup> = list_backups(dir)
        .into_iter()
        .filter(|backup| backup.display_name == file_name)
        .collect();
    candidates.sort_by_key(|backup| backup.timestamp_nanos);
    candidates
        .last()
        .and_then(|backup| std::fs::read_to_string(&backup.path).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_prunes_backups() {
        let dir = tempfile::TempDir::new().unwrap();
        let folder = dir.path();
        write_backup(folder, "config.kdl", "a").unwrap();
        write_backup(folder, "config.kdl", "b").unwrap();
        write_backup(folder, "config.kdl", "c").unwrap();
        prune(folder, 2).unwrap();
        let remaining: Vec<PathBuf> = std::fs::read_dir(folder)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .collect();
        assert_eq!(remaining.len(), 2, "prune keeps the newest two");
        // the two newest contents survive (c, then b)
        let mut texts: Vec<String> = remaining
            .iter()
            .map(|path| std::fs::read_to_string(path).unwrap())
            .collect();
        texts.sort();
        assert_eq!(texts, vec!["b".to_string(), "c".to_string()]);
    }

    #[test]
    fn writes_respect_subdirectories() {
        let folder = tempfile::TempDir::new().unwrap().path().join("a/b");
        let path = write_backup(&folder, "keybinds.kdl", "x").unwrap();
        assert!(path.is_file());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }

    #[test]
    fn lists_and_restores_newest() {
        let folder = tempfile::TempDir::new().unwrap().path().join("backups");
        write_backup(&folder, "config.kdl", "old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        write_backup(&folder, "config.kdl", "new").unwrap();
        write_backup(&folder, "keybinds.kdl", "binds").unwrap();

        let mut names: Vec<String> = list_backups(&folder)
            .into_iter()
            .map(|backup| backup.display_name)
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "config.kdl".to_string(),
                "config.kdl".to_string(),
                "keybinds.kdl".to_string()
            ]
        );

        assert_eq!(
            restore_latest(&folder, "config.kdl").as_deref(),
            Some("new")
        );
        assert_eq!(
            restore_latest(&folder, "keybinds.kdl").as_deref(),
            Some("binds")
        );
        assert_eq!(restore_latest(&folder, "unknown.kdl"), None);
    }
}
