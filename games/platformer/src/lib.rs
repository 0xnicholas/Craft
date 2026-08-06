use craft_kernel::craft_node;

pub use craft_physics;

craft_node!(Player, {
    components: {
        position: Vec2 = [0.0, 0.0],
        velocity: Vec2 = [0.0, 0.0],
        hitbox: Vec2 = [0.4, 0.4],
        grounded: Bool = false,
        won: Bool = false,
    },
});

craft_node!(Goal, {
    components: {
        position: Vec2 = [0.0, 0.0],
        hitbox: Vec2 = [0.5, 0.5],
    },
});

craft_node!(Level, {
    components: {
        tilemap: String = "",
        tile_size: Float = 1.0,
    },
});

craft_node!(Input, {
    components: {
        direction: Vec2 = [0.0, 0.0],
        action: Bool = false,
    },
});

pub fn build_node_registry() -> craft_kernel::NodeRegistry {
    let mut r = craft_kernel::NodeRegistry::new();
    r.register::<Player>();
    r.register::<Goal>();
    r.register::<Level>();
    r.register::<Input>();
    r
}

pub const SCENE_JSON: &str = include_str!("../scene.json");

pub fn load_scene() -> craft_kernel::EngineResult<craft_kernel::Scene> {
    let registry = build_node_registry();
    craft_kernel::Scene::parse(SCENE_JSON, "scene.json", &registry)
}
