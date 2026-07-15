use mlua::prelude::*;

/// Which lifecycle hooks a loaded Lua class implements. Detected at
/// `load_class` time by inspecting the class table. Used by the runtime
/// to skip empty hook calls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClassHooks {
    pub on_tick: bool,
    pub on_signal: bool,
    pub on_spawn: bool,
}

/// A registered Lua class: its source, name, and which hooks it defines.
/// The class table itself lives in the Lua VM; this struct is metadata
/// kept on the Rust side so we know which hooks to call without re-probing
/// the VM every tick.
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub source: String,
    pub hooks: ClassHooks,
}

pub(crate) fn probe_hooks(class_table: &LuaTable) -> LuaResult<ClassHooks> {
    Ok(ClassHooks {
        on_tick: contains_callable(class_table, "on_tick")?,
        on_signal: contains_callable(class_table, "on_signal")?,
        on_spawn: contains_callable(class_table, "on_spawn")?,
    })
}

fn contains_callable(table: &LuaTable, key: &str) -> LuaResult<bool> {
    match table.get::<LuaValue>(key) {
        Ok(LuaValue::Function(_)) => Ok(true),
        Ok(_) => Ok(false),
        Err(_) => Ok(false),
    }
}
