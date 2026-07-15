use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// One entry in a `luarocks.lock` file: a named Lua module, the path
/// it resolves to, its pinned version, and a SHA-256 of its source for
/// drift detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockEntry {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lockfile {
    pub entries: Vec<LockEntry>,
}

impl Lockfile {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct Raw {
            #[serde(default)]
            modules: Vec<RawEntry>,
        }
        #[derive(serde::Deserialize)]
        struct RawEntry {
            name: String,
            version: String,
            path: PathBuf,
            sha256: String,
        }
        let raw: Raw = toml::from_str(s)?;
        Ok(Self {
            entries: raw
                .modules
                .into_iter()
                .map(|e| LockEntry {
                    name: e.name,
                    version: e.version,
                    path: e.path,
                    sha256: e.sha256,
                })
                .collect(),
        })
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        #[derive(serde::Serialize)]
        struct Raw<'a> {
            modules: Vec<RawEntry<'a>>,
        }
        #[derive(serde::Serialize)]
        struct RawEntry<'a> {
            name: &'a str,
            version: &'a str,
            path: &'a Path,
            sha256: &'a str,
        }
        let raw = Raw {
            modules: self
                .entries
                .iter()
                .map(|e| RawEntry {
                    name: &e.name,
                    version: &e.version,
                    path: &e.path,
                    sha256: &e.sha256,
                })
                .collect(),
        };
        toml::to_string(&raw)
    }

    pub fn lookup(&self, name: &str) -> Option<&LockEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Compute SHA-256 of file contents, returned as lowercase hex.
pub fn sha256_of_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// Compute SHA-256 of a string, returned as lowercase hex.
pub fn sha256_of_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

/// Holds the workspace-relative modules directory and a cache of
/// already-loaded module sources. Used by LuaRuntime to back the
/// `require(name)` call: turns "lib.vec2" into <modules_dir>/lib/vec2.lua,
/// reads it, validates against the lockfile if one is set, and caches
/// the result in `package.loaded`.
#[derive(Debug, Clone)]
pub struct ModuleLoader {
    pub modules_dir: PathBuf,
    pub lockfile: Lockfile,
}

impl ModuleLoader {
    pub fn new(modules_dir: PathBuf) -> Self {
        Self {
            modules_dir,
            lockfile: Lockfile::empty(),
        }
    }

    pub fn with_lockfile(modules_dir: PathBuf, lockfile: Lockfile) -> Self {
        Self {
            modules_dir,
            lockfile,
        }
    }

    pub fn resolve_path(&self, module_name: &str) -> Option<PathBuf> {
        let rel: PathBuf = module_name.split('.').collect();
        let candidate = self.modules_dir.join(&rel).with_extension("lua");
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    }

    pub fn load_lockfile_from_path(&mut self, lockfile_path: &Path) -> std::io::Result<()> {
        let s = std::fs::read_to_string(lockfile_path)?;
        self.lockfile = Lockfile::from_toml(&s).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("luarocks.lock: {e}"),
            )
        })?;
        Ok(())
    }

    pub fn write_lockfile_to_path(&self, lockfile_path: &Path) -> std::io::Result<()> {
        let s = self
            .lockfile
            .to_toml()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        std::fs::write(lockfile_path, s)
    }

    pub fn validate_lockfile(&self) -> Result<(), String> {
        for entry in &self.lockfile.entries {
            let abs = self.absolute_path(&entry.path);
            let hash = sha256_of_file(&abs).map_err(|e| {
                format!(
                    "luarocks.lock entry {:?}: cannot read {}: {}",
                    entry.name,
                    abs.display(),
                    e
                )
            })?;
            if !hash.eq_ignore_ascii_case(&entry.sha256) {
                return Err(format!(
                    "luarocks.lock drift for module {:?}: expected sha256 {} but found {} at {}",
                    entry.name,
                    entry.sha256,
                    hash,
                    abs.display()
                ));
            }
        }
        Ok(())
    }

    /// Validate one entry on demand (used by the runtime's require
    /// searcher to fail fast before executing module source).
    pub fn validate_lockfile_for(&self, name: &str, path: &Path) -> Result<(), String> {
        if self.lockfile.is_empty() {
            return Ok(());
        }
        let entry = match self.lockfile.lookup(name) {
            Some(e) => e,
            None => {
                return Err(format!(
                    "luarocks.lock: module {name:?} is not in the lockfile"
                ));
            }
        };
        let recorded_path = self.absolute_path(&entry.path);
        let actual_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.modules_dir.join(path)
        };
        if recorded_path != actual_path {
            return Err(format!(
                "luarocks.lock: module {name:?} path drift (recorded {}, actual {})",
                recorded_path.display(),
                actual_path.display()
            ));
        }
        let hash = sha256_of_file(path)
            .map_err(|e| format!("luarocks.lock: cannot hash {name:?}: {e}"))?;
        if !hash.eq_ignore_ascii_case(&entry.sha256) {
            return Err(format!(
                "luarocks.lock: hash drift for {name:?} (recorded {}, actual {})",
                entry.sha256, hash
            ));
        }
        Ok(())
    }

    fn absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.modules_dir.join(path)
        }
    }

    /// Build a lockfile from currently-loaded source registry. Pass a
    /// `HashMap<name, (version, source)>` and paths are computed relative
    /// to `modules_dir` if possible (else absolute).
    pub fn build_lockfile(&self, modules: &HashMap<String, (String, String)>) -> Lockfile {
        let entries = modules
            .iter()
            .map(|(name, (version, source))| {
                let rel = name.split('.').collect::<PathBuf>().with_extension("lua");
                let abs = self.modules_dir.join(&rel);
                LockEntry {
                    name: name.clone(),
                    version: version.clone(),
                    path: abs,
                    sha256: sha256_of_str(source),
                }
            })
            .collect();
        Lockfile { entries }
    }
}
