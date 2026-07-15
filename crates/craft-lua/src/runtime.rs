use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use mlua::prelude::*;
use mlua::{Lua, LuaOptions};
use serde_json::Value;

use craft_kernel::{ComponentValue, Engine, Scene, SignalBus};

use crate::bindings::{NodeRef, build_node};
use crate::class::{ClassDef, probe_hooks};
use crate::determinism::{DeterminismMode, DeterminismState, DeterminismSwitches, RecordingLog};
use crate::modules::{LockEntry, Lockfile, ModuleLoader};
use crate::query::QueryRegistry;

pub type ScriptError = LuaError;

const RNG_MUL: u64 = 6364136223846793005;
const RNG_INC: u64 = 1442695040888963407;

#[derive(Clone)]
pub(crate) struct NodeBinding {
    pub class_name: String,
    pub self_table: LuaTable,
}

pub struct LuaRuntime {
    vm: Lua,
    queries: Rc<RefCell<QueryRegistry>>,
    rng_state: Rc<RefCell<u64>>,
    run_generation: Rc<RefCell<u64>>,
    classes: HashMap<String, ClassDef>,
    bindings: HashMap<String, NodeBinding>,
    module_loader: Option<ModuleLoader>,
    loaded_modules: HashMap<String, (String, String)>,
    determinism: Rc<RefCell<DeterminismState>>,
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
            run_generation: Rc::new(RefCell::new(0)),
            classes: HashMap::new(),
            bindings: HashMap::new(),
            module_loader: None,
            loaded_modules: HashMap::new(),
            determinism: Rc::new(RefCell::new(DeterminismState::default())),
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

        let generation = {
            let mut g = self.run_generation.borrow_mut();
            *g = g.wrapping_add(1);
            *g
        };

        let scene_handle = SceneHandle::wrap(scene, generation);
        let bus_handle = BusHandle::wrap(&mut engine.bus, generation);
        let queries = Rc::clone(&self.queries);
        let rng_state = Rc::clone(&self.rng_state);

        self.install_engine_api(
            scene_handle,
            bus_handle,
            queries,
            rng_state,
            generation,
            Rc::clone(&self.run_generation),
            Rc::clone(&self.determinism),
        )?;
        let result = self.vm.load(script).exec();
        let _ = self.uninstall_engine_api();
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn install_engine_api(
        &self,
        scene: SceneHandle,
        bus: BusHandle,
        queries: Rc<RefCell<QueryRegistry>>,
        rng_state: Rc<RefCell<u64>>,
        generation: u64,
        current_generation: Rc<RefCell<u64>>,
        determinism: Rc<RefCell<DeterminismState>>,
    ) -> LuaResult<()> {
        // Wipe any leftover engine globals (and NodeRefs bound to a prior run's
        // generation) before installing fresh ones, so a partial failure
        // here cannot leave userdata pointing at a stale `&mut Engine` borrow.
        self.uninstall_engine_api()?;

        let engine_table = self.vm.create_table()?;

        engine_table.set("emit", make_emit(&self.vm, bus.clone(), generation)?)?;
        engine_table.set(
            "spawn",
            make_spawn(
                &self.vm,
                scene.clone(),
                generation,
                Rc::clone(&current_generation),
                Rc::clone(&determinism),
            )?,
        )?;
        engine_table.set("call_system", make_call_system(&self.vm, queries)?)?;
        engine_table.set("rng", make_rng(&self.vm, rng_state)?)?;
        engine_table.set(
            "get_node",
            make_get_node(
                &self.vm,
                scene,
                generation,
                Rc::clone(&current_generation),
                Rc::clone(&determinism),
            )?,
        )?;
        engine_table.set("log", make_log(&self.vm)?)?;

        self.vm.globals().set("engine", engine_table)?;
        Ok(())
    }

    fn uninstall_engine_api(&self) -> LuaResult<()> {
        self.vm.globals().set("engine", LuaValue::Nil)?;
        Ok(())
    }

