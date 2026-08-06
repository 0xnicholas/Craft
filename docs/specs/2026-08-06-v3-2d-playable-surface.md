# v0.3.0 Playable Surface: Asset Generator + Animation + HUD + Audio + Windowed Lua

**Date**: 2026-08-06
**Status**: Draft (proposed for v0.3.0)
**Preceded by**: `docs/superpowers/specs/2026-08-06-v3-2d-surface-design.md` (milestone decisions)

## Problem Statement

The platformer is headless-verifiable but invisible: `craft-gpu` can open a window and render sprites, yet the game ships no sprites (no PNG exists anywhere in the repo), no animation (sprite frames are never advanced), no HUD text (nothing renders text), and no sound (audio exists but only at default volume, no looping, and no game assets trigger it). The asset pipeline is missing entirely: agents write data, but there is no data format for sprites and no deterministic path from that data to the PNGs the renderer loads. `craft-gpu` also never wires the Lua runtime, so scripted games like the platformer cannot run windowed at all.

## Solution

A deterministic **asset generator** (`craft-assets`) turns JSON **pixel maps** into committed PNGs (including multi-frame atlases and a bitmap font) and WAV tones. A new **animation system** (`craft-animation`) advances `frame` and `sprite_rect` components from an `animation` resource so the existing sprite renderer animates with zero renderer changes. The GPU renderer gains a **text pass** (bitmap font, screen-space HUD) and **Lua wiring**, and the audio system learns **volume + loop** from signal payloads. The platformer scene grows sprite/animation/text/audio components; benchmark 05's embedded reference is regenerated so CI keeps passing.

## User Stories

1. As a human player, I want to run the platformer in a window (F5 from the editor, or `cargo run -p craft-gpu -- games/platformer/scene.json`), so that the game is actually playable.
2. As an agent author, I want to declare sprites as JSON pixel maps, so that assets are data like scenes and behaviors.
3. As an agent author, I want to declare animation (frames, fps, loop) as data, so that characters move without hand-authoring texture state.
4. As a human author, I want to replace generated PNGs with hand-made art, so that the pipeline is a floor, not a ceiling.
5. As a player, I want HUD text (score, controls hint), so that the game communicates state.
6. As a player, I want jump/win sound effects and a looping background, so that the game gives audio feedback.
7. As a developer, I want the generator and the animation system deterministic, so that replay hashes and CI stay byte-stable.
8. As a benchmark maintainer, I want task 05 to keep passing after the game gains visuals, so that CI protects the playable game.
9. As a developer, I want the animation system headless-testable, so that CI covers it without a GPU.
10. As a developer, I want the HUD text layout as a pure function, so that it is testable without wgpu.
11. As an agent author, I want Lua classes to run in the windowed game, so that scripted games are playable, not just headless.
12. As a game author, I want `volume` and `loop` on audio signals, so that sound is mixable and music can loop.
13. As an editor user, I want F5 to launch the playable game with its assets and scripts, so that the authoring loop ends in a real window.
14. As a maintainer, I want the benchmark reference pinned to the shipped game files, so that asset/scene changes surface as a regeneration step, not silent drift.

## Implementation Decisions

### 1. Pixel map format and generator output

A pixel map is a JSON file: a palette mapping single characters to hex colors (`.` = transparent), rows of equal length, and a scale. Multi-frame sprites list frames; the generator packs them into a horizontal atlas.

```json
{
  "palette": { ".": null, "#": "#6C9FFF", "e": "#E74C3C" },
  "frames": [
    { "rows": ["###", "#.#", "###"] },
    { "rows": ["###", ".#.", "###"] }
  ],
  "scale": 4
}
```

Generator output per input file: `<name>.png` (the packed atlas) and `<name>.atlas.json` (per-frame source rects, in pixels after scaling). The generator also emits a built-in 5×7 bitmap font (`font.png` + `font.json`: char → src rect, digits + minimal ASCII) and simple WAV tones (square/triangle envelopes) for jump/coin/win. Generation is deterministic: fixed input → byte-identical output; no timestamps, no randomness, fixed PNG encoder settings.

### 2. Animation contract (zero renderer changes)

New crate `craft-animation` with `craft_system!(AnimationSystem, phase: PreTick)` (same shape as physics/particles/audio). A node declares `animation` (String, a `res://` resource):

```json
{ "atlas": "res://sprites/player.png", "frames": [[0,0,16,16], [16,0,16,16]], "fps": 8, "loop": true }
```

