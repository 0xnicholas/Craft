use std::path::Path;

pub fn regenerate_if_needed(workspace_root: &Path) -> std::io::Result<bool> {
    let target_dir = workspace_root.join(".craft");
    let target = target_dir.join("engine_types.lua");
    let stub = craft_schema::lua_engine_stub();

    if target.exists() {
        if let Ok(existing) = std::fs::read_to_string(&target) {
            let header = stub.lines().next().unwrap_or("");
            if existing.starts_with(header) {
                return Ok(false);
            }
        }
    }

    std::fs::create_dir_all(&target_dir)?;
    std::fs::write(&target, &stub)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn writes_stub_on_first_run() {
        let dir = TempDir::new().unwrap();
        assert!(regenerate_if_needed(dir.path()).unwrap());
        assert!(dir.path().join(".craft/engine_types.lua").exists());
    }

    #[test]
    fn does_not_overwrite_when_version_unchanged() {
        let dir = TempDir::new().unwrap();
        regenerate_if_needed(dir.path()).unwrap();
        assert!(!regenerate_if_needed(dir.path()).unwrap());
    }

    #[test]
    fn overwrites_when_version_changes() {
        let dir = TempDir::new().unwrap();
        regenerate_if_needed(dir.path()).unwrap();
        let target = dir.path().join(".craft/engine_types.lua");
        let original = std::fs::read_to_string(&target).unwrap();
        std::fs::write(&target, format!("-- schema-version: OLD\n{original}")).unwrap();
        assert!(regenerate_if_needed(dir.path()).unwrap());
        let after = std::fs::read_to_string(&target).unwrap();
        assert!(after.contains(craft_schema::SCHEMA_VERSION));
    }
}
