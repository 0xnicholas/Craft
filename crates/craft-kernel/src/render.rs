use std::collections::BTreeMap;

use crate::scene::{Component, ComponentValue, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    pub const fn default_for_terminal() -> Self {
        Self::new(80, 24)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RenderCapabilities: u32 {
        const TEXT    = 1 << 0;
        const SPRITE  = 1 << 1;
        const SHADER  = 1 << 2;
        const MESH    = 1 << 3;
    }
}

impl Default for RenderCapabilities {
    fn default() -> Self {
        Self::TEXT
    }
}

#[derive(Debug, Clone)]
pub struct ComponentView<'a> {
    pub node_id: &'a str,
    pub type_name: &'a str,
    pub components: &'a BTreeMap<String, Component>,
    pub position: Option<(f64, f64)>,
}

impl<'a> ComponentView<'a> {
    pub fn from_node(node: &'a Node) -> Self {
        let position = node
            .components
            .get("position")
            .and_then(|c| match &c.value {
                ComponentValue::Vec2([x, y]) => Some((*x, *y)),
                ComponentValue::Float(f) => Some((*f, 0.0)),
                _ => None,
            });
        Self {
            node_id: &node.id,
            type_name: &node.type_name,
            components: &node.components,
            position,
        }
    }
}

impl Default for ComponentView<'_> {
    fn default() -> Self {
        static EMPTY: std::sync::OnceLock<BTreeMap<String, Component>> = std::sync::OnceLock::new();
        let map = EMPTY.get_or_init(BTreeMap::new);
        Self {
            node_id: "",
            type_name: "",
            components: map,
            position: None,
        }
    }
}

pub trait Render: Send {
    fn render(&mut self, components: &[ComponentView], tick: u64);
    fn viewport(&self) -> Viewport;
    fn resize(&mut self, viewport: Viewport);
    fn shutdown(&mut self);

    fn capabilities(&self) -> RenderCapabilities {
        RenderCapabilities::TEXT
    }
}

#[derive(Debug, Default)]
pub struct NullRenderer {
    viewport: Viewport,
    frames_rendered: u64,
}

impl NullRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn frames_rendered(&self) -> u64 {
        self.frames_rendered
    }
}

impl Render for NullRenderer {
    fn render(&mut self, _components: &[ComponentView], _tick: u64) {
        self.frames_rendered += 1;
    }
    fn viewport(&self) -> Viewport {
        self.viewport
    }
    fn resize(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }
    fn shutdown(&mut self) {}
}

pub fn views_for_scene(nodes: &[Node]) -> Vec<ComponentView<'_>> {
    nodes.iter().map(ComponentView::from_node).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::ComponentKind;

    fn node_with_position(id: &str, x: f64, y: f64) -> Node {
        let mut components = BTreeMap::new();
        components.insert(
            "position".to_string(),
            Component {
                value: ComponentValue::Vec2([x, y]),
                kind: ComponentKind::Regular,
            },
        );
        Node {
            id: id.to_string(),
            type_name: "Test".to_string(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
        }
    }

    #[test]
    fn viewport_default_for_terminal() {
        let v = Viewport::default_for_terminal();
        assert_eq!(v.width, 80);
        assert_eq!(v.height, 24);
    }

    #[test]
    fn capabilities_default_is_text_only() {
        let caps = RenderCapabilities::default();
        assert!(caps.contains(RenderCapabilities::TEXT));
        assert!(!caps.contains(RenderCapabilities::SPRITE));
    }

    #[test]
    fn component_view_extracts_position() {
        let n = node_with_position("p1", 10.0, 20.0);
        let view = ComponentView::from_node(&n);
        assert_eq!(view.node_id, "p1");
        assert_eq!(view.position, Some((10.0, 20.0)));
    }

    #[test]
    fn views_for_scene_flattens_all_nodes() {
        let nodes = vec![
            node_with_position("a", 0.0, 0.0),
            node_with_position("b", 1.0, 1.0),
        ];
        let views = views_for_scene(&nodes);
        assert_eq!(views.len(), 2);
    }

    #[test]
    fn null_renderer_records_frames() {
        let mut r = NullRenderer::new();
        assert_eq!(r.frames_rendered(), 0);
        r.render(&[], 1);
        r.render(&[], 2);
        assert_eq!(r.frames_rendered(), 2);
    }

    #[test]
    fn null_renderer_resize_updates_viewport() {
        let mut r = NullRenderer::new();
        r.resize(Viewport::new(120, 40));
        assert_eq!(r.viewport(), Viewport::new(120, 40));
    }

    #[test]
    fn null_renderer_reports_text_capability() {
        let r = NullRenderer::new();
        let caps = r.capabilities();
        assert!(caps.contains(RenderCapabilities::TEXT));
    }

    #[test]
    fn null_renderer_shutdown_is_safe() {
        let mut r = NullRenderer::new();
        r.shutdown();
    }
}
