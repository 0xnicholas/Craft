pub mod behavior;
pub mod engine;
pub mod error;
pub mod evaluator;
pub mod hook;
pub mod hot_reload;
pub mod lint;
pub mod project;
pub mod render;
pub mod resource;
pub mod scene;
pub mod signal;
pub mod system;

pub use schemars;

pub use behavior::{
    Action, ActionCommand, Behavior, Expression, LogLevel, ResultTarget, StateDef, Target,
    Transition,
};
pub use engine::{Engine, EngineConfig, InputState};
pub use error::{
    AutoFix, EngineError, EngineResult, IoError, IoErrorKind, ParseError, ValidationError,
};
pub use evaluator::{
    Animation, EvalError, LogEntry, SceneState, Trigger, apply_commands,
    apply_commands_with_animations, evaluate_action, evaluate_behaviors, evaluate_dry_run,
    resolve_target,
};
pub use hook::EngineHook;
pub use hot_reload::{
    ComponentChange, HotReloadResult, HotReloader, SceneDiff, apply_scene_diff, compute_scene_diff,
    hot_reload_scene, reload_from_path,
};
pub use lint::{LintCode, LintSeverity, LintWarning, explain_node, lint};
pub use project::{PathsSection, Project, ProjectSection, ResolvedProject};
pub use render::{ComponentView, NullRenderer, Render, RenderCapabilities, Viewport};
pub use resource::{Resource, ResourceId, ResourceRef, ResourceRegistry};
pub use scene::{
    Component, ComponentKind, ComponentSpec, ComponentType, ComponentValue, Node, NodeDef,
    NodeRegistry, NodeTypeView, SCENE_KIND, Scene, hash_scene_state,
};
pub use signal::{Signal, SignalBus, SignalId};
pub use system::{
    System, SystemContext, SystemInfo, SystemPhase, SystemRegistration, SystemRegistry,
};

pub use craft_macros::{craft_node, craft_system};

pub use inventory;
pub use serde_json;
