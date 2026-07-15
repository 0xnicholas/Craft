use craft_kernel::craft_system;
use craft_kernel::inventory;
use craft_kernel::serde_json::json;
use craft_kernel::{Engine, System, SystemContext, SystemInfo, SystemPhase, SystemRegistry};
use std::cell::RefCell;
use std::rc::Rc;

struct CountingSystem {
    name: &'static str,
    phase: SystemPhase,
    counter: Rc<RefCell<u32>>,
}

impl CountingSystem {
    fn new(name: &'static str, phase: SystemPhase, counter: Rc<RefCell<u32>>) -> Self {
        Self {
            name,
            phase,
            counter,
        }
    }
}

impl System for CountingSystem {
    fn info(&self) -> SystemInfo {
        SystemInfo {
            name: self.name,
            phase: self.phase,
        }
    }

    fn run(&mut self, _ctx: &mut SystemContext<'_>) {
        *self.counter.borrow_mut() += 1;
    }
}

struct EmitSystem {
    name: &'static str,
    payload_signal: &'static str,
    payload_value: serde_json::Value,
}

impl System for EmitSystem {
    fn info(&self) -> SystemInfo {
        SystemInfo {
            name: self.name,
            phase: SystemPhase::Tick,
        }
    }

    fn run(&mut self, ctx: &mut SystemContext<'_>) {
        let sig = ctx
            .bus
            .resolve(self.payload_signal)
            .expect("signal must be declared");
        ctx.bus.emit(sig, self.payload_value.clone());
    }
}

struct ResourceReader {
    name: &'static str,
    uri: &'static str,
    sink: Rc<RefCell<Option<serde_json::Value>>>,
}

impl System for ResourceReader {
    fn info(&self) -> SystemInfo {
        SystemInfo {
            name: self.name,
            phase: SystemPhase::Tick,
        }
    }

    fn run(&mut self, ctx: &mut SystemContext<'_>) {
        let id = ctx
            .resources
            .resolve(self.uri)
            .expect("resource registered");
        let r = ctx.resources.get(id).unwrap();
        *self.sink.borrow_mut() = Some(r.data.clone());
    }
}

fn engine_with(systems: Vec<Box<dyn System>>) -> Engine {
    let mut engine = Engine::new();
    let mut reg = SystemRegistry::new();
    for s in systems {
        reg.push_boxed(s);
    }
    engine.systems = reg;
    engine
}

#[test]
fn list_systems_returns_registered_systems() {
    let counter = Rc::new(RefCell::new(0u32));
    let pre = Box::new(CountingSystem::new(
        "PreA",
        SystemPhase::PreTick,
        counter.clone(),
    ));
    let tick = Box::new(CountingSystem::new(
        "TickA",
        SystemPhase::Tick,
        counter.clone(),
    ));
    let post = Box::new(CountingSystem::new(
        "PostA",
        SystemPhase::PostTick,
        counter.clone(),
    ));

    let mut reg = SystemRegistry::new();
    reg.push_boxed(pre);
    reg.push_boxed(tick);
    reg.push_boxed(post);

    let infos = reg.list();
    let names: Vec<&str> = infos.iter().map(|i| i.name).collect();
    assert_eq!(names, vec!["PreA", "TickA", "PostA"]);
}

#[test]
fn engine_list_systems_surfaces_all_phases() {
    let c = Rc::new(RefCell::new(0u32));
    let mut engine = engine_with(vec![
        Box::new(CountingSystem::new("PreA", SystemPhase::PreTick, c.clone())),
        Box::new(CountingSystem::new("TickA", SystemPhase::Tick, c.clone())),
        Box::new(CountingSystem::new(
            "PostA",
            SystemPhase::PostTick,
            c.clone(),
        )),
    ]);

    let by_name: std::collections::HashMap<&str, SystemPhase> = engine
        .list_systems()
        .iter()
        .map(|i| (i.name, i.phase))
        .collect();
    assert_eq!(by_name["PreA"], SystemPhase::PreTick);
    assert_eq!(by_name["TickA"], SystemPhase::Tick);
    assert_eq!(by_name["PostA"], SystemPhase::PostTick);

    engine.tick();
    let total = *c.borrow();
    assert_eq!(total, 3, "PreTick + Tick + PostTick systems each ran once");
}

