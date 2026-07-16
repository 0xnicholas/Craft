use std::path::Path;

use craft_kernel::Project;

use crate::state::EditorError;

pub fn open(root: &Path) -> Result<Project, EditorError> {
    let manifest = root.join("craft.toml");
    let content = std::fs::read_to_string(&manifest)?;
    let project: Project = toml::from_str(&content)?;
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn opens_craft_toml_fixture() {
        let dir = TempDir::new().unwrap();
        let manifest = dir.path().join("craft.toml");
        std::fs::write(
            &manifest,
            r#"
[project]
name = "test"
"#,
        )
        .unwrap();
        let project = open(dir.path()).unwrap();
        assert_eq!(project.project.name, "test");
    }

    #[test]
    fn missing_manifest_returns_io_error() {
        let dir = TempDir::new().unwrap();
        let result = open(dir.path());
        assert!(result.is_err());
    }
}
