use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
use mlua::{Lua, LuaOptions};
use serde_json::Value;

use craft_kernel::{ComponentValue, Engine, SignalBus};

use crate::bindings::{NodeRef, build_node};
use crate::query::QueryRegistry;

pub type ScriptError = LuaError;

const RNG_MUL: u64 = 6364136223846793005;
const RNG_INC: u64 = 1442695040888963407;

pub struct LuaRuntime {
    vm: Lua,
    queries: Rc<RefCell<QueryRegistry>>,
    rng_state: Rc<RefCell<u64>>,
}

impl LuaRuntime {
    pub fn new(seed: u64) -> LuaResult<Self> {
        let vm = Lua::new_with(
            LuaStdLib::COROUTINE
                | LuaStdLib::TABLE
                | LuaStdLib::STRING
                | LuaStdLib::UTF8
                | LuaStdLib::MATH
                | LuaStdLib::PACKAGE,
            LuaOptions::new(),
        )?;

        sandbox_vm(&vm)?;

        Ok(Self {
            vm,
            queries: Rc::new(RefCell::new(QueryRegistry::new())),
            rng_state: Rc::new(RefCell::new(seed.wrapping_add(1))),
        })
    }

    pub fn register_query<F>(&mut self, name: &str, handler: F) -> &mut Self
    where
        F: Fn(Value) -> Result<Value, String> + 'static,
    {
        self.queries.borrow_mut().register(name, handler);
        self
    }

    pub fn query_registry(&self) -> std::cell::Ref<'_, QueryRegistry> {
        self.queries.borrow()
    }

    pub fn run(&mut self, engine: &mut Engine, script: &str) -> Result<(), ScriptError> {
        let scene = engine
            .scene_mut()
            .ok_or_else(|| LuaError::external("engine has no scene loaded"))?;

        let scene_handle = SceneHandle::wrap(scene);
        let bus_handle = BusHandle::wrap(&mut engine.bus);
        let queries = Rc::clone(&self.queries);
        let rng_state = Rc::clone(&self.rng_state);

        self.install_engine_api(scene_handle.clone(), bus_handle.clone(), queries, rng_state)?;
        let result = self.vm.load(script).exec();
        let _ = self.uninstall_engine_api();
        result
    }

    fn install_engine_api(
        &self,
        scene: SceneHandle,
        bus: BusHandle,
        queries: Rc<RefCell<QueryRegistry>>,
        rng_state: Rc<RefCell<u64>>,
    ) -> LuaResult<()> {
        let engine_table = self.vm.create_table()?;

        engine_table.set("emit", make_emit(&self.vm, bus.clone())?)?;
        engine_table.set("spawn", make_spawn(&self.vm, scene.clone())?)?;
        engine_table.set("call_system", make_call_system(&self.vm, queries)?)?;
        engine_table.set("rng", make_rng(&self.vm, rng_state)?)?;
        engine_table.set("get_node", make_get_node(&self.vm, scene)?)?;
        engine_table.set("log", make_log(&self.vm)?)?;

        self.vm.globals().set("engine", engine_table)?;
        Ok(())
    }

    fn uninstall_engine_api(&self) -> LuaResult<()> {
        self.vm.globals().set("engine", LuaValue::Nil)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SceneHandle {
    ptr: *mut craft_kernel::Scene,
}

unsafe impl Send for SceneHandle {}
unsafe impl Sync for SceneHandle {}

impl SceneHandle {
    fn wrap(scene: &mut craft_kernel::Scene) -> Self {
        Self {
            ptr: scene as *mut craft_kernel::Scene,
        }
    }

    pub fn with_ref<R>(&self, f: impl FnOnce(&craft_kernel::Scene) -> R) -> R {
        unsafe { f(&*self.ptr) }
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut craft_kernel::Scene) -> R) -> R {
        unsafe { f(&mut *self.ptr) }
    }
}

#[derive(Clone)]
pub struct BusHandle {
    ptr: *mut SignalBus,
}

unsafe impl Send for BusHandle {}
unsafe impl Sync for BusHandle {}

impl BusHandle {
    fn wrap(bus: &mut SignalBus) -> Self {
        Self {
            ptr: bus as *mut SignalBus,
        }
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut SignalBus) -> R) -> R {
        unsafe { f(&mut *self.ptr) }
    }
}

// SAFETY: Both SceneHandle and BusHandle store raw pointers that are guaranteed
// to remain valid for the duration of a single LuaRuntime::run call, because
// the caller (run) creates the handles from a `&mut Engine` and drops them
// before returning. The raw-pointer indirection lets the RefCell reborrow
// without imposing an invariant `'static` bound on the inner `&mut T`.

