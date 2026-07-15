use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::resource::ResourceRegistry;
use crate::signal::SignalBus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemPhase {
    PreTick,
    Tick,
    PostTick,
}

impl SystemPhase {
    pub const ALL: [Self; 3] = [Self::PreTick, Self::Tick, Self::PostTick];

    pub fn rank(self) -> u8 {
        match self {
            Self::PreTick => 0,
            Self::Tick => 1,
            Self::PostTick => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub name: &'static str,
    pub phase: SystemPhase,
}

pub struct SystemContext<'a> {
    pub bus: &'a mut SignalBus,
    pub resources: &'a ResourceRegistry,
    pub tick: u64,
}

pub trait System: 'static {
    fn info(&self) -> SystemInfo;
    fn run(&mut self, ctx: &mut SystemContext<'_>);
}

pub struct SystemRegistration {
    pub info: SystemInfo,
    pub instantiate: fn() -> Box<dyn System>,
}

inventory::collect!(SystemRegistration);

pub fn collected_systems() -> Vec<SystemRegistration> {
    inventory::iter::<SystemRegistration>()
        .map(|reg| SystemRegistration {
            info: SystemInfo {
                name: reg.info.name,
                phase: reg.info.phase,
            },
            instantiate: reg.instantiate,
        })
        .collect()
}

#[derive(Default)]
pub struct SystemRegistry {
    instances: Vec<Box<dyn System>>,
    by_name: HashMap<String, usize>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_boxed(&mut self, system: Box<dyn System>) {
        let name = system.info().name.to_string();
        self.by_name.insert(name, self.instances.len());
        self.instances.push(system);
    }

    pub fn instantiate_all(&mut self) {
        self.instances.clear();
        self.by_name.clear();
        let mut regs = collected_systems();
        regs.sort_by(|a, b| {
            a.info
                .phase
                .rank()
                .cmp(&b.info.phase.rank())
                .then_with(|| a.info.name.cmp(b.info.name))
        });
        for reg in regs {
            let name = reg.info.name.to_string();
            let instance = (reg.instantiate)();
            self.by_name.insert(name, self.instances.len());
            self.instances.push(instance);
        }
    }

    pub fn list(&self) -> Vec<SystemInfo> {
        self.instances.iter().map(|s| s.info()).collect()
    }

    pub fn run_phase(&mut self, phase: SystemPhase, ctx: &mut SystemContext<'_>) {
        for system in &mut self.instances {
            if system.info().phase == phase {
                system.run(ctx);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_phase_rank_orders_correctly() {
        assert!(SystemPhase::PreTick.rank() < SystemPhase::Tick.rank());
        assert!(SystemPhase::Tick.rank() < SystemPhase::PostTick.rank());
    }

    #[test]
    fn empty_registry_runs_nothing() {
        let reg = SystemRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn phase_all_lists_every_phase() {
        assert_eq!(SystemPhase::ALL.len(), 3);
        assert!(SystemPhase::ALL.contains(&SystemPhase::PreTick));
        assert!(SystemPhase::ALL.contains(&SystemPhase::Tick));
        assert!(SystemPhase::ALL.contains(&SystemPhase::PostTick));
    }

    #[test]
    fn system_info_constructs_with_metadata() {
        let info = SystemInfo {
            name: "Foo",
            phase: SystemPhase::Tick,
        };
        assert_eq!(info.name, "Foo");
        assert_eq!(info.phase, SystemPhase::Tick);
    }
}