Each tick the system advances the node's `frame` (Int) and rewrites `sprite_rect` (Rect) from the frame list, using a per-node `frame_time` (Float) accumulator; `ticks_per_frame = ceil(60 / fps)` with the game's 60Hz tick. The existing `sprite_draw_command` already consumes `sprite` + `sprite_rect`, so the renderer is untouched. Systems run in deterministic order (phase, then name), preserving replay guarantees.

### 3. HUD text

`text` (String) + `text_pos` (Vec2, screen pixels) + `text_scale` (Float) components on any node; the GPU renderer draws the string with the bitmap font in screen space (camera-independent), one quad per character. Character→quad layout is a pure function (`font.json` + string + scale → quad list) tested without wgpu. Unknown characters render as a visible placeholder glyph.

### 4. Audio volume and loop

`AudioSystem` stops reading only signal names; it consumes `ctx.pending_signals` (name + payload) so a signal payload `{ "volume": 0.5, "loop": true }` controls playback (rodio `Sink::set_volume`, `repeat()`). Defaults stay as today (full volume, no loop) so existing games are unaffected.

### 5. Windowed Lua + physics

`craft-gpu` gains dependencies on `craft-lua` and `craft-physics` (link-time system registration, matching every game crate's pattern). `spawn_game_window` wires a `LuaRuntime` (same seed as the engine): scripts load from the scene file's sibling `scripts/` directory, classes bind by each node's `lua_class`, and the `LuaEngineHook` installs into the engine loop. `main.rs` sets `CRAFT_ASSET_ROOT` from `--asset-root` so the audio system finds WAVs.

### 6. Platformer scene and benchmark regeneration

The platformer scene gains: sprites (player, ground tile, goal), animation (run/jump frames), HUD text (controls hint), and audio (jump/win signals emitted from Lua; looping background). The game crate links `craft-audio` and `craft-animation`. Because the scene changes, `benchmarks/05_platformer.json`'s embedded reference is regenerated from the shipped files; the m10 drift test enforces it. Gameplay numbers (speed, gravity, jump) and the win condition stay byte-identical — 05 passes without touching its input script or assertions.

## Testing Decisions

What makes a good test here: external behavior only — a sprite file appears with the right pixels, a frame advances, a replay hash holds, a benchmark passes. Never test wgpu draw calls (CI has no GPU; the existing sprite path already has zero renderer tests — that is accepted status quo).

| Seam | Kind | Coverage |
|------|------|----------|
| `craft-animation` (new crate) | unit + integration | frame advance, fps pacing, loop wrap, no `animation` = untouched, determinism (replay byte-equal with animation active), ordering vs physics |
| `craft-assets` (new crate) | unit | pixel map parsing (bad palette, ragged rows), deterministic output (two runs byte-identical), PNG decodes at expected dimensions, atlas rects correct, font/wav emission |
| text layout pure function | unit | char→quad mapping, placeholder glyph, scale |
| `craft-audio` | unit | signal payload volume/loop parsed; no payload = legacy behavior; missing file = silent skip (existing pattern) |
| `games/platformer` integration (existing seam) | integration | after the visual upgrade the game still completes the scripted playthrough and replays byte-equal (existing tests unchanged) |
| m10 benchmark 05 (existing seam) | integration | reference regenerated; stub run still passes; drift test pins reference to shipped files |

Prior art: `craft-physics` unit tests, `crates/craft-replay/tests/physics_replay_integration.rs`, `games/platformer/tests/integration.rs`, `crates/craft-eval/tests/m10_integration.rs`.

## Out of Scope

- Gamepad/mouse input (keyboard suffices; deferred to a later version)
- Save/load, dialogue, rich text (only digits + minimal ASCII in the font)
- A sprite/atlas editor tool; pixel maps are authored by hand or by agents
- Advanced audio (streams, mixers, positional sound); volume + loop only
- Animated texture atlases beyond horizontal packing
- Any 3D or mesh work
- Changing benchmark 05's input script, win condition, or gameplay constants

## Further Notes

- Completes v0.3.0 acceptance item 4 ("Platformer ships as a playable game with generated PNG/WAV assets") from `docs/superpowers/specs/2026-08-06-v3-2d-surface-design.md`.
- Vocabulary follows `CONTEXT.md`: pixel map, asset generator, tilemap, input state, benchmark task, Lua class.
- `docs/specs/` is the home for milestone specs from now on (per maintainer preference); the older `docs/superpowers/specs/` entries remain as history.
- The generator is the same class of tool the benchmark harness assumes ("generate assets → headless run"); headless runs never render, so benchmark 05 remains renderer-independent.
