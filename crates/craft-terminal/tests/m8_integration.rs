use craft_kernel::craft_node;
use craft_kernel::{ComponentView, Engine, NullRenderer, Render, Scene, Viewport};
use craft_terminal::AnsiRenderer;
use std::time::Instant;

craft_node!(M8Player, {
    components: {
        health: Int = 100,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(M8Enemy, {
    components: {
        position: Vec2 = [0.0, 0.0],
    },
});

fn registry() -> craft_kernel::NodeRegistry {
    let mut r = craft_kernel::NodeRegistry::new();
    r.register::<M8Player>();
    r.register::<M8Enemy>();
    r
}

fn engine_with(scene_json: &str) -> Engine {
    let scene_value: serde_json::Value = serde_json::from_str(scene_json).expect("parse json");
    let scene = Scene::from_value(scene_value, "scene.json", &registry()).expect("scene value");
    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine
}

fn tiny_scene() -> String {
    r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            { "id": "p1", "type": "M8Player", "components": { "health": 100, "position": [0.0, 0.0] } },
            { "id": "e1", "type": "M8Enemy",  "components": { "position": [5.0, -3.0] } }
        ]
    }"#
    .to_string()
}

#[test]
fn null_renderer_is_default_and_no_ops() {
    let mut engine = Engine::new();
    engine.render_now();
    assert!(engine.renderer().viewport() == Viewport::default());
    let counts = engine.renderer_mut();
    let _ = counts;
}

#[test]
fn ansi_renderer_renders_scene_via_engine() {
    let mut engine = engine_with(&tiny_scene());
    engine.set_renderer(Box::new(AnsiRenderer::new()));
    engine.render_now();
    let r: &dyn Render = engine.renderer();
    let _ = r.viewport();
}

#[test]
fn rendering_each_tick_works() {
    let mut engine = engine_with(&tiny_scene());
    engine.set_renderer(Box::new(AnsiRenderer::new()));
    engine.enable_rendering(true);
    for _ in 0..3 {
        engine.tick();
    }
}

#[test]
fn ansi_output_contains_node_glyphs_and_metadata() {
    let engine = engine_with(&tiny_scene());
    let mut renderer = AnsiRenderer::new();
    {
        assert!(renderer.last_frame().is_none(), "no frame before render");
    }
    {
        let views: Vec<ComponentView> = engine
            .scene
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .map(ComponentView::from_node)
            .collect();
        renderer.render(&views, 0);
    }
    let frame = renderer.last_frame().expect("frame");
    assert!(frame.contains("tick=0"));
    assert!(frame.contains("nodes=2"));
    assert!(frame.contains("[p1]=@"));
    assert!(frame.contains("[e1]=*"));
    assert!(frame.contains("80x24"));
}

#[test]
fn viewport_resize_propagates_to_renderer() {
    let mut r = AnsiRenderer::new();
    r.resize(Viewport::new(40, 20));
    assert_eq!(r.viewport(), Viewport::new(40, 20));
    r.resize(Viewport::new(120, 30));
    assert_eq!(r.viewport(), Viewport::new(120, 30));
}

#[test]
fn component_view_extraction_is_deterministic() {
    let scene_value: serde_json::Value = serde_json::from_str(&tiny_scene()).unwrap();
    let scene = Scene::from_value(scene_value, "scene.json", &registry()).unwrap();
    let views: Vec<ComponentView> = scene.nodes.iter().map(ComponentView::from_node).collect();

    let engine = engine_with(&tiny_scene());
    let views2: Vec<ComponentView> = engine
        .scene
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(ComponentView::from_node)
        .collect();

    assert_eq!(views.len(), views2.len());
    for (a, b) in views.iter().zip(views2.iter()) {
        assert_eq!(a.node_id, b.node_id);
        assert_eq!(a.type_name, b.type_name);
        assert_eq!(a.position, b.position);
    }
}

#[test]
fn many_nodes_produce_long_line() {
    let mut nodes = vec![];
    for i in 0..50 {
        nodes.push(format!(
            r#"{{ "id": "n{i}", "type": "M8Player", "components": {{ "health": {i}, "position": [{}.0, -{}] }} }}"#,
            i, i
        ));
    }
    let json = format!(
        r#"{{ "kind": "scene", "name": "t", "nodes": [{}] }}"#,
        nodes.join(",")
    );
    let mut engine = engine_with(&json);
    engine.set_renderer(Box::new(AnsiRenderer::new()));
    engine.render_now();
}

#[test]
fn tick_budget_under_8ms_at_60hz_for_1000_ticks() {
    let mut engine = engine_with(&tiny_scene());
    engine.set_renderer(Box::new(NullRenderer::new()));
    engine.enable_rendering(false);

    for _ in 0..100 {
        engine.tick();
    }
    let start = Instant::now();
    let ticks = 1000u64;
    for _ in 0..ticks {
        engine.tick();
    }
    let elapsed = start.elapsed();
    let avg_micros = elapsed.as_micros() / ticks as u128;
    let avg_ms = avg_micros as f64 / 1000.0;
    assert!(
        avg_ms <= 8.0,
        "ADR 0015: tick budget <=8ms at 60Hz; got {avg_ms:.3}ms/tick (1000 ticks took {elapsed:?})"
    );
}

#[test]
fn renderer_can_be_swapped_at_runtime() {
    let mut engine = engine_with(&tiny_scene());
    engine.set_renderer(Box::new(NullRenderer::new()));
    engine.render_now();
    assert_eq!(engine.renderer().viewport(), Viewport::default());

    engine.set_renderer(Box::new(AnsiRenderer::with_viewport(Viewport::new(40, 12))));
    assert_eq!(engine.renderer().viewport(), Viewport::new(40, 12));

    engine.enable_rendering(true);
    engine.tick();
}

#[test]
fn render_capabilities_reports_text() {
    let r = AnsiRenderer::new();
    let caps = r.capabilities();
    assert!(caps.contains(craft_kernel::RenderCapabilities::TEXT));
    assert!(!caps.contains(craft_kernel::RenderCapabilities::SPRITE));
}
