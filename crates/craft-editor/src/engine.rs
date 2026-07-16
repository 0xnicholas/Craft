use std::path::{Path, PathBuf};
use std::time::Instant;

use craft_kernel::{
    ComponentView, Engine, EngineError, EngineResult, HotReloadResult, NodeRegistry, Render,
};

use crate::io::load_scene;
use crate::renderer::EditorRenderer;
use crate::state::EditorError;

pub struct EditorEngine {
    pub engine: Engine,
    pub renderer: EditorRenderer,
    pub scene_path: Option<PathBuf>,
    pub is_running: bool,
    pub is_paused: bool,
    last_tick: Instant,
    tick_rate_hz: u32,
    pub lua_runtime: Option<craft_lua::LuaRuntime>,
    pub lua_init_error: Option<String>,
}

impl EditorEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        let renderer = EditorRenderer::new();
        engine.enable_rendering(true);
        let (lua_runtime, lua_init_error) = match craft_lua::LuaRuntime::new(0) {
            Ok(rt) => (Some(rt), None),
            Err(e) => (None, Some(e.to_string())),
        };
        Self {
            engine,
            renderer,
            scene_path: None,
            is_running: false,
            is_paused: false,
            last_tick: Instant::now(),
            tick_rate_hz: 60,
            lua_runtime,
            lua_init_error,
        }
    }

    pub fn load_scene_file(&mut self, path: &Path) -> EngineResult<()> {
        let registry: &NodeRegistry = self.engine.node_registry_mut();
        let scene = load_scene(path, registry).map_err(engine_err_from_editor)?;
        self.engine.load_scene(scene);
        self.scene_path = Some(path.to_path_buf());
        self.is_running = true;
        self.render_current_scene();
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = false;
    }

    pub fn step(&mut self) {
        self.engine.tick();
        self.render_current_scene();
    }

    pub fn reload(&mut self) -> EngineResult<HotReloadResult> {
        let Some(path) = &self.scene_path else {
            return Err(EngineError::Internal("no scene loaded".into()));
        };
        let registry: &NodeRegistry = self.engine.node_registry_mut();
        let scene = load_scene(path, registry).map_err(engine_err_from_editor)?;
        let result = self.engine.apply_hot_reload(&scene)?;
        self.render_current_scene();
        Ok(result)
    }

    pub fn tick_if_due(&mut self) -> bool {
        if !self.is_running || self.is_paused {
            return false;
        }
        let period = std::time::Duration::from_micros(1_000_000 / self.tick_rate_hz as u64);
        if self.last_tick.elapsed() < period {
            return false;
        }
        self.engine.tick();
        self.last_tick = Instant::now();
        self.render_current_scene();
        true
    }

    fn render_current_scene(&mut self) {
        let views: Vec<ComponentView<'_>> = self
            .engine
            .scene()
            .map(|scene| scene.nodes.iter().map(ComponentView::from_node).collect())
            .unwrap_or_default();
        self.renderer.render(&views, self.engine.tick);
    }

    pub fn renderer(&self) -> &EditorRenderer {
        &self.renderer
    }

    pub fn scene_path(&self) -> Option<&Path> {
        self.scene_path.as_deref()
    }

    pub fn lua_runtime_mut(&mut self) -> Option<&mut craft_lua::LuaRuntime> {
        self.lua_runtime.as_mut()
    }

    pub fn lua_runtime(&self) -> Option<&craft_lua::LuaRuntime> {
        self.lua_runtime.as_ref()
    }

    pub fn lua_runtime_error(&self) -> Option<&str> {
        self.lua_init_error.as_deref()
    }
}

impl Default for EditorEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn engine_err_from_editor(error: EditorError) -> EngineError {
    EngineError::Internal(error.to_string())
}
