use std::collections::BTreeMap;
use std::fmt::Write;

use craft_kernel::scene::Component;
use craft_kernel::{ComponentValue, ComponentView, Render, RenderCapabilities, Viewport};

pub struct AnsiRenderer {
    viewport: Viewport,
    last_frame: Option<String>,
    frame_counter: u64,
    buffer: Vec<char>,
}

impl AnsiRenderer {
    pub fn new() -> Self {
        Self::with_viewport(Viewport::default_for_terminal())
    }

    pub fn with_viewport(viewport: Viewport) -> Self {
        let size = (viewport.width as usize) * (viewport.height as usize);
        Self {
            viewport,
            last_frame: None,
            frame_counter: 0,
            buffer: vec![' '; size],
        }
    }

    pub fn last_frame(&self) -> Option<&str> {
        self.last_frame.as_deref()
    }

    pub fn frames_rendered(&self) -> u64 {
        self.frame_counter
    }

    fn clear_buffer(&mut self) {
        for ch in &mut self.buffer {
            *ch = ' ';
        }
    }

    fn write_at(&mut self, x: usize, y: usize, ch: char) {
        let w = self.viewport.width as usize;
        let h = self.viewport.height as usize;
        if x >= w || y >= h {
            return;
        }
        self.buffer[y * w + x] = ch;
    }

    fn project_to_screen(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let w = self.viewport.width as usize;
        let h = self.viewport.height as usize;
        let half_w = w as f64 / 2.0;
        let half_h = h as f64 / 2.0;
        let sx = (x + half_w).round() as i64;
        let sy = (-(y) + half_h).round() as i64;
        if sx < 0 || sy < 0 {
            return None;
        }
        let sx = sx as usize;
        let sy = sy as usize;
        if sx >= w || sy >= h {
            return None;
        }
        Some((sx, sy))
    }

    fn glyph_for_type(type_name: &str) -> char {
        match type_name {
            "Player" | "BRPlayer" | "M7Player" | "HRPlayer" | "Probe" => '@',
            "Enemy" | "HREnemy" | "Mob" | "Bullet" => '*',
            "Tower" | "Spawner" => 'T',
            t if t.ends_with("Counter") => '#',
            t if t.ends_with("Enemy") => '*',
            t if t.ends_with("Player") => '@',
            _ => '.',
        }
    }
}

impl Default for AnsiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for AnsiRenderer {
    fn render(&mut self, components: &[ComponentView], tick: u64) {
        self.frame_counter += 1;
        self.clear_buffer();

        let w = self.viewport.width as usize;
        let h = self.viewport.height as usize;
        if w > 0 && h > 0 {
            for x in 0..w {
                self.write_at(x, 0, '-');
                self.write_at(x, h - 1, '-');
            }
            for y in 0..h {
                self.write_at(0, y, '|');
                self.write_at(w - 1, y, '|');
            }
        }

        let mut info = String::new();
        for view in components {
            let Some((sx, sy)) = view
                .position
                .and_then(|(x, y)| self.project_to_screen(x, y))
            else {
                continue;
            };
            let ch = Self::glyph_for_type(view.type_name);
            self.write_at(sx, sy, ch);
            let _ = write!(&mut info, "[{}]={} ", view.node_id, ch);
        }

        let mut out = String::new();
        let _ = write!(&mut out, "\x1b[H\x1b[2J");
        let _ = writeln!(
            &mut out,
            "tick={tick} nodes={} viewport={}x{}",
            components.len(),
            self.viewport.width,
            self.viewport.height
        );
        let _ = writeln!(&mut out, "{}", info.trim_end());

        for y in 0..h {
            let start = y * w;
            let end = start + w;
            let line: String = self.buffer[start..end].iter().collect();
            let _ = writeln!(&mut out, "{line}");
        }

        self.last_frame = Some(out);
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn resize(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        let size = (viewport.width as usize) * (viewport.height as usize);
        self.buffer = vec![' '; size];
    }

    fn shutdown(&mut self) {
        self.last_frame = None;
        self.buffer.clear();
    }

    fn capabilities(&self) -> RenderCapabilities {
        RenderCapabilities::TEXT
    }
}

pub fn component_value_to_text(v: &ComponentValue) -> String {
    match v {
        ComponentValue::Nil => "nil".to_string(),
        ComponentValue::Bool(b) => b.to_string(),
        ComponentValue::Int(i) => i.to_string(),
        ComponentValue::Float(f) => format!("{f:.2}"),
        ComponentValue::String(s) => s.clone(),
        ComponentValue::Vec2([x, y]) => format!("({x:.1}, {y:.1})"),
    }
}

#[allow(dead_code)]
fn _exhaustive_match(v: &ComponentValue) -> &'static str {
    match v {
        ComponentValue::Nil => "nil",
        ComponentValue::Bool(_) => "bool",
        ComponentValue::Int(_) => "int",
        ComponentValue::Float(_) => "float",
        ComponentValue::String(_) => "string",
        ComponentValue::Vec2(_) => "vec2",
    }
}

