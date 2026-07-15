use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, EngineResult, IoError, ParseError};

const RES_SCHEME: &str = "res://";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub project: ProjectSection,
    #[serde(default)]
    pub paths: PathsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    #[serde(default)]
    pub entry_scene: Option<String>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub tick_hz: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathsSection {
    #[serde(default)]
    pub res_root: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedProject {
    pub root: PathBuf,
    pub project: ProjectSection,
    pub res_root: PathBuf,
    pub tick_hz: u32,
    pub seed: u64,
}

impl Project {
    pub fn load(path: &Path) -> EngineResult<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            EngineError::Io(IoError::read(path.display().to_string(), e.to_string()))
        })?;
        let file = path.display().to_string();
        Self::parse(&contents, &file)
    }

    pub fn parse(contents: &str, file: &str) -> EngineResult<Self> {
        toml::from_str(contents).map_err(|e| {
            let span = e.span();
            let (line, column) = span
                .as_ref()
                .map(|r| {
                    (
                        line_at(contents, r.start) as u32,
                        column_at(contents, r.start) as u32,
                    )
                })
                .unwrap_or((0, 0));
            EngineError::Parse(ParseError {
                file: file.to_string(),
                line: Some(line),
                column: Some(column),
                message: e.message().to_string(),
                snippet: span.as_ref().and_then(|r| snippet_at(contents, r.start)),
            })
        })
    }

    pub fn resolve(&self, manifest_path: &Path) -> EngineResult<ResolvedProject> {
        let root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let res_root = match &self.paths.res_root {
            Some(r) => {
                if Path::new(r).is_absolute() {
                    return Err(EngineError::Internal(format!(
                        "paths.res_root must be relative (absolute paths are forbidden): {r}"
                    )));
                }
                root.join(r)
            }
            None => root.clone(),
        };

        if let Some(entry) = &self.project.entry_scene {
            if !entry.starts_with(RES_SCHEME) {
                return Err(EngineError::Internal(format!(
                    "project.entry_scene must start with \"{RES_SCHEME}\": got {entry}"
                )));
            }
        }

        Ok(ResolvedProject {
            root,
            project: self.project.clone(),
            res_root,
            tick_hz: self.project.tick_hz.unwrap_or(60),
            seed: self.project.seed.unwrap_or(0),
        })
    }
}

impl ResolvedProject {
    pub fn resolve_res(&self, uri: &str) -> Option<PathBuf> {
        let stripped = uri.strip_prefix(RES_SCHEME)?;
        if stripped.is_empty() {
            return None;
        }
        let joined = self.res_root.join(stripped);
        Some(normalize_path(&joined))
    }
}

fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in p.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn line_at(contents: &str, byte: usize) -> usize {
    contents[..byte.min(contents.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

fn column_at(contents: &str, byte: usize) -> usize {
    let line_start = contents[..byte.min(contents.len())]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    contents[line_start..byte.min(contents.len())]
        .chars()
        .count()
        + 1
}

fn snippet_at(contents: &str, byte: usize) -> Option<String> {
    let line = line_at(contents, byte);
    let lines: Vec<&str> = contents.lines().collect();
    let start = line.saturating_sub(1);
    let end = (line + 2).min(lines.len());
    Some(lines[start..end].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_manifest_from_prd() {
        let toml = r#"
[project]
name = "tower_defense"
entry_scene = "res://games/tower_defense/scene.json"
seed = 42
tick_hz = 60

[paths]
res_root = "."
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        assert_eq!(project.project.name, "tower_defense");
        assert_eq!(
            project.project.entry_scene.as_deref(),
            Some("res://games/tower_defense/scene.json")
        );
        assert_eq!(project.project.seed, Some(42));
        assert_eq!(project.project.tick_hz, Some(60));
        assert_eq!(project.paths.res_root.as_deref(), Some("."));
    }

    #[test]
    fn minimal_manifest_defaults_optional_fields() {
        let toml = r#"
[project]
name = "minimal"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        assert_eq!(project.project.name, "minimal");
        assert_eq!(project.project.entry_scene, None);
        assert_eq!(project.project.seed, None);
        assert_eq!(project.project.tick_hz, None);
        assert_eq!(project.paths.res_root, None);
    }

    #[test]
    fn resolves_res_root_relative_to_manifest() {
        let toml = r#"
[project]
name = "x"

[paths]
res_root = "assets"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let resolved = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect("resolve");
        assert_eq!(resolved.res_root, PathBuf::from("/proj/assets"));
    }

    #[test]
    fn rejects_absolute_res_root() {
        let toml = r#"
[project]
name = "x"

[paths]
res_root = "/etc/passwd"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let err = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect_err("must fail");
        assert!(matches!(err, EngineError::Internal(_)));
    }

    #[test]
    fn resolve_res_decodes_uri() {
        let toml = r#"
[project]
name = "x"

[paths]
res_root = "assets"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let resolved = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect("resolve");
        let path = resolved
            .resolve_res("res://sprites/foo.json")
            .expect("resolve");
        assert_eq!(path, PathBuf::from("/proj/assets/sprites/foo.json"));
    }

    #[test]
    fn resolve_res_rejects_non_res_uri() {
        let toml = r#"
[project]
name = "x"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let resolved = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect("resolve");
        assert_eq!(resolved.resolve_res("/etc/passwd"), None);
        assert_eq!(resolved.resolve_res(""), None);
    }

    #[test]
    fn resolve_res_normalizes_parent_traversal() {
        let toml = r#"
[project]
name = "x"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let resolved = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect("resolve");
        let path = resolved
            .resolve_res("res://a/b/../c.json")
            .expect("resolve");
        assert_eq!(path, PathBuf::from("/proj/a/c.json"));
    }

    #[test]
    fn entry_scene_must_use_res_scheme() {
        let toml = r#"
[project]
name = "x"
entry_scene = "scenes/main.json"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let err = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect_err("must fail");
        assert!(matches!(err, EngineError::Internal(_)));
    }

    #[test]
    fn rejects_malformed_toml() {
        let toml = "this is not = = valid toml ===";
        let err = Project::parse(toml, "craft.toml").expect_err("must fail");
        assert!(matches!(err, EngineError::Parse(_)));
    }

    #[test]
    fn rejects_missing_project_section() {
        let toml = r#"
name = "tower_defense"
"#;
        let err = Project::parse(toml, "craft.toml").expect_err("must fail");
        assert!(matches!(err, EngineError::Parse(_)));
    }

    #[test]
    fn rejects_missing_project_name() {
        let toml = r#"
[project]
version = "0.1.0"
"#;
        let err = Project::parse(toml, "craft.toml").expect_err("must fail");
        assert!(matches!(err, EngineError::Parse(_)));
    }

    #[test]
    fn resolved_defaults_tick_hz_to_60() {
        let toml = r#"
[project]
name = "x"
"#;
        let project = Project::parse(toml, "craft.toml").expect("parse");
        let resolved = project
            .resolve(Path::new("/proj/craft.toml"))
            .expect("resolve");
        assert_eq!(resolved.tick_hz, 60);
        assert_eq!(resolved.seed, 0);
    }
}
