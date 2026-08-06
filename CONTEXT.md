# Craft

Craft is an AI-native game engine: scenes, behaviors, and assets are structured data that AI agents and human authors create, loaded and executed by a deterministic single-threaded engine.

## Language

### Authoring

**Agent**:
An LLM that authors Craft content — scenes, behaviors, Lua classes, and assets — through the schema-validated API surface.
_Avoid_: bot, copilot (the copilot is the editor's agent UI, not the agent itself)

**Scene**:
The top-level JSON document describing a complete game world: a node tree with behaviors, resources, and settings.
_Avoid_: level (a level is one part of a scene), world

**Node**:
The single object type in the engine. Every entity is a Node; kinds differ only by which components they carry. There is no class inheritance.
_Avoid_: entity, actor, game object

**Component**:
A named value on a node — position, velocity, hitbox, sprite, and so on. Components are a node's entire data model.

**Behavior**:
Structured JSON logic attached to a node: a state machine, an on_tick handler, or an on_signal handler. Behaviors are the agent-authoring tier.
_Avoid_: script (scripts are Lua)

**Verb**:
One of the nine closed-set action words an agent may issue. Agents cannot invent new verbs.

**Lua class**:
Hot-reloadable Lua logic bound to a node's `lua_class` field. Lua is the human-authoring tier alongside behaviors.

**Hot reload**:
The default authoring loop: edit a scene, behavior, or Lua file and the running engine applies the diff without restart.

### Runtime

**Tick**:
One engine update at the configured tick rate. All logic — behaviors, Lua hooks, systems, signals — runs inside a tick.

**System**:
A registered logic pass over the scene, declared with `craft_system!` and running PreTick or PostTick. Physics, particles, and audio are systems.
_Avoid_: plugin, module

**Signal**:
A named event emitted by a node and delivered to subscribers on the next tick, never the same tick.

**Determinism**:
The guarantee that the same scene, seed, and input produce a byte-identical state hash on every tick.

**Replay**:
A recorded run — per-tick state hashes plus input frames — that can be re-run and verified byte-equal.
_Avoid_: demo, simulation

**Input state**:
The engine's input abstraction: a direction vector plus one action button, driveable by any renderer or test.

### Game surface

**Tilemap**:
A grid of tile IDs referencing a tileset atlas; the level representation for tile-based games.

**Pixel map**:
The JSON source of truth for a sprite: rows of color strings, baked to PNG by the asset generator.
_Avoid_: texture, image (PNGs are derived artifacts)

**Asset generator**:
The deterministic pipeline that converts pixel maps into committed binary assets — PNG sprites and WAV sounds.

### Evaluation

**Benchmark task**:
A reproducible agent evaluation: a fixed task spec (scene, seed, tick count) and a deterministic pass/fail verdict. Micro tasks exercise a single behavior; the platformer task is the first full-game task.
