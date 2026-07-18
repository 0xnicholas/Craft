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

#[derive(Debug, Clone)]
pub struct SpriteDrawCommand {
    pub texture_id: String,
    pub src_rect: Option<[f32; 4]>,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub modulate: [f32; 4],
    pub z_index: i32,
}

impl<'a> ComponentView<'a> {
    pub fn sprite_draw_command(&self) -> Option<SpriteDrawCommand> {
        let sprite_path = self.get_string("sprite")?;
        if !self.get_bool("visible").unwrap_or(true) {
            return None;
        }
        let alpha = self.get_float("alpha").unwrap_or(1.0) as f32;
        if alpha <= 0.0 {
            return None;
        }
        let position = self.position.map(|(x, y)| [x as f32, y as f32])?;
        let modulate = self
            .get_vec3("modulate")
            .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32, alpha])
            .unwrap_or([1.0, 1.0, 1.0, alpha]);
        Some(SpriteDrawCommand {
            texture_id: sprite_path,
            src_rect: self
                .get_rect("sprite_rect")
                .map(|r| [r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32]),
            position,
            scale: self
                .get_vec2("scale")
                .map(|s| [s[0] as f32, s[1] as f32])
                .unwrap_or([1.0, 1.0]),
            rotation: self.get_float("rotation").unwrap_or(0.0) as f32,
            modulate,
            z_index: self.get_int("z_index").unwrap_or(0) as i32,
        })
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::String(s) => Some(s.clone()),
            _ => None,
        })
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::Bool(b) => Some(*b),
            _ => None,
        })
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::Float(f) => Some(*f),
            ComponentValue::Int(i) => Some(*i as f64),
            _ => None,
        })
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::Int(i) => Some(*i),
            _ => None,
        })
    }

    pub fn get_vec2(&self, key: &str) -> Option<[f64; 2]> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::Vec2(v) => Some(*v),
            _ => None,
        })
    }

    pub fn get_vec3(&self, key: &str) -> Option<[f64; 3]> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::Vec3(v) => Some(*v),
            _ => None,
        })
    }

    pub fn get_rect(&self, key: &str) -> Option<[f64; 4]> {
        self.components.get(key).and_then(|c| match &c.value {
            ComponentValue::Rect(r) => Some(*r),
            _ => None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub position: (f64, f64),
    pub zoom: f64,
    pub follow: Option<String>,
}

pub fn extract_camera(components: &[ComponentView]) -> Option<CameraInfo> {
    for view in components {
        if view.type_name == "Camera2D" {
            let position = view.position.unwrap_or((0.0, 0.0));
            let zoom = view.get_float("zoom").unwrap_or(1.0);
            let follow = view.get_string("follow");
            return Some(CameraInfo {
                position,
                zoom,
                follow,
            });
        }
    }
    None
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
            lua_class: None,
            destroyed: false,
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

    #[test]
    fn sprite_draw_command_extracts_from_view_with_sprite() {
        let mut components = BTreeMap::new();
        components.insert(
            "sprite".to_string(),
            Component {
                value: ComponentValue::String("player.png".to_string()),
                kind: crate::scene::ComponentKind::Regular,
            },
        );
        let view = ComponentView {
            node_id: "p1",
            type_name: "Player",
            components: &components,
            position: Some((10.0, 20.0)),
        };
        let cmd = view.sprite_draw_command().unwrap();
        assert_eq!(cmd.texture_id, "player.png");
        assert_eq!(cmd.position, [10.0, 20.0]);
    }

    #[test]
    fn sprite_draw_command_returns_none_without_sprite() {
        let components = BTreeMap::new();
        let view = ComponentView {
            node_id: "p1",
            type_name: "Player",
            components: &components,
            position: Some((0.0, 0.0)),
        };
        assert!(view.sprite_draw_command().is_none());
    }

    #[test]
    fn sprite_draw_command_skips_invisible() {
        let mut components = BTreeMap::new();
        components.insert(
            "sprite".to_string(),
            Component {
                value: ComponentValue::String("p.png".to_string()),
                kind: crate::scene::ComponentKind::Regular,
            },
        );
        components.insert(
            "visible".to_string(),
            Component {
                value: ComponentValue::Bool(false),
                kind: crate::scene::ComponentKind::Regular,
            },
        );
        let view = ComponentView {
            node_id: "p1",
            type_name: "Player",
            components: &components,
            position: Some((0.0, 0.0)),
        };
        assert!(view.sprite_draw_command().is_none());
    }

    #[test]
    fn extract_camera_finds_camera2d_node() {
        let components = BTreeMap::new();
        let views = vec![ComponentView {
            node_id: "cam",
            type_name: "Camera2D",
            components: &components,
            position: Some((100.0, 200.0)),
        }];
        let cam = extract_camera(&views).unwrap();
        assert_eq!(cam.position, (100.0, 200.0));
    }
}
