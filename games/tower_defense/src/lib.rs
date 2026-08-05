use craft_kernel::craft_node;

pub use craft_audio;
pub use craft_particles;
pub use craft_physics;

craft_node!(Spawner, {
    components: {
        cooldown: Int = 0,
        spawn_interval: Int = 20,
        spawned_count: Int = 0,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(Tower, {
    components: {
        cooldown: Int = 0,
        fire_rate: Int = 10,
        shots_fired: Int = 0,
        damage: Int = 5,
        kills: Int = 0,
        position: Vec2 = [10.0, 0.0],
    },
});

craft_node!(Enemy, {
    components: {
        health: Int = 5,
        lifetime: Int = 200,
        speed: Float = 0.5,
        position: Vec2 = [0.0, 0.0],
        velocity: Vec2 = [0.0, 0.0],
        hitbox: Vec2 = [1.0, 1.0],
    },
});

craft_node!(Camera2D, {
    components: {
        position: Vec2 = [0.0, 0.0],
        zoom: Float = 1.0,
    },
});

craft_node!(Projectile, {
    components: {
        position: Vec2 = [0.0, 0.0],
        velocity: Vec2 = [0.0, 0.0],
        hitbox: Vec2 = [0.5, 0.5],
        damage: Int = 5,
        lifetime: Int = 40,
    },
});

pub fn build_node_registry() -> craft_kernel::NodeRegistry {
    let mut r = craft_kernel::NodeRegistry::new();
    r.register::<Spawner>();
    r.register::<Tower>();
    r.register::<Enemy>();
    r.register::<Camera2D>();
    r.register::<Projectile>();
    r
}

pub const SCENE_JSON: &str = include_str!("../scene.json");

pub fn load_scene() -> craft_kernel::EngineResult<craft_kernel::Scene> {
    let registry = build_node_registry();
    craft_kernel::Scene::parse(SCENE_JSON, "scene.json", &registry)
}
