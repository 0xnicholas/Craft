use mlua::UserData;
use mlua::prelude::*;

use craft_kernel::{Component, ComponentKind, ComponentValue, Node};

use crate::runtime::{SceneHandle, component_value_to_lua, lua_to_component_value};

#[derive(Clone)]
pub(crate) struct NodeRef {
    pub id: String,
    pub scene: SceneHandle,
    pub current_generation: std::rc::Rc<std::cell::RefCell<u64>>,
}

impl NodeRef {
    pub(crate) fn new(
        id: String,
        scene: SceneHandle,
        current_generation: std::rc::Rc<std::cell::RefCell<u64>>,
    ) -> Self {
        Self {
            id,
            scene,
            current_generation,
        }
    }
}

impl UserData for NodeRef {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, node| Ok(node.id.clone()));
        fields.add_field_method_set("id", |_, _node: &mut NodeRef, _value: LuaValue| {
            Err(LuaError::external(
                "node id is read-only; spawn a new node to get a different id",
            ))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, node, key: String| {
            let current_gen = *node.current_generation.borrow();
            let lookup = node.scene.with_ref(current_gen, |s| {
                s.find_node(&node.id)
                    .and_then(|target| target.get_component_value(&key).cloned())
            });
            match lookup {
                Ok(Some(value)) => component_value_to_lua(lua, &value),
                Ok(None) => {
                    let exists = node
                        .scene
                        .with_ref(current_gen, |s| s.find_node(&node.id).is_some());
                    match exists {
                        Ok(true) => Ok(LuaValue::Nil),
                        Ok(false) => Err(LuaError::external(format!(
                            "node \"{}\" no longer exists",
                            node.id
                        ))),
                        Err(e) => Err(LuaError::external(e)),
                    }
                }
                Err(e) => Err(LuaError::external(e)),
            }
        });

        methods.add_meta_method(
            LuaMetaMethod::NewIndex,
            |_, node, (key, value): (String, LuaValue)| {
                let cv = lua_to_component_value(value)?;
                let current_gen = *node.current_generation.borrow();
                node.scene
                    .with_mut(current_gen, |s| {
                        let Some(target) = s.find_node_mut(&node.id) else {
                            return Err(format!("node \"{}\" no longer exists", node.id));
                        };
                        match target.components.entry(key) {
                            std::collections::btree_map::Entry::Occupied(mut e) => {
                                e.get_mut().value = cv;
                            }
                            std::collections::btree_map::Entry::Vacant(e) => {
                                e.insert(Component {
                                    value: cv,
                                    kind: ComponentKind::Regular,
                                });
                            }
                        }
                        Ok(())
                    })
                    .map_err(LuaError::external)
            },
        );

        methods.add_method("destroy", |_, node, ()| {
            let current_gen = *node.current_generation.borrow();
            let _ = node
                .scene
                .with_mut(current_gen, |s| {
                    if let Some(target) = s.find_node_mut_raw(&node.id) {
                        target.mark_destroyed();
                    }
                    Ok::<(), String>(())
                })
                .map_err(LuaError::external)?;
            Ok(())
        });

        methods.add_method("has_component", |_, node, key: String| {
            let current_gen = *node.current_generation.borrow();
            let has = node
                .scene
                .with_ref(current_gen, |s| match s.find_node(&node.id) {
                    Some(target) => target.components.contains_key(&key),
                    None => false,
                })
                .unwrap_or(false);
            Ok(has)
        });

        methods.add_method("__tostring", |_, node, ()| Ok(format!("Node({})", node.id)));
    }
}

pub(crate) fn build_node(
    type_name: &str,
    components: Vec<(String, ComponentValue)>,
    id: String,
) -> Node {
    let mut map = std::collections::BTreeMap::new();
    for (k, v) in components {
        map.insert(
            k,
            Component {
                value: v,
                kind: ComponentKind::Regular,
            },
        );
    }
    Node {
        id,
        type_name: type_name.to_string(),
        parent: None,
        components: map,
        behaviors: Vec::new(),
        active_state: None,
        destroyed: false,
    }
}