    pub fn load_class(&mut self, name: &str, source: &str) -> Result<ClassDef, ScriptError> {
        self.vm.load(source).exec()?;
        let class_table: LuaTable = self.vm.globals().get(name).map_err(|e| {
            LuaError::external(format!(
                "load_class({name:?}): script did not define a global named {name:?}: {e}"
            ))
        })?;
        let new_fn: LuaFunction = class_table.get("new").map_err(|e| {
            LuaError::external(format!(
                "load_class({name:?}): class table missing a `new` method: {e}"
            ))
        })?;
        let _ = new_fn;
        let hooks = probe_hooks(&class_table)?;
        let def = ClassDef {
            name: name.to_string(),
            source: source.to_string(),
            hooks,
        };
        self.classes.insert(name.to_string(), def.clone());
        Ok(def)
    }

    pub fn reload_class(&mut self, name: &str, source: &str) -> Result<(), ScriptError> {
        if !self.classes.contains_key(name) {
            return Err(LuaError::external(format!(
                "reload_class({name:?}): class not loaded; call load_class first"
            )));
        }
        self.vm.load(source).exec()?;
        let class_table: LuaTable = self.vm.globals().get(name)?;
        let hooks = probe_hooks(&class_table)?;
        self.classes.insert(
            name.to_string(),
            ClassDef {
                name: name.to_string(),
                source: source.to_string(),
                hooks,
            },
        );
        Ok(())
    }

    pub fn class_count(&self) -> usize {
        self.classes.len()
    }

    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Bind a scene node to a previously loaded class. Calls `class.new(node)`
    /// to produce the per-instance `self` table and stores it. If the class
    /// declares `on_spawn`, that hook is invoked immediately so the instance
    /// can initialize its Lua-side state in the same call.
    pub fn bind_node(
        &mut self,
        scene: &Scene,
        node_id: &str,
        class_name: &str,
    ) -> Result<(), ScriptError> {
        if scene.find_node(node_id).is_none() {
            return Err(LuaError::external(format!(
                "bind_node: node {node_id:?} not found in scene"
            )));
        }
        let class = self.classes.get(class_name).ok_or_else(|| {
            LuaError::external(format!(
                "bind_node({node_id:?}, {class_name:?}): class not loaded"
            ))
        })?;
        if !class.hooks.on_tick && !class.hooks.on_signal && !class.hooks.on_spawn {
            return Err(LuaError::external(format!(
                "bind_node({node_id:?}, {class_name:?}): class has no lifecycle hooks"
            )));
        }

        let generation = *self.run_generation.borrow();
        let scene_handle = SceneHandle::wrap_scene(scene, generation);
        let node_ref = NodeRef::new(
            node_id.to_string(),
            scene_handle,
            Rc::clone(&self.run_generation),
            Rc::clone(&self.determinism),
        );
        let node_ud = self.vm.create_userdata(node_ref)?;

        let class_table: LuaTable = self.vm.globals().get(class_name)?;
        let new_fn: LuaFunction = class_table.get("new")?;
        let self_table: LuaTable = new_fn.call(LuaValue::UserData(node_ud))?;

        let should_call_spawn = class.hooks.on_spawn;
        self.bindings.insert(
            node_id.to_string(),
            NodeBinding {
                class_name: class_name.to_string(),
                self_table: self_table.clone(),
            },
        );

        if should_call_spawn {
            let binding = self.bindings.get(node_id).expect("binding just inserted");
            self.call_hook(binding, "on_spawn", ())?;
        }
        Ok(())
    }

    pub fn unbind_node(&mut self, node_id: &str) -> bool {
        self.bindings.remove(node_id).is_some()
    }

