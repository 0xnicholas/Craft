use craft_kernel::scene::ComponentValue;

craft_kernel::craft_system!(AnimationSystem, phase: PreTick, {
    let Some(scene) = ctx.scene.as_mut() else {
        return;
    };
    for node in &mut scene.nodes {
        if node.destroyed {
            continue;
        }
        let Some(uri) = node.components.get("animation").and_then(|c| match &c.value {
            ComponentValue::String(s) => Some(s.clone()),
            _ => None,
        }) else {
            continue;
        };
        let Some(res) = ctx
            .resources
            .resolve(&uri)
            .and_then(|id| ctx.resources.get(id))
        else {
            continue;
        };
        let Some(frames) = res.data.get("frames").and_then(|v| v.as_array()) else {
            continue;
        };
        if frames.is_empty() {
            continue;
        }
        let frame_count = frames.len() as i64;
        let fps = res
            .data
            .get("fps")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .max(0.001);
        let looping = res
            .data
            .get("loop")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let ticks_per_frame = (60.0 / fps).ceil().max(1.0) as u64;

        let frame = node
            .components
            .get("frame")
            .and_then(|c| match &c.value {
                ComponentValue::Int(i) => Some(*i),
                _ => None,
            })
            .unwrap_or(0);
        let frame_time = node
            .components
            .get("frame_time")
            .and_then(|c| match &c.value {
                ComponentValue::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(0.0);

        let at_last_frame = frame >= frame_count - 1;
        let (next_frame, next_time) = if at_last_frame && !looping {
            (frame, frame_time)
        } else if frame_time + 1.0 >= ticks_per_frame as f64 {
            let next = frame + 1;
            if looping {
                (next % frame_count, 0.0)
            } else {
                (next.min(frame_count - 1), 0.0)
            }
        } else {
            (frame, frame_time + 1.0)
        };

        node.components.insert(
            "frame".to_string(),
            craft_kernel::Component {
                value: ComponentValue::Int(next_frame),
                kind: Default::default(),
            },
        );
        node.components.insert(
            "frame_time".to_string(),
            craft_kernel::Component {
                value: ComponentValue::Float(next_time),
                kind: Default::default(),
            },
        );
        if let Some(rect) = frames[next_frame as usize].as_array() {
            let get = |i: usize| rect.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            node.components.insert(
                "sprite_rect".to_string(),
                craft_kernel::Component {
                    value: ComponentValue::Rect([get(0), get(1), get(2), get(3)]),
                    kind: Default::default(),
                },
            );
        }
    }
});
