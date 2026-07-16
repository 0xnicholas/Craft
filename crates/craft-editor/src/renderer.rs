use craft_kernel::{ComponentView, Render, Viewport};

#[derive(Debug, Clone, Copy)]
pub struct MonoCell {
    pub ch: char,
}

#[derive(Debug, Clone)]
pub struct TerminalGrid {
    pub cells: Vec<MonoCell>,
    pub width: u16,
    pub height: u16,
}

impl TerminalGrid {
    pub fn blank(width: u16, height: u16) -> Self {
        Self {
            cells: vec![MonoCell { ch: ' ' }; (width as usize) * (height as usize)],
            width,
            height,
        }
    }

    pub fn cell(&self, x: usize, y: usize) -> MonoCell {
        let idx = y * self.width as usize + x;
        self.cells[idx]
    }
}

pub struct EditorRenderer {
    viewport: Viewport,
    last_grid: TerminalGrid,
    frame_counter: u64,
}

impl EditorRenderer {
    pub fn new() -> Self {
        let viewport = Viewport::default_for_terminal();
        let mut renderer = Self {
            last_grid: TerminalGrid::blank(viewport.width as u16, viewport.height as u16),
            viewport,
            frame_counter: 0,
        };
        renderer.draw_borders();
        renderer
    }

    pub fn frames_rendered(&self) -> u64 {
        self.frame_counter
    }

    pub fn grid(&self) -> &TerminalGrid {
        &self.last_grid
    }

    fn write_at(&mut self, x: usize, y: usize, ch: char) {
        let w = self.viewport.width as usize;
        let h = self.viewport.height as usize;
        if x >= w || y >= h {
            return;
        }
        self.last_grid.cells[y * w + x] = MonoCell { ch };
    }

    fn draw_borders(&mut self) {
        let w = self.viewport.width as usize;
        let h = self.viewport.height as usize;
        if w == 0 || h == 0 {
            return;
        }
        for x in 0..w {
            self.write_at(x, 0, '-');
            self.write_at(x, h - 1, '-');
        }
        for y in 0..h {
            self.write_at(0, y, '|');
            self.write_at(w - 1, y, '|');
        }
        self.write_at(0, 0, '+');
        self.write_at(w - 1, 0, '+');
        self.write_at(0, h - 1, '+');
        self.write_at(w - 1, h - 1, '+');
    }
}

impl Default for EditorRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for EditorRenderer {
    fn render(&mut self, components: &[ComponentView], _tick: u64) {
        self.frame_counter += 1;
        for cell in &mut self.last_grid.cells {
            cell.ch = ' ';
        }
        self.draw_borders();

        for view in components {
            let Some((px, py)) = view.position else {
                continue;
            };
            let glyph = crate::render_helpers::glyph_for_type(view.type_name);
            if let Some((sx, sy)) = crate::render_helpers::project_to_screen(px, py, self.viewport)
            {
                self.write_at(sx, sy, glyph);
            }
        }
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn resize(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        self.last_grid = TerminalGrid::blank(viewport.width as u16, viewport.height as u16);
        self.draw_borders();
    }

    fn shutdown(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_kernel::{Component, ComponentKind, ComponentValue, Node};
    use std::collections::BTreeMap;

    fn view_with_pos(id: &str, type_name: &str, x: f64, y: f64) -> ComponentView<'static> {
        let mut components = BTreeMap::new();
        components.insert(
            "position".into(),
            Component {
                value: ComponentValue::Vec2([x, y]),
                kind: ComponentKind::Regular,
            },
        );
        let node = Box::leak(Box::new(Node {
            id: id.into(),
            type_name: type_name.into(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
            lua_class: None,
            destroyed: false,
        }));
        ComponentView::from_node(node)
    }

    #[test]
    fn renders_empty_scene_no_panic() {
        let mut renderer = EditorRenderer::new();
        renderer.render(&[], 0);
        assert_eq!(renderer.frames_rendered(), 1);
    }

    #[test]
    fn draws_borders() {
        let renderer = EditorRenderer::new();
        let grid = renderer.grid();
        assert_eq!(grid.cell(0, 0).ch, '+');
    }

    #[test]
    fn draws_player_at_origin() {
        let mut renderer = EditorRenderer::new();
        let view = view_with_pos("p1", "Player", 0.0, 0.0);
        renderer.render(&[view], 1);
        let grid = renderer.grid();
        let (center_x, center_y) = (
            renderer.viewport().width as usize / 2,
            renderer.viewport().height as usize / 2,
        );
        assert_eq!(grid.cell(center_x, center_y).ch, '@');
    }
}