    /// Fire `on_tick` for every bound node. Each hook receives a freshly
    /// written `self.node` so reads/writes against components see current
    /// scene state. The previous `self.node` UserData is overwritten and
    /// will be garbage-collected by Lua.
    pub fn tick_pre_pass(&mut self, engine: &mut Engine) -> Result<(), ScriptError> {
        let scene = match engine.scene_mut() {
            Some(s) => s,
            None => return Ok(()),
        };
        let generation = *self.run_generation.borrow();
        let scene_handle = SceneHandle::wrap_scene(scene, generation);

        let node_ids: Vec<String> = self.bindings.keys().cloned().collect();
        for node_id in node_ids {
            let Some(node) = scene.find_node(&node_id) else {
                continue;
            };
            let _ = node;
            let Some(class) = self.classes.get(
                self.bindings
                    .get(&node_id)
                    .map(|b| b.class_name.as_str())
                    .unwrap_or(""),
            ) else {
                continue;
            };
            if !class.hooks.on_tick {
                continue;
            }
            let node_ref = NodeRef::new(
                node_id.clone(),
                scene_handle.clone(),
                Rc::clone(&self.run_generation),
                Rc::clone(&self.determinism),
            );
            let ud = self.vm.create_userdata(node_ref)?;
            let binding = self.bindings.get(&node_id).expect("checked above");
            binding.self_table.set("node", LuaValue::UserData(ud))?;
            self.call_hook(binding, "on_tick", ())?;
        }
        Ok(())
    }

    /// Fire `on_signal(name, args)` for every bound node whose class declares
    /// the hook.
    pub fn dispatch_signal(
        &mut self,
        engine: &mut Engine,
        signal_name: &str,
        args: &Value,
    ) -> Result<(), ScriptError> {
        let scene = match engine.scene_mut() {
            Some(s) => s,
            None => return Ok(()),
        };
        let generation = *self.run_generation.borrow();
        let scene_handle = SceneHandle::wrap_scene(scene, generation);
        let args_lua = json_to_lua_value(&self.vm, args)?;

        let node_ids: Vec<String> = self.bindings.keys().cloned().collect();
        for node_id in node_ids {
            let class_name = match self.bindings.get(&node_id) {
                Some(b) => b.class_name.clone(),
                None => continue,
            };
            let class = match self.classes.get(&class_name) {
                Some(c) => c,
                None => continue,
            };
            if !class.hooks.on_signal {
                continue;
            }
            let node_ref = NodeRef::new(
                node_id.clone(),
                scene_handle.clone(),
                Rc::clone(&self.run_generation),
                Rc::clone(&self.determinism),
            );
            let ud = self.vm.create_userdata(node_ref)?;
            let binding = self.bindings.get(&node_id).expect("checked above");
            binding.self_table.set("node", LuaValue::UserData(ud))?;
            self.call_hook(binding, "on_signal", (signal_name, args_lua.clone()))?;
        }
        Ok(())
    }

    /// Fire `on_spawn` for a single bound node. Called by the engine when
    /// a new node enters the scene (e.g. from `engine.spawn`).
    pub fn dispatch_spawn(
        &mut self,
        engine: &mut Engine,
        node_id: &str,
    ) -> Result<(), ScriptError> {
        let scene = match engine.scene_mut() {
            Some(s) => s,
            None => return Ok(()),
        };
        let generation = *self.run_generation.borrow();
        let scene_handle = SceneHandle::wrap_scene(scene, generation);

        let binding = match self.bindings.get(node_id) {
            Some(b) => b,
            None => return Ok(()),
        };
        let class = match self.classes.get(&binding.class_name) {
            Some(c) => c,
            None => return Ok(()),
        };
        if !class.hooks.on_spawn {
            return Ok(());
        }
        let node_ref = NodeRef::new(
            node_id.to_string(),
            scene_handle,
            Rc::clone(&self.run_generation),
            Rc::clone(&self.determinism),
        );
        let ud = self.vm.create_userdata(node_ref)?;
        binding.self_table.set("node", LuaValue::UserData(ud))?;
        self.call_hook(binding, "on_spawn", ())
    }

    fn call_hook<A>(&self, binding: &NodeBinding, hook: &str, args: A) -> LuaResult<()>
    where
        A: IntoLuaMulti,
    {
        let func: Option<LuaFunction> = binding.self_table.get(hook).ok();
        if let Some(f) = func {
            let _: () = f.call((binding.self_table.clone(), args))?;
        }
        Ok(())
    }

    pub fn set_modules_dir(&mut self, dir: PathBuf) {
        let lockfile = self
            .module_loader
            .as_ref()
            .map(|l| l.lockfile.clone())
            .unwrap_or_default();
        self.module_loader = Some(ModuleLoader {
            modules_dir: dir,
            lockfile,
        });
        self.install_require_searcher();
    }

