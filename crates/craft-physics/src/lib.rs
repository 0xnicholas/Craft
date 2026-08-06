use craft_kernel::scene::ComponentValue;

craft_kernel::craft_system!(PhysicsSystem, phase: PreTick, {
    let Some(scene) = ctx.scene.as_mut() else {
        return;
    };
    let dt = 1.0;

    for node in &mut scene.nodes {
        if node.destroyed {
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
    }

    // Tile collision: resolve moving nodes against the level's solid
    // grid. The level node carries a `tilemap` component pointing at a
    // `res://` resource `{ width, height, tiles }`; out-of-bounds cells
    // are solid except above the level, so jumps are unclamped.
    // Resolution is swept (old edge -> new edge) and axis-separated
    // (Y then X, platformer order: land before walls), scanning cells in
    // row-major order — deterministic by construction and immune to
    // tunneling at high speeds.
    let level = scene.nodes.iter().find_map(|n| {
        let uri = n.components.get("tilemap").and_then(|c| match &c.value {
            ComponentValue::String(s) => Some(s.clone()),
            _ => None,
        })?;
        let tile_size = n
            .components
            .get("tile_size")
            .and_then(|c| match &c.value {
                ComponentValue::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(1.0);
        Some((uri, tile_size))
    });
    if let Some((uri, tile_size)) = level {
        let grid = ctx
            .resources
            .resolve(&uri)
            .and_then(|id| ctx.resources.get(id));
        if let Some(res) = grid
            && let (Some(width), Some(height), Some(tiles)) = (
                res.data.get("width").and_then(|v| v.as_u64()),
                res.data.get("height").and_then(|v| v.as_u64()),
                res.data.get("tiles").and_then(|v| v.as_array()),
            )
        {
            let is_solid = |col: i64, row: i64| -> bool {
                if col < 0 || col >= width as i64 || row >= height as i64 {
                    return true;
                }
                if row < 0 {
                    return false;
                }
                tiles[(row * width as i64 + col) as usize]
                    .as_i64()
                    .map(|t| t != 0)
                    .unwrap_or(false)
            };
            for node in &mut scene.nodes {
                if node.destroyed {
                    continue;
                }
                let Some([vx, vy]) = node
                    .components
                    .get("velocity")
                    .and_then(|c| match &c.value {
                        ComponentValue::Vec2(v) => Some(*v),
                        _ => None,
                    })
                else {
                    continue;
                };
                if vx == 0.0 && vy == 0.0 {
                    continue;
                }
                let Some([hx, hy]) = node
                    .components
                    .get("hitbox")
                    .and_then(|c| match &c.value {
                        ComponentValue::Vec2(v) => Some(*v),
                        _ => None,
                    })
                else {
                    continue;
                };
                let Some(ComponentValue::Vec2(p)) =
                    node.components.get("position").map(|c| &c.value)
                else {
                    continue;
                };
                let mut px = p[0];
                let mut py = p[1];
                let mut nvx = vx;
                let mut nvy = vy;
                if nvy != 0.0 {
                    let x0 = ((px - hx) / tile_size).floor() as i64;
                    let x1 = (((px + hx) / tile_size).ceil() as i64) - 1;
                    if nvy > 0.0 {
                        let r0 = ((py - nvy + hy) / tile_size).floor() as i64;
                        let r1 = (((py + hy) / tile_size).ceil() as i64) - 1;
                        if let Some(r) =
                            (r0..=r1).find(|&r| (x0..=x1).any(|c| is_solid(c, r)))
                        {
                            py = r as f64 * tile_size - hy;
                            nvy = 0.0;
                        }
                    } else {
                        let r0 = ((py - hy) / tile_size).floor() as i64;
                        let r1 = (((py - nvy - hy) / tile_size).ceil() as i64) - 1;
                        if let Some(r) = (r0..=r1)
                            .rev()
                            .find(|&r| (x0..=x1).any(|c| is_solid(c, r)))
                        {
                            py = (r + 1) as f64 * tile_size + hy;
                            nvy = 0.0;
                        }
                    }
                }
                if nvx != 0.0 {
                    let y0 = ((py - hy) / tile_size).floor() as i64;
                    let y1 = (((py + hy) / tile_size).ceil() as i64) - 1;
                    if nvx > 0.0 {
                        let c0 = ((px - nvx + hx) / tile_size).floor() as i64;
                        let c1 = (((px + hx) / tile_size).ceil() as i64) - 1;
                        if let Some(c) =
                            (c0..=c1).find(|&c| (y0..=y1).any(|r| is_solid(c, r)))
                        {
                            px = c as f64 * tile_size - hx;
                            nvx = 0.0;
                        }
                    } else {
                        let c0 = ((px - hx) / tile_size).floor() as i64;
                        let c1 = (((px - nvx - hx) / tile_size).ceil() as i64) - 1;
                        if let Some(c) = (c0..=c1)
                            .rev()
                            .find(|&c| (y0..=y1).any(|r| is_solid(c, r)))
                        {
                            px = (c + 1) as f64 * tile_size + hx;
                            nvx = 0.0;
                        }
                    }
                }
                if let Some(pos) = node.components.get_mut("position")
                    && let ComponentValue::Vec2(ref mut p) = pos.value
                {
                    p[0] = px;
                    p[1] = py;
                }
                if let Some(vel) = node.components.get_mut("velocity")
                    && let ComponentValue::Vec2(ref mut v) = vel.value
                {
                    v[0] = nvx;
                    v[1] = nvy;
                }
            }
        }
    }

    let mut collisions: Vec<(String, String)> = Vec::new();
    let positions: Vec<(String, (f64, f64))> = scene
        .nodes
        .iter()
        .filter(|n| !n.destroyed)
        .filter_map(|n| {
            n.components.get("position").and_then(|c| match &c.value {
                ComponentValue::Vec2(v) => Some((n.id.clone(), (v[0], v[1]))),
                _ => None,
            })
        })
        .collect();
    let hitboxes: Vec<(String, Option<[f64; 2]>, Option<f64>)> = scene
        .nodes
        .iter()
        .filter(|n| !n.destroyed)
        .map(|n| {
            let aabb = n.components.get("hitbox").and_then(|c| match &c.value {
                ComponentValue::Vec2(v) => Some(*v),
                _ => None,
            });
            let radius = n
                .components
                .get("hitbox_radius")
                .and_then(|c| match &c.value {
                    ComponentValue::Float(f) => Some(*f),
                    _ => None,
                });
            (n.id.clone(), aabb, radius)
        })
        .collect();

    for (i, (id_a, aabb_a, radius_a)) in hitboxes.iter().enumerate() {
        let pos_a = positions
            .iter()
            .find(|(id, _)| id == id_a)
            .map(|(_, p)| p)
            .cloned();
        for (id_b, aabb_b, radius_b) in hitboxes.iter().skip(i + 1) {
            let pos_b = positions
                .iter()
                .find(|(id, _)| id == id_b)
                .map(|(_, p)| p)
                .cloned();
            let collides = match (pos_a, pos_b, aabb_a, aabb_b, radius_a, radius_b) {
                (Some(pa), Some(pb), Some(ha), Some(hb), None, None) => {
                    aabb_vs_aabb(pa, *ha, pb, *hb)
                }
                (Some(pa), Some(pb), None, None, Some(ra), Some(rb)) => {
                    circle_vs_circle(pa, *ra, pb, *rb)
                }
                (Some(pa), Some(pb), Some(ha), None, Some(rb), None) => {
                    aabb_vs_circle(pa, *ha, pb, *rb)
                }
                (Some(pa), Some(pb), None, Some(hb), Some(ra), None) => {
                    aabb_vs_circle(pb, *hb, pa, *ra)
                }
                _ => false,
            };
            if collides {
                collisions.push((id_a.clone(), id_b.clone()));
            }
        }
    }

    for (a_id, b_id) in collisions {
        let payload = serde_json::json!({
            "a": a_id,
            "b": b_id,
        });
        ctx.pending_signals.push(("collide".to_string(), payload));
    }
});

fn aabb_vs_aabb(pa: (f64, f64), ha: [f64; 2], pb: (f64, f64), hb: [f64; 2]) -> bool {
    (pa.0 - ha[0]) < (pb.0 + hb[0])
        && (pa.0 + ha[0]) > (pb.0 - hb[0])
        && (pa.1 - ha[1]) < (pb.1 + hb[1])
        && (pa.1 + ha[1]) > (pb.1 - hb[1])
}

fn circle_vs_circle(pa: (f64, f64), ra: f64, pb: (f64, f64), rb: f64) -> bool {
    let dx = pa.0 - pb.0;
    let dy = pa.1 - pb.1;
    let dist_sq = dx * dx + dy * dy;
    let r_sum = ra + rb;
    dist_sq < r_sum * r_sum
}

fn aabb_vs_circle(p_box: (f64, f64), half: [f64; 2], p_circle: (f64, f64), r: f64) -> bool {
    let closest_x = p_circle.0.clamp(p_box.0 - half[0], p_box.0 + half[0]);
    let closest_y = p_circle.1.clamp(p_box.1 - half[1], p_box.1 + half[1]);
    let dx = p_circle.0 - closest_x;
    let dy = p_circle.1 - closest_y;
    dx * dx + dy * dy < r * r
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use craft_kernel::engine::Engine;
    use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};

    fn test_node(id: &str, type_name: &str, comps: &[(&str, ComponentValue)]) -> Node {
        let mut components = BTreeMap::new();
        for (k, v) in comps {
            components.insert(
                k.to_string(),
                Component {
                    value: v.clone(),
                    kind: ComponentKind::Regular,
                },
            );
        }
        Node {
            id: id.to_string(),
            type_name: type_name.to_string(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
            lua_class: None,
            destroyed: false,
        }
    }

    #[test]
    fn system_is_registered() {
        let engine = Engine::new();
        let names: Vec<&str> = engine.list_systems().iter().map(|s| s.name).collect();
        assert!(
            names.contains(&"PhysicsSystem"),
            "PhysicsSystem should be registered. Found: {names:?}"
        );
    }

    #[test]
    fn velocity_integrates_position() {
        let scene = Scene {
            kind: "scene".to_string(),
            name: "physics_test".to_string(),
            nodes: vec![test_node(
                "bullet",
                "Projectile",
                &[
                    ("position", ComponentValue::Vec2([0.0, 0.0])),
                    ("velocity", ComponentValue::Vec2([2.0, 3.0])),
                ],
            )],
            spawn_counter: 0,
        };

        let mut engine = Engine::new();
        engine.load_scene(scene);
        engine.tick();

        let pos = engine
            .scene()
            .unwrap()
            .nodes
            .first()
            .unwrap()
            .components
            .get("position")
            .unwrap()
            .value
            .clone();
        assert_eq!(pos, ComponentValue::Vec2([2.0, 3.0]));
    }

    #[test]
    fn aabb_collision_emits_signal() {
        let mut engine = Engine::new();
        let signal_id = engine.bus.declare("collide");
        let hit_count = Rc::new(RefCell::new(0u32));
        let hit_count_clone = hit_count.clone();
        engine.bus.subscribe(signal_id, move |_| {
            *hit_count_clone.borrow_mut() += 1;
        });

        let scene = Scene {
            kind: "scene".to_string(),
            name: "collision_test".to_string(),
            nodes: vec![
                test_node(
                    "a",
                    "Enemy",
                    &[
                        ("position", ComponentValue::Vec2([0.0, 0.0])),
                        ("hitbox", ComponentValue::Vec2([1.0, 1.0])),
                    ],
                ),
                test_node(
                    "b",
                    "Enemy",
                    &[
                        ("position", ComponentValue::Vec2([0.5, 0.5])),
                        ("hitbox", ComponentValue::Vec2([1.0, 1.0])),
                    ],
                ),
            ],
            spawn_counter: 0,
        };

        engine.load_scene(scene);
        engine.tick();
        assert_eq!(*hit_count.borrow(), 0, "signal fires next tick (ADR 0003)");

        engine.tick();
        assert_eq!(*hit_count.borrow(), 1, "collision signal delivered");
    }

    #[test]
    fn non_overlapping_aabbs_no_collision() {
        let mut engine = Engine::new();
        let signal_id = engine.bus.declare("collide");
        let hit_count = Rc::new(RefCell::new(0u32));
        let hit_count_clone = hit_count.clone();
        engine.bus.subscribe(signal_id, move |_| {
            *hit_count_clone.borrow_mut() += 1;
        });

        let scene = Scene {
            kind: "scene".to_string(),
            name: "no_collision".to_string(),
            nodes: vec![
                test_node(
                    "a",
                    "Tower",
                    &[
                        ("position", ComponentValue::Vec2([0.0, 0.0])),
                        ("hitbox", ComponentValue::Vec2([1.0, 1.0])),
                    ],
                ),
                test_node(
                    "b",
                    "Tower",
                    &[
                        ("position", ComponentValue::Vec2([10.0, 10.0])),
                        ("hitbox", ComponentValue::Vec2([1.0, 1.0])),
                    ],
                ),
            ],
            spawn_counter: 0,
        };

        engine.load_scene(scene);
        engine.tick();
        engine.tick();
        assert_eq!(*hit_count.borrow(), 0);
    }

    #[test]
    fn aabb_overlap_detected() {
        assert!(aabb_vs_aabb((0.0, 0.0), [1.0, 1.0], (1.5, 0.0), [1.0, 1.0]));
    }

    #[test]
    fn aabb_separated_not_colliding() {
        assert!(!aabb_vs_aabb(
            (0.0, 0.0),
            [1.0, 1.0],
            (3.0, 0.0),
            [1.0, 1.0]
        ));
    }

    #[test]
    fn circle_overlap_detected() {
        assert!(circle_vs_circle((0.0, 0.0), 1.0, (1.5, 0.0), 1.0));
    }

    #[test]
    fn circle_separated_not_colliding() {
        assert!(!circle_vs_circle((0.0, 0.0), 1.0, (3.0, 0.0), 1.0));
    }

    #[test]
    fn aabb_vs_circle_overlap() {
        assert!(aabb_vs_circle((0.0, 0.0), [1.0, 1.0], (1.4, 0.0), 0.5));
    }

    #[test]
    fn aabb_vs_circle_separated() {
        assert!(!aabb_vs_circle((0.0, 0.0), [0.5, 0.5], (2.0, 0.0), 0.5));
    }

    fn tile_engine(with_level: bool, solid_ground: bool) -> Engine {
        let mut engine = Engine::new();
        engine.resources.register(
            "res://levels/t1.json",
            craft_kernel::serde_json::json!({
                "width": 10,
                "height": 10,
                "tiles": (0..100)
                    .map(|i| if i / 10 == 9 && solid_ground { 1 } else { 0 })
                    .collect::<Vec<i64>>(),
            }),
        );
        let mut nodes = Vec::new();
        if with_level {
            nodes.push(Node {
                id: "level".to_string(),
                type_name: "Level".to_string(),
                parent: None,
                components: BTreeMap::from([
                    (
                        "tilemap".to_string(),
                        Component {
                            value: ComponentValue::String("res://levels/t1.json".into()),
                            kind: ComponentKind::Regular,
                        },
                    ),
                    (
                        "tile_size".to_string(),
                        Component {
                            value: ComponentValue::Float(1.0),
                            kind: ComponentKind::Regular,
                        },
                    ),
                ]),
                behaviors: vec![],
                active_state: None,
                lua_class: None,
                destroyed: false,
            });
        }
        nodes.push(test_node(
            "ball",
            "Ball",
            &[
                ("position", ComponentValue::Vec2([5.0, 1.0])),
                ("velocity", ComponentValue::Vec2([0.0, 0.0])),
                ("hitbox", ComponentValue::Vec2([0.4, 0.4])),
            ],
        ));
        let scene = Scene {
            kind: "scene".to_string(),
            name: "tile_test".to_string(),
            nodes,
            spawn_counter: 0,
        };
        engine.load_scene(scene);
        engine
    }

    fn set_ball_velocity(engine: &mut Engine, vy: f64) {
        engine
            .scene_mut()
            .unwrap()
            .find_node_mut("ball")
            .unwrap()
            .components
            .get_mut("velocity")
            .unwrap()
            .value = ComponentValue::Vec2([0.0, vy]);
    }

    #[test]
    fn falling_ball_rests_on_top_of_ground_tile() {
        let mut engine = tile_engine(true, true);
        for _ in 0..60 {
            set_ball_velocity(&mut engine, 8.0);
            engine.tick();
        }
        let ball = engine.scene().unwrap().find_node("ball").unwrap();
        let pos = match &ball.components.get("position").unwrap().value {
            ComponentValue::Vec2(v) => *v,
            other => panic!("unexpected position value {other:?}"),
        };
        let vel = match &ball.components.get("velocity").unwrap().value {
            ComponentValue::Vec2(v) => *v,
            other => panic!("unexpected velocity value {other:?}"),
        };
        assert_eq!(
            pos[1], 8.6,
            "high-speed fall must rest on top of ground row 9 (9.0 - half 0.4), no tunneling"
        );
        assert_eq!(vel[1], 0.0, "vertical velocity zeroed on landing");
    }

    #[test]
    fn ball_without_level_node_falls_through() {
        let mut engine = tile_engine(false, false);
        for _ in 0..10 {
            set_ball_velocity(&mut engine, 2.0);
            engine.tick();
        }
        let ball = engine.scene().unwrap().find_node("ball").unwrap();
        let pos = match &ball.components.get("position").unwrap().value {
            ComponentValue::Vec2(v) => *v,
            other => panic!("unexpected position value {other:?}"),
        };
        assert!(
            pos[1] > 15.0,
            "no level node: tile resolution must not run (got y={})",
            pos[1]
        );
    }
}
