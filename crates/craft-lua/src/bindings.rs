use mlua::UserData;
use mlua::prelude::*;

use craft_kernel::{Component, ComponentKind, ComponentValue, Node};

use crate::runtime::{SceneHandle, component_value_to_lua, lua_to_component_value};

#[derive(Clone)]
pub(crate) struct NodeRef {
    pub id: String,
    pub scene: SceneHandle,
}

impl NodeRef {
    pub(crate) fn new(id: String, scene: SceneHandle) -> Self {
        Self { id, scene }
    }
}

impl UserData for NodeRef {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, node| Ok(node.id.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, node, key: String| {
            let value_opt = node.scene.with_ref(|s| {
                s.find_node(&node.id)
                    .and_then(|target| target.get_component_value(&key).cloned())
            });
            match value_opt {
                Some(value) => component_value_to_lua(lua, &value),
                None => node.scene.with_ref(|s| {
                    if s.find_node(&node.id).is_some() {
                        Ok(LuaValue::Nil)
                    } else {
                        Err(LuaError::external(format!(
                            "node \"{}\" no longer exists",
                            node.id
                        )))
                    }
                }),
            }
        });

        methods.add_meta_method(
            LuaMetaMethod::NewIndex,
            |_, node, (key, value): (String, LuaValue)| {
                let cv = lua_to_component_value(value)?;
                node.scene.with_mut(|s| {
                    let Some(target) = s.find_node_mut(&node.id) else {
                        return Err(LuaError::external(format!(
                            "node \"{}\" no longer exists",
                            node.id
                        )));
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
            },
        );

        methods.add_method("destroy", |_, node, ()| {
            node.scene.with_mut(|s| s.nodes.retain(|n| n.id != node.id));
            Ok(())
        });

        methods.add_method("has_component", |_, node, key: String| {
            let has = node.scene.with_ref(|s| match s.find_node(&node.id) {
                Some(target) => target.components.contains_key(&key),
                None => false,
            });
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
    }
}