    pub fn module_loader(&self) -> Option<&ModuleLoader> {
        self.module_loader.as_ref()
    }

    pub fn load_lockfile_from_path(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        let loader = self.module_loader.as_mut().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "set_modules_dir() must be called before load_lockfile",
            )
        })?;
        loader.load_lockfile_from_path(path)?;
        self.install_require_searcher();
        Ok(())
    }

    pub fn write_lockfile_to_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        let loader = self.module_loader.as_ref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "set_modules_dir() must be called before write_lockfile",
            )
        })?;
        loader.write_lockfile_to_path(path)
    }

    pub fn validate_lockfile(&self) -> Result<(), String> {
        match self.module_loader.as_ref() {
            Some(l) => l.validate_lockfile(),
            None => Err("no module loader configured; call set_modules_dir first".to_string()),
        }
    }

    pub fn lock_dependencies(&mut self) -> Lockfile {
        let lockfile = match self.module_loader.as_ref() {
            Some(loader) => loader.build_lockfile(&self.loaded_modules),
            None => Lockfile::empty(),
        };
        if let Some(loader) = self.module_loader.as_mut() {
            loader.lockfile = lockfile.clone();
        }
        lockfile
    }

    pub fn lockfile(&self) -> Option<Lockfile> {
        self.module_loader.as_ref().map(|l| l.lockfile.clone())
    }

    pub fn loaded_module_count(&self) -> usize {
        self.loaded_modules.len()
    }

    pub fn record_module(
        &mut self,
        name: &str,
        version: &str,
        source: &str,
    ) -> Result<(), ScriptError> {
        let module_name = name.to_string();
        let source_str = source.to_string();
        self.loaded_modules.insert(
            module_name.clone(),
            (version.to_string(), source_str.clone()),
        );
        let package: LuaTable = self.vm.globals().get("package")?;
        let loaded: LuaTable = package.get("loaded")?;
        loaded.set(module_name.as_str(), source_str)?;
        let hash = crate::modules::sha256_of_str(source);
        if let Some(loader) = self.module_loader.as_mut() {
            let rel = module_name
                .split('.')
                .collect::<PathBuf>()
                .with_extension("lua");
            let path = loader.modules_dir.join(&rel);
            let entry = LockEntry {
                name: module_name.clone(),
                version: version.to_string(),
                path,
                sha256: hash,
            };
            if !loader.lockfile.entries.iter().any(|e| e.name == entry.name) {
                loader.lockfile.entries.push(entry);
            }
        }
        Ok(())
    }

    fn install_require_searcher(&self) {
        let Some(loader) = self.module_loader.as_ref().cloned() else {
            return;
        };
        let loader_rc = Rc::new(loader);

        let loaded_rc = Rc::new(RefCell::new(HashMap::<String, LuaValue>::new()));
        let loaded_for_searcher = Rc::clone(&loaded_rc);
        let result: LuaResult<LuaFunction> =
            self.vm.create_function(move |lua_ctx, name: String| {
                let loader = loader_rc.as_ref();
                let path = match loader.resolve_path(&name) {
                    Some(p) => p,
                    None => {
                        return Err(LuaError::external(format!(
                            "module {name:?} not found in modules dir"
                        )));
                    }
                };
                if let Err(e) = loader.validate_lockfile_for(&name, &path) {
                    return Err(LuaError::external(e));
                }
                if let Some(cached) = loaded_for_searcher.borrow().get(&name).cloned() {
                    return Ok(cached);
                }
                let contents = std::fs::read_to_string(&path).map_err(|e| {
                    LuaError::external(format!(
                        "require({name}): cannot read {}: {e}",
                        path.display()
                    ))
                })?;
                let chunk = lua_ctx.load(&contents).set_name(&name).into_function()?;
                let loaded: LuaValue = chunk.call(())?;
                loaded_for_searcher
                    .borrow_mut()
                    .insert(name, loaded.clone());
                Ok(loaded)
            });
        let Ok(searcher) = result else {
            return;
        };
        let _ = self.vm.globals().set("require", searcher);
    }

    pub fn set_determinism(&mut self, mode: DeterminismMode) -> Result<(), ScriptError> {
        let switches = mode.switches();
        self.determinism.borrow_mut().switches = switches;
        self.determinism.borrow_mut().mode = Some(mode);

        let math_table: LuaTable = self.vm.globals().get("math")?;
        if switches.rng {
            let state = Rc::clone(&self.rng_state);
            let det = Rc::clone(&self.determinism);
            let rng_fn = self.vm.create_function(move |_, (lo, hi): (i64, i64)| {
                if lo > hi {
                    return Err(LuaError::external(format!(
                        "engine.rng(lo, hi) requires lo <= hi, got lo={lo} hi={hi}"
                    )));
                }
                let span = (hi as i128) - (lo as i128) + 1;
                if span > (u64::MAX as i128) {
                    return Err(LuaError::external(format!(
                        "engine.rng range [{lo}, {hi}] exceeds u64 span"
                    )));
                }
                let mut st = state.borrow_mut();
                *st = st.wrapping_mul(RNG_MUL).wrapping_add(RNG_INC);
                let range = span as u64;
                let offset = *st % range;
                {
                    let mut det_b = det.borrow_mut();
                    det_b
                        .recording
                        .record_call("engine.rng", format!("{lo},{hi}"));
                }
                Ok(lo + offset as i64)
            })?;
            math_table.set("random", rng_fn.clone())?;
            math_table.set("randomseed", rng_fn)?;
        } else {
            math_table.set("random", LuaValue::Nil)?;
            math_table.set("randomseed", LuaValue::Nil)?;
        }

        if switches.float {
            let det = Rc::clone(&self.determinism);
            let craft_table = self.vm.create_table()?;
            let det_for_is_finite = Rc::clone(&det);
            craft_table.set(
                "is_finite",
                self.vm.create_function(move |_, x: f64| {
                    Ok(det_for_is_finite.borrow().switches.float && x.is_finite())
                })?,
            )?;
            let det_for_sanitize = Rc::clone(&det);
            craft_table.set(
                "sanitize",
                self.vm
                    .create_function(move |_, (x, default): (f64, Option<f64>)| {
                        let det_b = det_for_sanitize.borrow();
                        if det_b.switches.float && !x.is_finite() {
                            Ok(default.unwrap_or(0.0))
                        } else {
                            Ok(x)
                        }
                    })?,
            )?;
            let det_for_check = Rc::clone(&det);
            craft_table.set(
                "require_finite",
                self.vm.create_function(move |_, x: f64| {
                    let det_b = det_for_check.borrow();
                    if det_b.switches.float && !x.is_finite() {
                        Err(LuaError::external(format!(
                            "float lock is on: value {x} is non-finite"
                        )))
                    } else {
                        Ok(x)
                    }
                })?,
            )?;
            let det_for_log = Rc::clone(&det);
            craft_table.set(
                "log_non_finite",
                self.vm.create_function(move |_, (op, x): (String, f64)| {
                    if !x.is_finite() {
                        det_for_log
                            .borrow_mut()
                            .recording
                            .record_call("craft.float.reject", format!("{op}: {x}"));
                    }
                    Ok(())
                })?,
            )?;
            self.vm.globals().set("craft", craft_table)?;
        } else {
            self.vm.globals().set("craft", LuaValue::Nil)?;
        }

        if switches.order {
            self.install_sorted_pairs()?;
        }
        Ok(())
    }

    /// Install sorted-key `pairs()` and `ipairs()` over global tables
    /// so that `for k, v in pairs(t)` iterates keys in a stable order
    /// across platforms and runs. Implementation: wrap the global `pairs`
    /// in pure Lua so it sorts keys before yielding them. Mixed-key tables
    /// fall back to type-tag ordering to keep the comparison total.
    fn install_sorted_pairs(&self) -> LuaResult<()> {
        let source = r#"
            do
                if _G.pairs ~= nil and _G.craft_original_pairs == nil then
                    _G.craft_original_pairs = _G.pairs
                end
                _G.pairs = function(t, k)
                    if type(t) ~= "table" then
                        return _G.craft_original_pairs(t, k)
                    end
                    local keys = {}
                    for key, _ in _G.craft_original_pairs(t) do
                        keys[#keys + 1] = key
                    end
                    table.sort(keys, function(a, b)
                        local ta, tb = type(a), type(b)
                        if ta == tb then
                            if ta == "number" or ta == "string" then
                                return a < b
                            end
                            return tostring(a) < tostring(b)
                        end
                        return ta < tb
                    end)
                    local i = 0
                    return function()
                        i = i + 1
                        local key = keys[i]
                        if key == nil then return nil end
                        return key, t[key]
                    end
                end
            end
        "#;
        self.vm.load(source).exec()
    }

    pub fn switches(&self) -> DeterminismSwitches {
        self.determinism.borrow().switches
    }

    pub fn determinism_mode(&self) -> Option<DeterminismMode> {
        self.determinism.borrow().mode
    }

    pub fn recording_log(&self) -> RecordingLog {
        self.determinism.borrow().recording.clone()
    }

    pub fn take_recording_log(&self) -> RecordingLog {
        let mut d = self.determinism.borrow_mut();
        std::mem::take(&mut d.recording)
    }

    pub fn clear_recording_log(&self) {
        self.determinism.borrow_mut().recording.clear();
    }
}

