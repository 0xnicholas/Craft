use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::{EngineError, EngineResult, IoError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u32);

impl ResourceId {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub id: ResourceId,
    pub uri: String,
    pub data: Value,
    pub source: PathBuf,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceRef {
    pub uri: String,
    pub snapshot_version: u32,
}

impl ResourceRef {
    pub fn new(uri: impl Into<String>, snapshot_version: u32) -> Self {
        Self {
            uri: uri.into(),
            snapshot_version,
        }
    }
}

#[derive(Debug, Default)]
pub struct ResourceRegistry {
    next_id: u32,
    by_id: HashMap<ResourceId, Resource>,
    by_uri: HashMap<String, ResourceId>,
    base_dirs: Vec<PathBuf>,
    next_version: u32,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let mut r = Self::default();
        r.base_dirs.push(base_dir.into());
        r
    }

    pub fn add_base_dir(&mut self, dir: impl Into<PathBuf>) {
        self.base_dirs.push(dir.into());
    }

    pub fn register(&mut self, uri: impl Into<String>, data: Value) -> ResourceId {
        let uri = uri.into();
        if let Some(&id) = self.by_uri.get(&uri) {
            return id;
        }
        let id = ResourceId(self.next_id);
        self.next_id += 1;
        let version = self.next_version;
        self.next_version += 1;
        let resource = Resource {
            id,
            uri: uri.clone(),
            data,
            source: PathBuf::new(),
            version,
        };
        self.by_id.insert(id, resource);
        self.by_uri.insert(uri, id);
        id
    }

    pub fn load(&mut self, uri: &str) -> EngineResult<ResourceId> {
        if let Some(&id) = self.by_uri.get(uri) {
            return Ok(id);
        }
        let path = self.resolve_uri(uri)?;
        let contents = fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EngineError::Io(IoError::not_found(uri.to_string(), e.to_string()))
            } else {
                EngineError::Io(IoError::read(uri.to_string(), e.to_string()))
            }
        })?;
        let data: Value = serde_json::from_str(&contents).map_err(|e| {
            EngineError::Io(IoError::read(uri.to_string(), format!("invalid JSON: {e}")))
        })?;
        let id = ResourceId(self.next_id);
        self.next_id += 1;
        let version = self.next_version;
        self.next_version += 1;
        let resource = Resource {
            id,
            uri: uri.to_string(),
            data,
            source: path,
            version,
        };
        self.by_id.insert(id, resource);
        self.by_uri.insert(uri.to_string(), id);
        Ok(id)
    }

    pub fn resolve_uri(&self, uri: &str) -> EngineResult<PathBuf> {
        let stripped = uri.strip_prefix("res://").ok_or_else(|| {
            EngineError::Io(IoError::not_found(
                uri.to_string(),
                format!("URI must start with \"res://\", got {uri}"),
            ))
        })?;
        if stripped.is_empty() {
            return Err(EngineError::Io(IoError::not_found(
                uri.to_string(),
                "URI has no path after res://",
            )));
        }
        for base in &self.base_dirs {
            let candidate = base.join(stripped);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(EngineError::Io(IoError::not_found(
            uri.to_string(),
            "not found in any registered res:// base directory",
        )))
    }

    pub fn get(&self, id: ResourceId) -> Option<&Resource> {
        self.by_id.get(&id)
    }

    pub fn resolve(&self, uri: &str) -> Option<ResourceId> {
        self.by_uri.get(uri).copied()
    }

    pub fn version(&self, id: ResourceId) -> Option<u32> {
        self.by_id.get(&id).map(|r| r.version)
    }

    pub fn resolve_ref(&self, uri: &str) -> Option<ResourceRef> {
        self.by_uri.get(uri).and_then(|id| {
            self.by_id.get(id).map(|r| ResourceRef {
                uri: r.uri.clone(),
                snapshot_version: r.version,
            })
        })
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn register_assigns_unique_ids() {
        let mut r = ResourceRegistry::new();
        let a = r.register("res://sprites/player.json", json!({"hp": 100}));
        let b = r.register("res://sprites/enemy.json", json!({"hp": 50}));
        assert_ne!(a, b);
    }

    #[test]
    fn register_is_idempotent_on_uri() {
        let mut r = ResourceRegistry::new();
        let a = r.register("res://x.json", json!(1));
        let b = r.register("res://x.json", json!(2));
        assert_eq!(a, b);
        assert_eq!(r.get(a).unwrap().data, json!(1));
    }

    #[test]
    fn resolve_uri_returns_id_after_register() {
        let mut r = ResourceRegistry::new();
        let id = r.register("res://data/foo.json", json!({"k": "v"}));
        assert_eq!(r.resolve("res://data/foo.json"), Some(id));
        assert_eq!(r.resolve("res://data/missing.json"), None);
    }

    #[test]
    fn load_reads_file_from_base_dir() {
        let dir = tempdir();
        let path = dir.join("hp.json");
        std::fs::write(&path, r#"{"hp":100}"#).unwrap();
        let mut r = ResourceRegistry::with_base_dir(dir.clone());
        let id = r.load("res://hp.json").expect("load");
        let resource = r.get(id).expect("resource");
        assert_eq!(resource.data, json!({"hp": 100}));
        assert_eq!(resource.source, path);
    }

    #[test]
    fn load_is_idempotent() {
        let dir = tempdir();
        std::fs::write(dir.join("x.json"), r#"{"v":1}"#).unwrap();
        let mut r = ResourceRegistry::with_base_dir(dir.clone());
        let a = r.load("res://x.json").expect("load");
        let b = r.load("res://x.json").expect("load");
        assert_eq!(a, b);
    }

    #[test]
    fn load_rejects_unknown_uri() {
        let dir = tempdir();
        let mut r = ResourceRegistry::with_base_dir(dir);
        let err = r.load("res://missing.json").expect_err("must fail");
        let EngineError::Io(io) = err else {
            panic!("expected Io error");
        };
        assert_eq!(io.kind, crate::error::IoErrorKind::NotFound);
    }

    #[test]
    fn load_rejects_non_res_uri() {
        let mut r = ResourceRegistry::new();
        let err = r.load("/etc/passwd").expect_err("must fail");
        assert!(matches!(err, EngineError::Io(_)));
    }

    #[test]
    fn resolve_uri_finds_existing_file() {
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("sprites")).unwrap();
        std::fs::write(dir.join("sprites/foo.json"), "{}").unwrap();
        let r = ResourceRegistry::with_base_dir(dir);
        let path = r.resolve_uri("res://sprites/foo.json").expect("resolve");
        assert!(path.exists());
    }

    #[test]
    fn multiple_base_dirs_search_in_order() {
        let dir_a = tempdir();
        let dir_b = tempdir();
        std::fs::write(dir_b.join("only.json"), r#"{"from":"b"}"#).unwrap();
        let mut r = ResourceRegistry::new();
        r.add_base_dir(dir_a.clone());
        r.add_base_dir(dir_b.clone());
        let id = r.load("res://only.json").expect("load");
        assert_eq!(r.get(id).unwrap().data, json!({"from": "b"}));
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "craft_resource_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