#[allow(dead_code)]
fn make_view<'a>(
    id: &'a str,
    type_name: &'a str,
    _x: f64,
    _y: f64,
    map: &'a BTreeMap<String, Component>,
) -> ComponentView<'a> {
    let position = map.get("position").and_then(|c| match &c.value {
        ComponentValue::Vec2([x, y]) => Some((*x, *y)),
        _ => None,
    });
    ComponentView {
        node_id: id,
        type_name,
        components: map,
        position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player_view<'a>(
        id: &'a str,
        x: f64,
        y: f64,
        map: &'a BTreeMap<String, Component>,
    ) -> ComponentView<'a> {
        make_view(id, "Player", x, y, map)
    }

    fn player_components(x: f64, y: f64) -> BTreeMap<String, Component> {
        let mut m = BTreeMap::new();
        m.insert(
            "position".to_string(),
            Component {
                value: ComponentValue::Vec2([x, y]),
                kind: craft_kernel::scene::ComponentKind::Regular,
            },
        );
        m
    }

    #[test]
    fn ansi_renderer_produces_output() {
        let map = player_components(0.0, 0.0);
        let mut r = AnsiRenderer::new();
        r.render(&[player_view("p1", 0.0, 0.0, &map)], 1);
        let frame = r.last_frame().expect("frame");
        assert!(frame.contains("tick=1"));
        assert!(frame.contains("nodes=1"));
        assert!(frame.contains("80x24"));
        assert!(frame.contains("@"));
    }

    #[test]
    fn ansi_renderer_draws_border() {
        let mut r = AnsiRenderer::with_viewport(Viewport::new(20, 5));
        r.render(&[], 1);
        let frame = r.last_frame().expect("frame");
        assert!(frame.contains("----"));
        assert!(frame.contains("|"));
    }

    #[test]
    fn ansi_renderer_renders_multiple_nodes() {
        let p1 = player_components(0.0, 0.0);
        let p2 = player_components(2.0, 3.0);
        let mut r = AnsiRenderer::with_viewport(Viewport::new(40, 20));
        r.render(
            &[
                player_view("p1", 0.0, 0.0, &p1),
                player_view("p2", 2.0, 3.0, &p2),
            ],
            1,
        );
        let frame = r.last_frame().expect("frame");
        assert!(frame.contains("[p1]=@"));
        assert!(frame.contains("[p2]=@"));
    }

    #[test]
    fn ansi_renderer_increments_frame_counter() {
        let mut r = AnsiRenderer::new();
        assert_eq!(r.frames_rendered(), 0);
        r.render(&[], 1);
        r.render(&[], 2);
        r.render(&[], 3);
        assert_eq!(r.frames_rendered(), 3);
    }

    #[test]
    fn resize_clears_buffer() {
        let mut r = AnsiRenderer::with_viewport(Viewport::new(10, 5));
        r.render(&[], 1);
        r.resize(Viewport::new(20, 10));
        assert_eq!(r.viewport(), Viewport::new(20, 10));
        r.render(&[], 2);
        let frame = r.last_frame().expect("frame");
        assert!(frame.contains("20x10"));
    }

    #[test]
    fn glyph_for_player_uses_at_sign() {
        assert_eq!(AnsiRenderer::glyph_for_type("Player"), '@');
        assert_eq!(AnsiRenderer::glyph_for_type("HRPlayer"), '@');
    }

    #[test]
    fn glyph_for_enemy_uses_asterisk() {
        assert_eq!(AnsiRenderer::glyph_for_type("Enemy"), '*');
        assert_eq!(AnsiRenderer::glyph_for_type("HREnemy"), '*');
    }

    #[test]
    fn component_value_to_text_for_int() {
        assert_eq!(component_value_to_text(&ComponentValue::Int(42)), "42");
    }

    #[test]
    fn component_value_to_text_for_vec2() {
        assert_eq!(
            component_value_to_text(&ComponentValue::Vec2([1.0, 2.5])),
            "(1.0, 2.5)"
        );
    }

    #[test]
    fn component_value_to_text_for_string() {
        assert_eq!(
            component_value_to_text(&ComponentValue::String("hello".to_string())),
            "hello"
        );
    }

    #[test]
    fn shutdown_clears_frame_buffer() {
        let mut r = AnsiRenderer::new();
        r.render(&[], 1);
        r.shutdown();
        assert!(r.last_frame().is_none());
    }

    #[test]
    fn out_of_viewport_position_is_skipped() {
        let map = player_components(1000.0, 1000.0);
        let mut r = AnsiRenderer::with_viewport(Viewport::new(20, 10));
        r.render(&[player_view("far", 1000.0, 1000.0, &map)], 1);
        let frame = r.last_frame().expect("frame");
        assert!(!frame.contains("[far]="));
    }

    #[test]
    fn capabilities_is_text() {
        let r = AnsiRenderer::new();
        assert!(r.capabilities().contains(RenderCapabilities::TEXT));
    }
}
