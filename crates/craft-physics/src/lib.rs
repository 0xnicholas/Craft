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
    use craft_kernel::scene::{
        Component, ComponentKind, ComponentValue, Node, Scene,
    };

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
        engine
            .bus
            .subscribe(signal_id, move |_| {
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
        assert_eq!(
            *hit_count.borrow(),
            0,
            "signal fires next tick (ADR 0003)"
        );

        engine.tick();
        assert_eq!(*hit_count.borrow(), 1, "collision signal delivered");
    }

    #[test]
    fn non_overlapping_aabbs_no_collision() {
        let mut engine = Engine::new();
        let signal_id = engine.bus.declare("collide");
        let hit_count = Rc::new(RefCell::new(0u32));
        let hit_count_clone = hit_count.clone();
        engine
            .bus
            .subscribe(signal_id, move |_| {
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
}
