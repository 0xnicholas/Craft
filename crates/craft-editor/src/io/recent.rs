use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecentProjects {
    pub version: u32,
    pub entries: Vec<RecentEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecentEntry {
    pub root: PathBuf,
    pub last_opened: SystemTime,
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}

impl RecentProjects {
    pub fn add_or_bump(&mut self, root: &Path) {
        let now = SystemTime::now();
        let already = self.entries.iter_mut().find(|e| e.root == root);
        match already {
            Some(e) => e.last_opened = now,
            None => self.entries.push(RecentEntry {
                root: root.to_path_buf(),
                last_opened: now,
            }),
        }
        self.entries
            .sort_by_key(|e| std::cmp::Reverse(e.last_opened));
        self.entries.truncate(10);
    }
}

pub fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("ai", "craft", "editor").map(|p| p.config_dir().to_path_buf())
}

pub fn load() -> RecentProjects {
    let Some(dir) = config_dir() else {
        return RecentProjects::default();
    };
    let path = dir.join("recent.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<RecentProjects>(&s).ok())
        .unwrap_or_default()
}

pub fn save(recent: &RecentProjects) -> std::io::Result<()> {
    let Some(dir) = config_dir() else {
        return Ok(());
    };
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("recent.json");
    let json = serde_json::to_string_pretty(recent)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_new_entry_then_dedupe_and_cap() {
        let mut r = RecentProjects::default();
        for i in 0..15 {
            r.add_or_bump(Path::new(&format!("/p/{i}")));
        }
        assert_eq!(r.entries.len(), 10);
        assert_eq!(r.entries[0].root, PathBuf::from("/p/14"));
        assert_eq!(r.entries[9].root, PathBuf::from("/p/5"));
    }

    #[test]
    fn add_existing_bumps_to_top() {
        let mut r = RecentProjects::default();
        r.add_or_bump(Path::new("/p/a"));
        r.add_or_bump(Path::new("/p/b"));
        r.add_or_bump(Path::new("/p/a"));
        assert_eq!(r.entries[0].root, PathBuf::from("/p/a"));
        assert_eq!(r.entries.len(), 2);
    }

    #[test]
    fn save_load_roundtrip() {
        let mut r = RecentProjects::default();
        r.add_or_bump(Path::new("/Users/x/projects/foo"));
        let json = serde_json::to_string(&r).unwrap();
        let loaded: RecentProjects = serde_json::from_str(&json).unwrap();
        assert_eq!(
            loaded.entries[0].root,
            PathBuf::from("/Users/x/projects/foo")
        );
    }
}