fn sandbox_vm(vm: &Lua) -> LuaResult<()> {
    let globals = vm.globals();

    globals.set("io", LuaValue::Nil)?;
    globals.set("os", LuaValue::Nil)?;
    globals.set("debug", LuaValue::Nil)?;
    globals.set("dofile", LuaValue::Nil)?;
    globals.set("loadfile", LuaValue::Nil)?;
    globals.set("load", LuaValue::Nil)?;
    globals.set("loadstring", LuaValue::Nil)?;
    globals.set("require", LuaValue::Nil)?;

    let package: LuaTable = globals.get("package")?;
    package.set("loadlib", LuaValue::Nil)?;
    package.set("cpath", LuaValue::Nil)?;
    package.set("searchers", LuaValue::Nil)?;
    package.set("loaders", LuaValue::Nil)?;
    package.set("preload", vm.create_table()?)?;

    let math: LuaTable = globals.get("math")?;
    math.set("random", LuaValue::Nil)?;
    math.set("randomseed", LuaValue::Nil)?;
    Ok(())
}

fn make_emit(vm: &Lua, bus: BusHandle) -> LuaResult<LuaFunction> {
    vm.create_function(move |_, (name, payload): (String, LuaValue)| {
        let payload_json = lua_value_to_json(payload);
        bus.with_mut(|bus| {
            let id = bus.declare(&name);
            bus.emit(id, payload_json);
        });
        Ok(())
    })
}

fn make_spawn(vm: &Lua, scene: SceneHandle) -> LuaResult<LuaFunction> {
    vm.create_function(move |lua, (type_name, components): (String, LuaTable)| {
        let comp_pairs = read_components(&components)?;
        let id = scene.with_mut(|s| {
            let id = s.next_spawn_id(&type_name);
            let node = build_node(&type_name, comp_pairs, id.clone());
            s.add_node(node);
            id
        });
        let ud = lua.create_userdata(NodeRef::new(id, scene.clone()))?;
        Ok(LuaValue::UserData(ud))
    })
}

fn make_call_system(vm: &Lua, queries: Rc<RefCell<QueryRegistry>>) -> LuaResult<LuaFunction> {
    vm.create_function(move |lua, (name, args): (String, Option<LuaTable>)| {
        let args_json = match args {
            Some(t) => lua_table_to_json(t, 0)?,
            None => Value::Null,
        };
        let registry = queries.borrow();
        match registry.call(&name, args_json) {
            Ok(result) => json_to_lua_value(lua, &result),
            Err(e) => Err(LuaError::external(format!("query \"{name}\" failed: {e}"))),
        }
    })
}

fn make_rng(vm: &Lua, state: Rc<RefCell<u64>>) -> LuaResult<LuaFunction> {
    vm.create_function(move |_, (lo, hi): (i64, i64)| {
        let mut state_ref = state.borrow_mut();
        *state_ref = state_ref.wrapping_mul(RNG_MUL).wrapping_add(RNG_INC);
        let range = (hi - lo + 1) as u64;
        let offset = if range == 0 { 0 } else { *state_ref % range };
        Ok(lo + offset as i64)
    })
}

fn make_get_node(vm: &Lua, scene: SceneHandle) -> LuaResult<LuaFunction> {
    vm.create_function(move |lua, id: String| {
        let exists = scene.with_ref(|s| s.find_node(&id).is_some());
        if exists {
            let ud = lua.create_userdata(NodeRef::new(id, scene.clone()))?;
            Ok(LuaValue::UserData(ud))
        } else {
            Ok(LuaValue::Nil)
        }
    })
}

fn make_log(vm: &Lua) -> LuaResult<LuaFunction> {
    vm.create_function(move |_, (level, msg): (String, String)| {
        eprintln!("[lua:{level}] {msg}");
        Ok(())
    })
}

fn read_components(table: &LuaTable) -> LuaResult<Vec<(String, ComponentValue)>> {
    let mut out = Vec::new();
    for pair in table.pairs::<String, LuaValue>() {
        let (k, v) = pair?;
        let cv = lua_to_component_value(v)?;
        out.push((k, cv));
    }
    Ok(out)
}

