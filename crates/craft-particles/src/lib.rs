use craft_kernel::scene::ComponentValue;

craft_kernel::craft_system!(ParticleSystem, phase: PostTick, {
    let Some(scene) = ctx.scene.as_mut() else {
        return;
    };
    let dt = 1.0;

    // Phase 1a: collect emitter data (read-only pass)
    struct EmitterData {
        id: String,
        pos: [f64; 2],
        vel: [f64; 2],
        color: [f64; 3],
        size: Option<f64>,
        per_burst: usize,
        p_lifetime: i64,
    }
    let mut emitters_to_spawn: Vec<EmitterData> = Vec::new();
    for node in &mut scene.nodes {
        if node.destroyed || node.type_name != "ParticleEmitter" {
            continue;
        }
        let mut should_destroy = false;
        if let Some(comp) = node.components.get_mut("emitter_lifetime")
            && let ComponentValue::Int(ref mut i) = comp.value
        {
            *i -= 1;
            if *i <= 0 {
                should_destroy = true;
            }
        }
        if should_destroy {
            node.mark_destroyed();
            continue;
        }
        let mut should_spawn = false;
        let emit_rate = node
            .components
            .get("emit_rate")
            .and_then(|c| match &c.value {
                ComponentValue::Int(i) => Some(*i),
                _ => None,
            })
            .unwrap_or(1);
        if let Some(comp) = node.components.get_mut("cooldown")
            && let ComponentValue::Int(ref mut i) = comp.value
        {
            *i -= 1;
            if *i <= 0 {
                should_spawn = true;
                *i = emit_rate;
            }
        }
        if should_spawn {
            let per_burst = node
                .components
                .get("particles_per_burst")
                .and_then(|c| match &c.value {
                    ComponentValue::Int(i) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(8);
            let p_lifetime = node
                .components
                .get("particle_lifetime")
                .and_then(|c| match &c.value {
                    ComponentValue::Int(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(20);
            let pos = node
                .components
                .get("position")
                .and_then(|c| match &c.value {
                    ComponentValue::Vec2(v) => Some(*v),
                    _ => None,
                })
                .unwrap_or([0.0, 0.0]);
            let vel = node
                .components
                .get("velocity")
                .and_then(|c| match &c.value {
                    ComponentValue::Vec2(v) => Some(*v),
                    _ => None,
                })
                .unwrap_or([0.0, 0.0]);
            let color = node
                .components
                .get("modulate")
                .and_then(|c| match &c.value {
                    ComponentValue::Vec3(v) => Some(*v),
                    _ => None,
                })
                .unwrap_or([1.0, 0.5, 0.0]);
            let size = node.components.get("size").and_then(|c| match &c.value {
                ComponentValue::Float(f) => Some(*f),
                _ => None,
            });
            emitters_to_spawn.push(EmitterData {
                id: node.id.clone(),
                pos,
                vel,
                color,
                size,
                per_burst,
                p_lifetime,
            });
        }
    }

    // Phase 1b: spawn particles from collected emitter data
    for ed in &emitters_to_spawn {
        for i in 0..ed.per_burst {
            let angle = (i as f64) * std::f64::consts::TAU / (ed.per_burst as f64);
            let speed = 1.5;
            let pvx = ed.vel[0] + angle.cos() * speed;
            let pvy = ed.vel[1] + angle.sin() * speed;
            let pid = format!("__particle_{}_{}", ed.id, scene.spawn_counter);
            scene.spawn_counter += 1;
            let mut comps = std::collections::BTreeMap::new();
            comps.insert(
                "position".to_string(),
                craft_kernel::scene::Component {
                    value: ComponentValue::Vec2(ed.pos),
                    kind: Default::default(),
                },
            );
            comps.insert(
                "velocity".to_string(),
                craft_kernel::scene::Component {
                    value: ComponentValue::Vec2([pvx, pvy]),
                    kind: Default::default(),
                },
            );
            comps.insert(
                "lifetime".to_string(),
                craft_kernel::scene::Component {
                    value: ComponentValue::Int(ed.p_lifetime),
                    kind: Default::default(),
                },
            );
            comps.insert(
                "max_lifetime".to_string(),
                craft_kernel::scene::Component {
                    value: ComponentValue::Int(ed.p_lifetime),
                    kind: Default::default(),
                },
            );
            comps.insert(
                "modulate".to_string(),
                craft_kernel::scene::Component {
                    value: ComponentValue::Vec3(ed.color),
                    kind: Default::default(),
                },
            );
            if let Some(sz) = ed.size {
                comps.insert(
                    "size".to_string(),
                    craft_kernel::scene::Component {
                        value: ComponentValue::Float(sz),
                        kind: Default::default(),
                    },
                );
            }
            scene.nodes.push(craft_kernel::Node {
                id: pid,
                type_name: "Particle".to_string(),
                parent: Some(ed.id.clone()),
                components: comps,
                behaviors: vec![],
                active_state: None,
                lua_class: None,
                destroyed: false,
            });
        }
    }

    // Phase 2: update particles
    for node in &mut scene.nodes {
        if node.destroyed || node.type_name != "Particle" {
            continue;
        }
        let vel = node
            .components
            .get("velocity")
            .and_then(|c| match &c.value {
                ComponentValue::Vec2(v) => Some(*v),
                _ => None,
            });
        if let Some([vx, vy]) = vel
            && let Some(pos) = node.components.get_mut("position")
            && let ComponentValue::Vec2(ref mut p) = pos.value
        {
            p[0] += vx * dt;
            p[1] += vy * dt;
        }
        let max_lt = node
            .components
            .get("max_lifetime")
            .and_then(|c| match &c.value {
                ComponentValue::Int(i) => Some(*i),
                _ => None,
            });
        let mut expired = false;
        if let Some(comp) = node.components.get_mut("lifetime")
            && let ComponentValue::Int(ref mut i) = comp.value
        {
            *i -= 1;
            if *i <= 0 {
                expired = true;
            }
        }
        if expired {
            node.mark_destroyed();
        } else if let Some(max) = max_lt {
            let current_lt = node
                .components
                .get("lifetime")
                .and_then(|c| match &c.value {
                    ComponentValue::Int(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);
            let alpha = if max > 0 {
                current_lt as f64 / max as f64
            } else {
                0.0
            };
            node.components.insert(
                "alpha".to_string(),
                craft_kernel::scene::Component {
                    value: ComponentValue::Float(alpha),
                    kind: Default::default(),
                },
            );
        }
    }

    scene.purge_destroyed();
});

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use craft_kernel::engine::Engine;
    use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};

    fn make_node(id: &str, ty: &str, comps: &[(&str, ComponentValue)]) -> Node {
        let mut map = BTreeMap::new();
        for (k, v) in comps {
            map.insert(
                k.to_string(),
                Component {
                    value: v.clone(),
                    kind: ComponentKind::Regular,
                },
            );
        }
        Node {
            id: id.to_string(),
            type_name: ty.to_string(),
            parent: None,
            components: map,
            behaviors: vec![],
            active_state: None,
            lua_class: None,
            destroyed: false,
        }
    }

    #[test]
    fn emitter_spawns_particles() {
        let mut engine = Engine::new();
        let scene = Scene {
            kind: "scene".to_string(),
            name: "p".to_string(),
            nodes: vec![make_node(
                "em1",
                "ParticleEmitter",
                &[
                    ("position", ComponentValue::Vec2([0.0, 0.0])),
                    ("emit_rate", ComponentValue::Int(5)),
                    ("particles_per_burst", ComponentValue::Int(4)),
                    ("particle_lifetime", ComponentValue::Int(15)),
                    ("emitter_lifetime", ComponentValue::Int(10)),
                    ("cooldown", ComponentValue::Int(0)),
                    ("modulate", ComponentValue::Vec3([1.0, 0.3, 0.0])),
                ],
            )],
            spawn_counter: 0,
        };
        engine.load_scene(scene);
        engine.tick();

        let particles: Vec<_> = engine
            .scene()
            .unwrap()
            .nodes
            .iter()
            .filter(|n| n.type_name == "Particle")
            .collect();
        assert_eq!(particles.len(), 4, "first burst: 4 particles");
        assert!(particles
            .iter()
            .all(|p| p.components.contains_key("velocity")));
    }

    #[test]
    fn emitter_destroys_after_lifetime() {
        let mut engine = Engine::new();
        let scene = Scene {
            kind: "scene".to_string(),
            name: "p".to_string(),
            nodes: vec![make_node(
                "em1",
                "ParticleEmitter",
                &[
                    ("position", ComponentValue::Vec2([0.0, 0.0])),
                    ("emit_rate", ComponentValue::Int(99)),
                    ("particles_per_burst", ComponentValue::Int(1)),
                    ("particle_lifetime", ComponentValue::Int(10)),
                    ("emitter_lifetime", ComponentValue::Int(3)),
                    ("cooldown", ComponentValue::Int(0)),
                ],
            )],
            spawn_counter: 0,
        };
        engine.load_scene(scene);

        for _ in 0..4 {
            engine.tick();
        }

        let emitters: Vec<_> = engine
            .scene()
            .unwrap()
            .nodes
            .iter()
            .filter(|n| n.id == "em1")
            .collect();
        assert!(emitters.is_empty(), "emitter should be destroyed");
    }

    #[test]
    fn particles_move_and_expire() {
        let mut engine = Engine::new();
        let scene = Scene {
            kind: "scene".to_string(),
            name: "p".to_string(),
            nodes: vec![make_node(
                "em1",
                "ParticleEmitter",
                &[
                    ("position", ComponentValue::Vec2([0.0, 0.0])),
                    ("emit_rate", ComponentValue::Int(99)),
                    ("particles_per_burst", ComponentValue::Int(2)),
                    ("particle_lifetime", ComponentValue::Int(5)),
                    ("emitter_lifetime", ComponentValue::Int(5)),
                    ("cooldown", ComponentValue::Int(0)),
                ],
            )],
            spawn_counter: 0,
        };
        engine.load_scene(scene);
        engine.tick();

        let particles: Vec<_> = engine
            .scene()
            .unwrap()
            .nodes
            .iter()
            .filter(|n| n.type_name == "Particle")
            .collect();
        assert_eq!(particles.len(), 2);
        let pos = particles[0]
            .components
            .get("position")
            .and_then(|c| match &c.value {
                ComponentValue::Vec2(v) => Some(*v),
                _ => None,
            })
            .unwrap();
        assert_ne!(pos, [0.0, 0.0], "particles should have moved");

        for _ in 0..10 {
            engine.tick();
        }
        let remaining: Vec<_> = engine
            .scene()
            .unwrap()
            .nodes
            .iter()
            .filter(|n| n.type_name == "Particle")
            .collect();
        assert!(remaining.is_empty(), "all particles should be expired");
    }
}
