use std::path::{Path, PathBuf};

use serde::Deserialize;

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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CraftTomlLua {
    #[serde(default)]
    pub modules_dir: Option<PathBuf>,
}

pub fn read_lua_section(root: &Path) -> CraftTomlLua {
    let manifest = root.join("craft.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return CraftTomlLua::default();
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return CraftTomlLua::default();
    };
    let Some(lua_value) = table.get("lua").cloned() else {
        return CraftTomlLua::default();
    };
    CraftTomlLua::deserialize(toml::de::ValueDeserializer::new(&lua_value.to_string()))
        .unwrap_or_default()
}

#[cfg(test)]
mod lua_section_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn returns_default_when_no_manifest() {
        let dir = TempDir::new().unwrap();
        let section = read_lua_section(dir.path());
        assert!(section.modules_dir.is_none());
    }

    #[test]
    fn returns_default_when_lua_section_missing() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("craft.toml"),
            "[project]\nname = \"x\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let section = read_lua_section(dir.path());
        assert!(section.modules_dir.is_none());
    }

    #[test]
    fn parses_lua_modules_dir() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("craft.toml"),
            "[project]\nname = \"x\"\nversion = \"0.1.0\"\nkind = \"game\"\n\n[lua]\nmodules_dir = \"scripts\"\n",
        )
        .unwrap();
        let section = read_lua_section(dir.path());
        assert_eq!(section.modules_dir, Some(PathBuf::from("scripts")));
    }

    #[test]
    fn returns_default_on_invalid_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("craft.toml"),
            "this is = not valid = toml =",
        )
        .unwrap();
        let section = read_lua_section(dir.path());
        assert!(section.modules_dir.is_none());
    }
}
