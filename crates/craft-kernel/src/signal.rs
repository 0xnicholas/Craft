use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(pub u32);

impl SignalId {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub id: SignalId,
    pub payload: Value,
}

type HandlerFn = Box<dyn Fn(&Value) + 'static>;

pub struct SignalBus {
    next_id: u32,
    by_name: HashMap<String, SignalId>,
    name_by_id: HashMap<SignalId, String>,
    handlers: HashMap<SignalId, Vec<HandlerFn>>,
    pending: Vec<Signal>,
}

impl std::fmt::Debug for SignalBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalBus")
            .field("next_id", &self.next_id)
            .field("signals", &self.name_by_id)
            .field("subscriber_counts", &self.handler_count_summary())
            .field("pending", &self.pending.len())
            .finish()
    }
}

impl SignalBus {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            by_name: HashMap::new(),
            name_by_id: HashMap::new(),
            handlers: HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub fn declare(&mut self, name: &str) -> SignalId {
        if let Some(&id) = self.by_name.get(name) {
            return id;
        }
        let id = SignalId(self.next_id);
        self.next_id += 1;
        self.by_name.insert(name.to_string(), id);
        self.name_by_id.insert(id, name.to_string());
        id
    }

    pub fn resolve(&self, name: &str) -> Option<SignalId> {
        self.by_name.get(name).copied()
    }

    pub fn name_of(&self, id: SignalId) -> Option<&str> {
        self.name_by_id.get(&id).map(String::as_str)
    }

    pub fn subscribe<F>(&mut self, signal: SignalId, handler: F)
    where
        F: Fn(&Value) + 'static,
    {
        self.handlers
            .entry(signal)
            .or_default()
            .push(Box::new(handler));
    }

    pub fn emit(&mut self, signal: SignalId, payload: Value) {
        self.pending.push(Signal {
            id: signal,
            payload,
        });
    }

    pub fn emit_by_name(&mut self, name: &str, payload: &Value) {
        let id = self.declare(name);
        self.emit(id, payload.clone());
    }

    pub fn drain(&mut self) -> Vec<Signal> {
        std::mem::take(&mut self.pending)
    }

    pub fn pending_signal_names(&self) -> Vec<String> {
        self.pending
            .iter()
            .filter_map(|s| self.name_of(s.id).map(String::from))
            .collect()
    }

    pub fn deliver_pending(&mut self) -> usize {
        let pending = self.drain();
        let count = pending.len();
        for signal in &pending {
            if let Some(handlers) = self.handlers.get(&signal.id) {
                for handler in handlers {
                    handler(&signal.payload);
                }
            }
        }
        count
    }

    pub fn delivered_names(&self) -> Vec<(String, Value)> {
        Vec::new()
    }

    pub fn deliver_and_collect(&mut self) -> Vec<(String, Value)> {
        let pending = self.drain();
        let mut delivered = Vec::new();
        for signal in pending {
            let name = self.name_of(signal.id).unwrap_or("unknown").to_string();
            if let Some(handlers) = self.handlers.get(&signal.id) {
                for handler in handlers {
                    handler(&signal.payload);
                }
            }
            delivered.push((name, signal.payload));
        }
        delivered
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn subscriber_count(&self, signal: SignalId) -> usize {
        self.handlers.get(&signal).map(Vec::len).unwrap_or(0)
    }

    fn handler_count_summary(&self) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = self
            .handlers
            .iter()
            .map(|(id, hs)| {
                let name = self
                    .name_by_id
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| format!("anon:{}", id.0));
                (name, hs.len())
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

impl Default for SignalBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn declare_assigns_unique_ids() {
        let mut bus = SignalBus::new();
        let a = bus.declare("player_died");
        let b = bus.declare("enemy_spawned");
        assert_ne!(a, b);
        assert_eq!(bus.resolve("player_died"), Some(a));
        assert_eq!(bus.resolve("enemy_spawned"), Some(b));
        assert_eq!(bus.name_of(a), Some("player_died"));
        assert_eq!(bus.name_of(b), Some("enemy_spawned"));
    }

    #[test]
    fn declare_is_idempotent() {
        let mut bus = SignalBus::new();
        let a = bus.declare("foo");
        let b = bus.declare("foo");
        assert_eq!(a, b);
    }

    #[test]
    fn subscribe_and_deliver_runs_handlers() {
        let mut bus = SignalBus::new();
        let sig = bus.declare("hit");

        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let log_a = log.clone();
        let log_b = log.clone();
        bus.subscribe(sig, move |payload| {
            log_a.borrow_mut().push(format!("a:{payload}"));
        });
        bus.subscribe(sig, move |payload| {
            log_b.borrow_mut().push(format!("b:{payload}"));
        });

        assert_eq!(bus.subscriber_count(sig), 2);

        bus.emit(sig, json!({"damage": 5}));
        assert_eq!(bus.pending_count(), 1);

        let delivered = bus.deliver_pending();
        assert_eq!(delivered, 1);
        assert_eq!(bus.pending_count(), 0);

        let entries = log.borrow().clone();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|s| s.starts_with("a:")));
        assert!(entries.iter().any(|s| s.starts_with("b:")));
    }

    #[test]
    fn emit_does_not_deliver_same_tick() {
        let mut bus = SignalBus::new();
        let sig = bus.declare("damage");

        let count = Rc::new(RefCell::new(0u32));
        let count_inner = count.clone();
        bus.subscribe(sig, move |_| {
            *count_inner.borrow_mut() += 1;
        });

        bus.emit(sig, json!(1));
        assert_eq!(
            *count.borrow(),
            0,
            "subscribers must not fire on the emit tick (next-tick delivery)"
        );
        bus.emit(sig, json!(2));
        assert_eq!(*count.borrow(), 0, "still no delivery on emit tick");

        bus.deliver_pending();
        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn emit_with_no_subscribers_is_harmless() {
        let mut bus = SignalBus::new();
        let sig = bus.declare("ghost");
        bus.emit(sig, json!(null));
        assert_eq!(bus.deliver_pending(), 1);
    }

    #[test]
    fn pending_signals_buffer_in_order() {
        let mut bus = SignalBus::new();
        let sig = bus.declare("step");
        let log = Rc::new(RefCell::new(Vec::<i64>::new()));
        let log_inner = log.clone();
        bus.subscribe(sig, move |payload| {
            if let Some(n) = payload.as_i64() {
                log_inner.borrow_mut().push(n);
            }
        });

        for n in 0..5 {
            bus.emit(sig, json!(n));
        }
        bus.deliver_pending();
        assert_eq!(*log.borrow(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn handlers_for_unrelated_signals_do_not_fire() {
        let mut bus = SignalBus::new();
        let a = bus.declare("a");
        let b = bus.declare("b");

        let count = Rc::new(RefCell::new(0u32));
        let count_inner = count.clone();
        bus.subscribe(a, move |_| *count_inner.borrow_mut() += 1);

        bus.emit(b, json!(null));
        bus.deliver_pending();
        assert_eq!(*count.borrow(), 0);
    }

    #[test]
    fn deliver_clears_pending() {
        let mut bus = SignalBus::new();
        let sig = bus.declare("x");
        bus.emit(sig, json!(null));
        bus.deliver_pending();
        assert_eq!(bus.pending_count(), 0);
        let again = bus.deliver_pending();
        assert_eq!(again, 0);
    }
}
