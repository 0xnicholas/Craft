use std::path::Path;

use craft_kernel::{NodeRegistry, Scene};

use crate::state::EditorError;

pub fn load_scene(path: &Path, registry: &NodeRegistry) -> Result<Scene, EditorError> {
    let content = std::fs::read_to_string(path)?;
    let path_str = path.to_string_lossy();
    Scene::parse(&content, &path_str, registry).map_err(|e| EditorError::SceneParse(e.to_string()))
}

pub fn save_scene(path: &Path, scene: &Scene) -> Result<(), EditorError> {
    let value = scene.to_value();
    let json = serde_json::to_string_pretty(&value)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_save_roundtrip_preserves_content() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("scene.json");
        let original = r#"{ "kind": "scene", "name": "test", "nodes": [] }"#;
        std::fs::write(&path, original).unwrap();

        let registry = NodeRegistry::new();
        let loaded = load_scene(&path, &registry).unwrap();
        save_scene(&path, &loaded).unwrap();

        let reloaded = load_scene(&path, &registry).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&reloaded).unwrap()
        );
    }
}
