use craft_kernel::craft_node;

pub use craft_physics;
pub use craft_particles;
pub use craft_audio;

craft_node!(Player, {
    components: {
        position: Vec2 = [0.0, 0.0],
        velocity: Vec2 = [0.0, 0.0],
        health: Int = 100,
        damage: Int = 10,
        hitbox: Vec2 = [0.5, 0.5],
        move_counter: Int = 0,
    },
});

craft_node!(Enemy, {
    components: {
        position: Vec2 = [0.0, 0.0],
        health: Int = 30,
        damage: Int = 5,
        hitbox: Vec2 = [0.5, 0.5],
    },
});

craft_node!(Wall, {
    components: {
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(HealthPotion, {
    components: {
        position: Vec2 = [0.0, 0.0],
        heal_amount: Int = 20,
        hitbox: Vec2 = [0.3, 0.3],
    },
});

craft_node!(Exit, {
    components: {
        position: Vec2 = [0.0, 0.0],
        hitbox: Vec2 = [0.5, 0.5],
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
    r.register::<Enemy>();
    r.register::<Wall>();
    r.register::<HealthPotion>();
    r.register::<Exit>();
    r.register::<Input>();
    r
}

pub const SCENE_JSON: &str = include_str!("../scene.json");

pub fn load_scene() -> craft_kernel::EngineResult<craft_kernel::Scene> {
    let registry = build_node_registry();
    craft_kernel::Scene::parse(SCENE_JSON, "scene.json", &registry)
}