#[derive(Clone)]
pub struct SceneHandle {
    ptr: *mut craft_kernel::Scene,
    generation: u64,
}

// The raw pointer is only dereferenced through `with_ref`/`with_mut`, and only
// while a `LuaRuntime::run` call holds the original `&mut Scene` borrow. The
// `generation` is bumped on every `wrap` so userdata created during a prior
// `run` (whose `&mut Scene` borrow has since ended) can detect that its
// pointer is stale and refuse to dereference it. `NodeRef` and the engine
// globals store this generation alongside the pointer.
impl SceneHandle {
    fn wrap(scene: &mut craft_kernel::Scene, generation: u64) -> Self {
        Self {
            ptr: scene as *mut craft_kernel::Scene,
            generation,
        }
    }

    /// Wrap a shared `&Scene` reference. The raw pointer remains valid only
    /// while the original borrow lives; the caller is responsible for
    /// ensuring the borrow outlives every dereference of the returned handle
    /// (the typical pattern is "create one handle, use it within one
    /// function body that already holds the borrow").
    fn wrap_scene(scene: &craft_kernel::Scene, generation: u64) -> Self {
        Self {
            ptr: scene as *const craft_kernel::Scene as *mut craft_kernel::Scene,
            generation,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn with_ref<R>(
        &self,
        expected_generation: u64,
        f: impl FnOnce(&craft_kernel::Scene) -> R,
    ) -> Result<R, String> {
        if expected_generation != self.generation {
            return Err(format!(
                "scene handle is from a prior run() call (expected generation {expected_generation}, got {})",
                self.generation
            ));
        }
        Ok(unsafe { f(&*self.ptr) })
    }

    pub fn with_mut<R>(
        &self,
        expected_generation: u64,
        f: impl FnOnce(&mut craft_kernel::Scene) -> R,
    ) -> Result<R, String> {
        if expected_generation != self.generation {
            return Err(format!(
                "scene handle is from a prior run() call (expected generation {expected_generation}, got {})",
                self.generation
            ));
        }
        Ok(unsafe { f(&mut *self.ptr) })
    }
}

#[derive(Clone)]
pub struct BusHandle {
    ptr: *mut SignalBus,
    generation: u64,
}

impl BusHandle {
    fn wrap(bus: &mut SignalBus, generation: u64) -> Self {
        Self {
            ptr: bus as *mut SignalBus,
            generation,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn with_mut<R>(
        &self,
        expected_generation: u64,
        f: impl FnOnce(&mut SignalBus) -> R,
    ) -> Result<R, String> {
        if expected_generation != self.generation {
            return Err(format!(
                "bus handle is from a prior run() call (expected generation {expected_generation}, got {})",
                self.generation
            ));
        }
        Ok(unsafe { f(&mut *self.ptr) })
    }
}

// SAFETY: Both SceneHandle and BusHandle store raw pointers to engine
// internals that are only valid while the originating `LuaRuntime::run` call
// holds the `&mut Engine` borrow. The `generation` field makes that
// liveness checkable: each `with_ref`/`with_mut` call validates that the
// caller's expected generation matches the handle's generation, which is
// bumped on every `run`. Userdata created during a prior run (whose
// `&mut Engine` borrow has since ended) carry a stale generation and are
// refused with a LuaError before any dereference.

fn sandbox_vm(vm: &Lua) -> LuaResult<()> {
    let globals = vm.globals();

    globals.set("io", LuaValue::Nil)?;
    globals.set("os", LuaValue::Nil)?;
    globals.set("debug", LuaValue::Nil)?;
    globals.set("dofile", LuaValue::Nil)?;
    globals.set("loadfile", LuaValue::Nil)?;
    globals.set("load", LuaValue::Nil)?;
    globals.set("loadstring", LuaValue::Nil)?;

    let package: LuaTable = globals.get("package")?;
    package.set("loadlib", LuaValue::Nil)?;
    package.set("cpath", LuaValue::Nil)?;
    package.set("cpathcpath", LuaValue::Nil)?;
    package.set("preload", vm.create_table()?)?;

    let math: LuaTable = globals.get("math")?;
    math.set("random", LuaValue::Nil)?;
    math.set("randomseed", LuaValue::Nil)?;
    Ok(())
}

fn make_emit(vm: &Lua, bus: BusHandle, generation: u64) -> LuaResult<LuaFunction> {
    vm.create_function(move |_, (name, payload): (String, LuaValue)| {
        let payload_json = lua_value_to_json(payload)?;
        bus.with_mut(generation, |bus| {
            let id = bus.declare(&name);
            bus.emit(id, payload_json);
        })
        .map_err(LuaError::external)?;
        Ok(())
    })
}

fn make_spawn(
    vm: &Lua,
    scene: SceneHandle,
    generation: u64,
    current_generation: Rc<RefCell<u64>>,
    determinism: Rc<RefCell<DeterminismState>>,
) -> LuaResult<LuaFunction> {
    vm.create_function(move |lua, (type_name, components): (String, LuaTable)| {
        let comp_pairs = read_components(&components)?;
        let id = scene
            .with_mut(generation, |s| {
                let id = s.next_spawn_id(&type_name);
                let node = build_node(&type_name, comp_pairs, id.clone());
                s.add_node(node);
                id
            })
            .map_err(LuaError::external)?;
        let ud = lua.create_userdata(NodeRef::new(
            id,
            scene.clone(),
            Rc::clone(&current_generation),
            Rc::clone(&determinism),
        ))?;
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
        if lo > hi {
            return Err(LuaError::external(format!(
                "engine.rng(lo, hi) requires lo <= hi, got lo={lo} hi={hi}"
            )));
        }
        let span = (hi as i128) - (lo as i128) + 1;
        if span > (u64::MAX as i128) {
            return Err(LuaError::external(format!(
                "engine.rng range [{lo}, {hi}] exceeds u64 span; clamp the range or sample in two steps"
            )));
        }
        let mut state_ref = state.borrow_mut();
        *state_ref = state_ref
            .wrapping_mul(RNG_MUL)
            .wrapping_add(RNG_INC);
        let range = span as u64;
        let offset = *state_ref % range;
        Ok(lo + offset as i64)
    })
}

fn make_get_node(
    vm: &Lua,
    scene: SceneHandle,
    generation: u64,
    current_generation: Rc<RefCell<u64>>,
    determinism: Rc<RefCell<DeterminismState>>,
) -> LuaResult<LuaFunction> {
    vm.create_function(move |lua, id: String| {
        let exists = scene
            .with_ref(generation, |s| s.find_node(&id).is_some())
            .map_err(LuaError::external)?;
        if exists {
            let ud = lua.create_userdata(NodeRef::new(
                id,
                scene.clone(),
                Rc::clone(&current_generation),
                Rc::clone(&determinism),
            ))?;
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
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    let (k, _) = pair?;
                    if !matches!(k, LuaValue::Integer(_)) {
                        return Err(LuaError::external(format!(
                            "vec2 component values must be pure [x, y] arrays; found non-integer key {}",
                            value_type_name(&k)
                        )));
                    }
                }
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

fn lua_value_to_json(value: LuaValue) -> LuaResult<Value> {
    lua_value_to_json_safe(value, 0)
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