pub fn lua_to_component_value(value: LuaValue) -> LuaResult<ComponentValue> {
    match value {
        LuaValue::Nil => Ok(ComponentValue::Nil),
        LuaValue::Boolean(b) => Ok(ComponentValue::Bool(b)),
        LuaValue::Integer(i) => Ok(ComponentValue::Int(i)),
        LuaValue::Number(n) => Ok(ComponentValue::Float(n)),
        LuaValue::String(s) => Ok(ComponentValue::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => {
            let len = t.len()?;
            if len == 2 {
                let x: f64 = t.get(1)?;
                let y: f64 = t.get(2)?;
                Ok(ComponentValue::Vec2([x, y]))
            } else {
                Err(LuaError::external(format!(
                    "unsupported table of length {len}; only [x, y] vec2 tables can become component values"
                )))
            }
        }
        other => Err(LuaError::external(format!(
            "cannot convert lua value of type {} to component value",
            value_type_name(&other)
        ))),
    }
}

pub fn component_value_to_lua(lua: &Lua, value: &ComponentValue) -> LuaResult<LuaValue> {
    match value {
        ComponentValue::Nil => Ok(LuaValue::Nil),
        ComponentValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        ComponentValue::Int(i) => Ok(LuaValue::Integer(*i)),
        ComponentValue::Float(f) => Ok(LuaValue::Number(*f)),
        ComponentValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        ComponentValue::Vec2([x, y]) => {
            let t = lua.create_table()?;
            t.set(1, *x)?;
            t.set(2, *y)?;
            Ok(LuaValue::Table(t))
        }
    }
}

fn lua_value_to_json(value: LuaValue) -> Value {
    match value {
        LuaValue::Nil => Value::Null,
        LuaValue::Boolean(b) => Value::Bool(b),
        LuaValue::Integer(i) => Value::Number(i.into()),
        LuaValue::Number(n) => serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        LuaValue::String(s) => match s.to_str() {
            Ok(s) => Value::String(s.to_string()),
            Err(_) => Value::Null,
        },
        LuaValue::Table(t) => match lua_table_to_json(t, 0) {
            Ok(v) => v,
            Err(_) => Value::Null,
        },
        _ => Value::Null,
    }
}

fn lua_table_to_json(t: LuaTable, depth: u32) -> LuaResult<Value> {
    if depth > 8 {
        return Err(LuaError::external("table nesting exceeds 8"));
    }
    let len = t.len()?;
    if len > 0 {
        let mut arr = Vec::with_capacity(len as usize);
        for i in 1..=len {
            let v: LuaValue = t.get(i)?;
            arr.push(lua_value_to_json_safe(v, depth + 1)?);
        }
        return Ok(Value::Array(arr));
    }
    let mut obj = serde_json::Map::new();
    for pair in t.pairs::<LuaValue, LuaValue>() {
        let (k, v) = pair?;
        let key = match k {
            LuaValue::String(s) => s.to_str()?.to_string(),
            LuaValue::Integer(i) => i.to_string(),
            other => {
                return Err(LuaError::external(format!(
                    "object key must be string or integer, got {}",
                    value_type_name(&other)
                )));
            }
        };
        obj.insert(key, lua_value_to_json_safe(v, depth + 1)?);
    }
    Ok(Value::Object(obj))
}

fn lua_value_to_json_safe(value: LuaValue, depth: u32) -> LuaResult<Value> {
    match value {
        LuaValue::Nil => Ok(Value::Null),
        LuaValue::Boolean(b) => Ok(Value::Bool(b)),
        LuaValue::Integer(i) => Ok(Value::Number(i.into())),
        LuaValue::Number(n) => serde_json::Number::from_f64(n)
            .map(Value::Number)
            .ok_or_else(|| LuaError::external(format!("non-finite float: {n}"))),
        LuaValue::String(s) => Ok(Value::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => lua_table_to_json(t, depth),
        other => Err(LuaError::external(format!(
            "cannot marshal lua value of type {} to JSON",
            value_type_name(&other)
        ))),
    }
}

fn json_to_lua_value(lua: &Lua, value: &Value) -> LuaResult<LuaValue> {
    match value {
        Value::Null => Ok(LuaValue::Nil),
        Value::Bool(b) => Ok(LuaValue::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Err(LuaError::external(format!("unsupported number: {n}")))
            }
        }
        Value::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        Value::Array(arr) => {
            let t = lua.create_table_with_capacity(arr.len(), 0)?;
            for (i, item) in arr.iter().enumerate() {
                t.set(i + 1, json_to_lua_value(lua, item)?)?;
            }
            Ok(LuaValue::Table(t))
        }
        Value::Object(obj) => {
            let t = lua.create_table_with_capacity(0, obj.len())?;
            for (k, v) in obj {
                t.set(k.as_str(), json_to_lua_value(lua, v)?)?;
            }
            Ok(LuaValue::Table(t))
        }
    }
}

fn value_type_name(v: &LuaValue) -> &'static str {
    match v {
        LuaValue::Nil => "nil",
        LuaValue::Boolean(_) => "boolean",
        LuaValue::LightUserData(_) => "lightuserdata",
        LuaValue::Integer(_) => "integer",
        LuaValue::Number(_) => "number",
        LuaValue::String(_) => "string",
        LuaValue::Table(_) => "table",
        LuaValue::Function(_) => "function",
        LuaValue::Thread(_) => "thread",
        LuaValue::UserData(_) => "userdata",
        LuaValue::Error(_) => "error",
        _ => "other",
    }
}