#[test]
fn signals_emitted_by_system_deliver_on_next_tick() {
    let mut engine = Engine::new();
    let hit = engine.bus.declare("hit");
    let counter = Rc::new(RefCell::new(0u32));
    let counter_inner = counter.clone();
    engine.bus.subscribe(hit, move |_| {
        *counter_inner.borrow_mut() += 1;
    });

    let mut reg = SystemRegistry::new();
    reg.push_boxed(Box::new(EmitSystem {
        name: "EmitA",
        payload_signal: "hit",
        payload_value: json!({"from": "EmitA"}),
    }));
    engine.systems = reg;

    engine.tick();
    assert_eq!(
        *counter.borrow(),
        0,
        "ADR 0003: signals emitted during a tick must NOT fire subscribers in the same tick"
    );

    engine.tick();
    assert_eq!(
        *counter.borrow(),
        1,
        "the signal queued during tick 1 is delivered at start of tick 2"
    );

    engine.tick();
    assert_eq!(
        *counter.borrow(),
        2,
        "EmitSystem fires every Tick phase, so each subsequent tick yields one delivery"
    );
}

#[test]
fn system_can_read_resource_registry_via_context() {
    let mut engine = Engine::new();
    engine
        .resources
        .register("res://test/data.json", json!({"hp": 100}));

    let sink = Rc::new(RefCell::new(None));
    let mut reg = SystemRegistry::new();
    reg.push_boxed(Box::new(ResourceReader {
        name: "Reader",
        uri: "res://test/data.json",
        sink: sink.clone(),
    }));
    engine.systems = reg;

    engine.tick();
    let seen = sink.borrow().clone();
    assert_eq!(seen, Some(json!({"hp": 100})));
}

#[test]
fn signal_payload_passes_through_subscriber() {
    let mut engine = Engine::new();
    let hit = engine.bus.declare("hit");

    let fired = Rc::new(RefCell::new(Vec::<String>::new()));
    let fired_inner = fired.clone();
    engine.bus.subscribe(hit, move |payload| {
        if let Some(s) = payload.get("from").and_then(|v| v.as_str()) {
            fired_inner.borrow_mut().push(s.to_string());
        }
    });

    let mut reg = SystemRegistry::new();
    reg.push_boxed(Box::new(EmitSystem {
        name: "EmitA",
        payload_signal: "hit",
        payload_value: json!({"from": "EmitA"}),
    }));
    engine.systems = reg;

    engine.tick();
    assert!(
        fired.borrow().is_empty(),
        "next-tick delivery: payload not yet delivered"
    );

    engine.tick();
    let events = fired.borrow().clone();
    assert_eq!(events, vec!["EmitA".to_string()]);
}

craft_system!(MacroSubmitTest, phase: Tick, {
    // empty body, used to verify the macro submits to inventory at link time
});

#[test]
fn craft_system_macro_submits_to_global_inventory() {
    let collected: Vec<_> = inventory::iter::<craft_kernel::SystemRegistration>().collect();
    let names: Vec<&str> = collected.iter().map(|r| r.info.name).collect();
    assert!(
        names.contains(&"MacroSubmitTest"),
        "craft_system! should submit a SystemRegistration: got {names:?}"
    );
}

#[test]
fn engine_state_consistent_across_ticks() {
    let c = Rc::new(RefCell::new(0u32));
    let pre_c = c.clone();
    let tick_c = c.clone();
    let post_c = c.clone();
    let mut engine = engine_with(vec![
        Box::new(CountingSystem::new("P", SystemPhase::PreTick, pre_c)),
        Box::new(CountingSystem::new("T", SystemPhase::Tick, tick_c)),
        Box::new(CountingSystem::new("Q", SystemPhase::PostTick, post_c)),
    ]);

    for expected in 1..=5u32 {
        engine.tick();
        assert_eq!(c.borrow().clone(), expected * 3);
        assert_eq!(engine.tick, expected as u64);
    }
}
