use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::behavior::Behavior;
use crate::error::EngineResult;
use crate::scene::{Component, ComponentValue, Node, NodeRegistry, Scene};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ComponentChange {
    Updated {
        from: ComponentValue,
        to: ComponentValue,
    },
    Removed {
        previous: ComponentValue,
    },
    Added {
        value: ComponentValue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneDiff {
    pub added_nodes: Vec<Node>,
    pub removed_node_ids: Vec<String>,
    pub type_changes: BTreeMap<String, String>,
    pub component_changes: BTreeMap<String, BTreeMap<String, ComponentChange>>,
    pub behavior_changes: BTreeMap<String, Vec<Behavior>>,
}

impl SceneDiff {
    pub fn is_empty(&self) -> bool {
        self.added_nodes.is_empty()
            && self.removed_node_ids.is_empty()
            && self.type_changes.is_empty()
            && self.component_changes.is_empty()
            && self.behavior_changes.is_empty()
    }

    pub fn affected_node_ids(&self) -> Vec<String> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        for n in &self.added_nodes {
            out.insert(n.id.clone());
        }
        for id in &self.removed_node_ids {
            out.insert(id.clone());
        }
        for k in self.type_changes.keys() {
            out.insert(k.clone());
        }
        for k in self.component_changes.keys() {
            out.insert(k.clone());
        }
        for k in self.behavior_changes.keys() {
            out.insert(k.clone());
        }
        out.into_iter().collect()
    }
}

pub fn compute_scene_diff(current: &Scene, new: &Scene) -> SceneDiff {
    let mut diff = SceneDiff::default();

    let mut current_by_id: BTreeMap<&str, &Node> = BTreeMap::new();
    for n in &current.nodes {
        current_by_id.insert(n.id.as_str(), n);
    }
    let mut new_by_id: BTreeMap<&str, &Node> = BTreeMap::new();
    for n in &new.nodes {
        new_by_id.insert(n.id.as_str(), n);
    }

    for (id, new_node) in &new_by_id {
        match current_by_id.get(id) {
            None => diff.added_nodes.push((*new_node).clone()),
            Some(curr) => {
                if curr.type_name != new_node.type_name {
                    diff.type_changes
                        .insert((*id).to_string(), new_node.type_name.clone());
                }
                if curr.behaviors != new_node.behaviors {
                    diff.behavior_changes
                        .insert((*id).to_string(), new_node.behaviors.clone());
                }
                let comp_changes =
                    compute_component_changes(&curr.components, &new_node.components);
                if !comp_changes.is_empty() {
                    diff.component_changes
                        .insert((*id).to_string(), comp_changes);
                }
            }
        }
    }
    for id in current_by_id.keys() {
        if !new_by_id.contains_key(*id) {
            diff.removed_node_ids.push((*id).to_string());
        }
    }

    diff
}

fn compute_component_changes(
    current: &BTreeMap<String, Component>,
    new: &BTreeMap<String, Component>,
) -> BTreeMap<String, ComponentChange> {
    let mut out = BTreeMap::new();
    for (key, new_comp) in new {
        match current.get(key) {
            None => {
                out.insert(
                    key.clone(),
                    ComponentChange::Added {
                        value: new_comp.value.clone(),
                    },
                );
            }
            Some(curr_comp) => {
                if curr_comp.value != new_comp.value {
                    out.insert(
                        key.clone(),
                        ComponentChange::Updated {
                            from: curr_comp.value.clone(),
                            to: new_comp.value.clone(),
                        },
                    );
                }
            }
        }
    }
    for key in current.keys() {
        if !new.contains_key(key) {
            if let Some(prev) = current.get(key) {
                out.insert(
                    key.clone(),
                    ComponentChange::Removed {
                        previous: prev.value.clone(),
                    },
                );
            }
        }
    }
    out
}

pub fn apply_scene_diff(
    scene: &mut Scene,
    registry: &NodeRegistry,
    diff: &SceneDiff,
) -> EngineResult<()> {
    for id in &diff.removed_node_ids {
        scene.nodes.retain(|n| &n.id != id);
    }

    for (id, new_type) in &diff.type_changes {
        if registry.get(new_type).is_none() {
            continue;
        }
        if let Some(node) = scene.nodes.iter_mut().find(|n| &n.id == id) {
            node.type_name = new_type.clone();
        }
    }

    for (id, behaviors) in &diff.behavior_changes {
        if let Some(node) = scene.nodes.iter_mut().find(|n| &n.id == id) {
            node.behaviors = behaviors.clone();
        }
    }

    for (id, changes) in &diff.component_changes {
        if let Some(node) = scene.nodes.iter_mut().find(|n| &n.id == id) {
            for (key, change) in changes {
                match change {
                    ComponentChange::Updated { to, .. } | ComponentChange::Added { value: to } => {
                        node.components.insert(
                            key.clone(),
                            Component {
                                value: to.clone(),
                                kind: Default::default(),
                            },
                        );
                    }
                    ComponentChange::Removed { .. } => {
                        node.components.remove(key);
                    }
                }
            }
        }
    }

    for new_node in &diff.added_nodes {
        if registry.get(&new_node.type_name).is_none() {
            continue;
        }
        scene.nodes.push(new_node.clone());
    }

    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HotReloadResult {
    pub diff: SceneDiff,
    pub affected_node_ids: Vec<String>,
    pub applied: bool,
}

pub fn hot_reload_scene(
    scene: &mut Scene,
    registry: &NodeRegistry,
    new_scene: &Scene,
) -> EngineResult<HotReloadResult> {
    let diff = compute_scene_diff(scene, new_scene);
    let affected = diff.affected_node_ids();
    if diff.is_empty() {
        return Ok(HotReloadResult {
            diff,
            affected_node_ids: affected,
            applied: false,
        });
    }
    apply_scene_diff(scene, registry, &diff)?;
    Ok(HotReloadResult {
        diff,
        affected_node_ids: affected,
        applied: true,
    })
}

pub fn snapshot_node_value(current: &Scene, node_id: &str, key: &str) -> Option<ComponentValue> {
    current
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .and_then(|n| n.components.get(key))
        .map(|c| c.value.clone())
}

pub fn snapshot_count(scene: &Scene) -> usize {
    scene.nodes.len()
}

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub struct HotReloader {
    _watcher: RecommendedWatcher,
    rx: Receiver<PathBuf>,
}

impl std::fmt::Debug for HotReloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotReloader")
            .field("watching", &"<watcher handle>")
            .finish()
    }
}

