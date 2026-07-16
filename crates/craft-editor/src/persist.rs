use std::path::PathBuf;

use egui_dock::DockState;
use serde::{Deserialize, Serialize};

use crate::io::recent;

const TAB_KEYS: &[&str] = &[
    "Scene Tree",
    "Inspector",
    "Files",
    "Terminal Preview",
    "Behavior Editor",
    "Lua Editor",
    "Agent Copilot",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedDock {
    pub tab_titles: Vec<String>,
}

pub fn dock_file() -> Option<PathBuf> {
    recent::config_dir().map(|d| d.join("dock.bin"))
}

fn default_tab_titles() -> Vec<String> {
    TAB_KEYS.iter().map(|s| (*s).to_string()).collect()
}

pub fn build_default_dock() -> DockState<String> {
    DockState::new(default_tab_titles())
}

pub fn load_dock() -> Option<DockState<String>> {
    let path = dock_file()?;
    let bytes = std::fs::read(&path).ok()?;
    let persisted: PersistedDock = bincode::deserialize(&bytes).ok()?;
    if persisted.tab_titles.is_empty() {
        return Some(build_default_dock());
    }
    Some(DockState::new(persisted.tab_titles))
}

pub fn save_dock(dock: &DockState<String>) -> std::io::Result<()> {
    let path = dock_file().ok_or_else(|| std::io::Error::other("no config dir"))?;
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("dock file has no parent dir"))?;
    std::fs::create_dir_all(parent)?;

    let mut titles: Vec<String> = main_surface_tab_titles(dock);
    if titles.is_empty() {
        titles = default_tab_titles();
    }
    let persisted = PersistedDock { tab_titles: titles };
    let bytes = bincode::serialize(&persisted)
        .map_err(|e| std::io::Error::other(format!("bincode serialize: {e}")))?;
    std::fs::write(&path, bytes)
}

fn main_surface_tab_titles(dock: &DockState<String>) -> Vec<String> {
    dock.main_surface().tabs().map(|t| (*t).clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dock_has_all_seven_tabs() {
        let dock = build_default_dock();
        let titles: Vec<_> = dock.main_surface().tabs().map(|t| (*t).clone()).collect();
        assert_eq!(titles.len(), TAB_KEYS.len());
        for key in TAB_KEYS {
            assert!(titles.iter().any(|t| t == *key), "missing tab: {key}");
        }
    }

    #[test]
    fn round_trip_survives_set_of_titles() {
        let original = DockState::new(vec!["A".into(), "B".into(), "C".into()]);
        let titles: Vec<String> = original
            .main_surface()
            .tabs()
            .map(|t: &String| (*t).clone())
            .collect();
        let persisted = PersistedDock {
            tab_titles: titles.clone(),
        };
        let bytes = bincode::serialize(&persisted).expect("serialize");
        let restored: PersistedDock = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(restored.tab_titles, titles);
    }
}