impl HotReloader {
    pub fn watch(path: &Path) -> EngineResult<Self> {
        let (tx, rx) = channel::<PathBuf>();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    ) {
                        for p in event.paths {
                            let _ = tx.send(p);
                        }
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(200)),
        )
        .map_err(|e| {
            crate::error::EngineError::Io(crate::error::IoError::read(
                path.display().to_string(),
                format!("watcher init failed: {e}"),
            ))
        })?;
        let watch_target = if path.is_file() {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.to_path_buf())
        } else {
            path.to_path_buf()
        };
        watcher
            .watch(&watch_target, RecursiveMode::NonRecursive)
            .map_err(|e| {
                crate::error::EngineError::Io(crate::error::IoError::read(
                    path.display().to_string(),
                    format!("watch failed: {e}"),
                ))
            })?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn try_recv(&self) -> Option<PathBuf> {
        self.rx.try_recv().ok()
    }

    pub fn drain(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        while let Ok(p) = self.rx.try_recv() {
            out.push(p);
        }
        out
    }
}

pub fn reload_from_path(
    scene: &mut Scene,
    registry: &NodeRegistry,
    path: &Path,
) -> EngineResult<HotReloadResult> {
    let new_scene = Scene::load(path, registry)?;
    hot_reload_scene(scene, registry, &new_scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{ComponentKind, ComponentType};
    use serde_json::json;

    struct Player;
    impl Default for Player {
        fn default() -> Self {
            Player
        }
    }
    impl crate::scene::NodeDef for Player {
        fn type_name(&self) -> &'static str {
            "Player"
        }
        fn component_specs(&self) -> Vec<crate::scene::ComponentSpec> {
            vec![
                crate::scene::ComponentSpec::new(
                    "position",
                    ComponentType::Vec2,
                    json!([0.0, 0.0]),
                ),
                crate::scene::ComponentSpec::new("health", ComponentType::Int, json!(100)),
            ]
        }
    }

    struct Enemy;
    impl Default for Enemy {
        fn default() -> Self {
            Enemy
        }
    }
    impl crate::scene::NodeDef for Enemy {
        fn type_name(&self) -> &'static str {
            "Enemy"
        }
        fn component_specs(&self) -> Vec<crate::scene::ComponentSpec> {
            vec![crate::scene::ComponentSpec::new(
                "position",
                ComponentType::Vec2,
                json!([0.0, 0.0]),
            )]
        }
    }

    fn registry() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        r.register::<Player>();
        r.register::<Enemy>();
        r
    }

    fn make_node(id: &str, type_name: &str, health: i64) -> Node {
        let mut components = BTreeMap::new();
        components.insert(
            "position".to_string(),
            Component {
                value: ComponentValue::Vec2([0.0, 0.0]),
                kind: ComponentKind::Regular,
            },
        );
        components.insert(
            "health".to_string(),
            Component {
                value: ComponentValue::Int(health),
                kind: ComponentKind::Regular,
            },
        );
        Node {
            id: id.to_string(),
            type_name: type_name.to_string(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
            lua_class: None,
            destroyed: false,
        }
    }

    fn make_scene(nodes: Vec<Node>) -> Scene {
        Scene {
            kind: "scene".to_string(),
            name: "t".to_string(),
            nodes,
            spawn_counter: 0,
        }
    }

    #[test]
    fn diff_detects_component_change() {
        let current = make_scene(vec![make_node("p1", "Player", 100)]);
        let mut new = current.clone();
        new.nodes[0].components.insert(
            "health".to_string(),
            Component {
                value: ComponentValue::Int(50),
                kind: ComponentKind::Regular,
            },
        );
        let diff = compute_scene_diff(&current, &new);
        let changes = diff.component_changes.get("p1").expect("changes");
        assert!(changes.contains_key("health"));
    }

    #[test]
    fn diff_detects_added_node() {
        let current = make_scene(vec![make_node("p1", "Player", 100)]);
        let mut new = current.clone();
        new.nodes.push(make_node("e1", "Enemy", 0));
        let diff = compute_scene_diff(&current, &new);
        assert_eq!(diff.added_nodes.len(), 1);
        assert_eq!(diff.added_nodes[0].id, "e1");
    }

    #[test]
    fn diff_detects_removed_node() {
        let mut current = make_scene(vec![
            make_node("p1", "Player", 100),
            make_node("e1", "Enemy", 0),
        ]);
        let mut new = current.clone();
        new.nodes.retain(|n| n.id != "e1");
        let diff = compute_scene_diff(&current, &new);
        assert_eq!(diff.removed_node_ids, vec!["e1".to_string()]);
        current.nodes.retain(|n| n.id != "e1");
        apply_scene_diff(&mut current, &registry(), &diff).expect("apply");
        assert_eq!(current.nodes.len(), 1);
    }

    #[test]
    fn diff_detects_behavior_change() {
        let current = make_scene(vec![make_node("p1", "Player", 100)]);
        let mut new = current.clone();
        new.nodes[0].behaviors = vec![Behavior::OnTick { actions: vec![] }];
        let diff = compute_scene_diff(&current, &new);
        assert!(diff.behavior_changes.contains_key("p1"));
    }

    #[test]
    fn diff_detects_type_change() {
        let current = make_scene(vec![make_node("p1", "Player", 100)]);
        let mut new = current.clone();
        new.nodes[0].type_name = "Enemy".to_string();
        let diff = compute_scene_diff(&current, &new);
        assert_eq!(diff.type_changes.get("p1"), Some(&"Enemy".to_string()));
    }

    #[test]
    fn apply_preserves_node_ids() {
        let mut current = make_scene(vec![make_node("p1", "Player", 100)]);
        let mut new = current.clone();
        new.nodes[0].components.insert(
            "health".to_string(),
            Component {
                value: ComponentValue::Int(50),
                kind: ComponentKind::Regular,
            },
        );
        let diff = compute_scene_diff(&current, &new);
        apply_scene_diff(&mut current, &registry(), &diff).expect("apply");
        assert_eq!(current.nodes[0].id, "p1");
    }

    #[test]
    fn empty_diff_when_unchanged() {
        let current = make_scene(vec![make_node("p1", "Player", 100)]);
        let new = current.clone();
        let diff = compute_scene_diff(&current, &new);
        assert!(diff.is_empty());
    }

    #[test]
    fn hot_reload_returns_applied_false_when_unchanged() {
        let mut current = make_scene(vec![make_node("p1", "Player", 100)]);
        let new = current.clone();
        let result = hot_reload_scene(&mut current, &registry(), &new).expect("hot reload");
        assert!(!result.applied);
    }

    #[test]
    fn hot_reload_applies_changes() {
        let mut current = make_scene(vec![make_node("p1", "Player", 100)]);
        let mut new = current.clone();
        new.nodes[0].components.insert(
            "health".to_string(),
            Component {
                value: ComponentValue::Int(50),
                kind: ComponentKind::Regular,
            },
        );
        let result = hot_reload_scene(&mut current, &registry(), &new).expect("hot reload");
        assert!(result.applied);
        assert_eq!(result.affected_node_ids, vec!["p1".to_string()]);
        let health = current.nodes[0]
            .components
            .get("health")
            .expect("health")
            .value
            .clone();
        assert_eq!(health, ComponentValue::Int(50));
    }

    #[test]
    fn resource_ref_snapshot_version_preserved() {
        let mut r = crate::ResourceRegistry::new();
        let id1 = r.register("res://foo.json", json!({"hp": 100}));
        let v1 = r.version(id1).expect("version");
        let _id_again = r.register("res://foo.json", json!({"hp": 150}));
        let v_after = r.version(id1).expect("version after");
        assert_eq!(
            v1, v_after,
            "re-registering does not bump the existing version"
        );
        let ref1 = r.resolve_ref("res://foo.json").expect("ref");
        assert_eq!(ref1.snapshot_version, v1);
    }

    #[test]
    fn new_resource_gets_higher_version() {
        let mut r = crate::ResourceRegistry::new();
        let id1 = r.register("res://a.json", json!({}));
        let v1 = r.version(id1).expect("v1");
        let id2 = r.register("res://b.json", json!({}));
        let v2 = r.version(id2).expect("v2");
        assert!(v2 > v1, "new resources get strictly higher versions");
    }
}
