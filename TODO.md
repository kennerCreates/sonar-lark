# TODO

## Procedural Pilot System

### Phase 1 — Pilot Data Model, Personality & Persistence ✅
Complete. `Pilot` struct with gamertag, personality traits (8 variants with `DroneConfig` modifiers), skill profiles (level + per-axis variation), stats, and placeholders for Phases 2–3. Combinatorial gamertag generator (6 styles, collision-free). `PilotRoster` (24 pilots) persisted as RON, loaded at startup, saved after each race. `SelectedPilots`/`PilotConfigs` resources created per race from roster. `DroneIdentity` component on each drone. Leaderboard, results UI, and crash effects all read pilot data. 21 pilot-specific tests passing.

### Phase 2 — SVG Portrait Generation ✅
Complete. Pilot avatars using hand-drawn Inkscape SVG fragments assembled at runtime, rasterized via `resvg` 0.47, and displayed as Bevy `Image` textures in UI. `portrait/` submodule with 4 files: `mod.rs` (data model, 6 slot enums including `ShirtStyle`, `PortraitDescriptor` with `generate()`), `fragments.rs` (hand-drawn Inkscape SVG fragments with viewBox `"9.5 11.5 20.1 20.1"`, `assemble_svg()`, `shirt_fragment()`, color helpers with `SHIRT_COLOR` token, portrait-03 fragments use `translate(-28,0)` wrapper), `rasterize.rs` (resvg pipeline -> Bevy `Image`), `cache.rs` (`PortraitCache` resource, 48x48 at race start). Layer order: bg -> hair_back -> face -> shirt -> eyes -> mouth -> hair_front -> accessory. Accessory reduced to 4 variants (Necklace, SpikedCollar, Piercings, Earring) with serde aliases for backward compat. Fallback mappings for old enum variants (Long->Oval, Diamond->Angular, Goggles->Wide, Winking->Normal, Gritted->Frown, Helmet->Beanie, Bald->ShortCrop, Ponytail->LongSwept). Portraits displayed in leaderboard (16x16) and results (20x20) with fallback solid-color squares. Backward-compatible: Phase 1 rosters auto-backfill portraits via deterministic seeding.

### Pre-Phase-3 Refactoring

#### 1. ~~Fix delete-course state reset bug + extract `reset_editor_state()` helper~~ ✅

#### 2. ~~Split `spawn_drones` into sub-functions~~ ✅

#### 3. ~~Shared `GRAVITY` constant~~ ✅

#### 4. ~~Move `POINTS_PER_GATE` and `FINISH_EXTENSION` to `common/`~~ ✅

#### 5. ~~Extract drone name/color resolution helper~~ ✅

#### 6. ~~Promote `fmt_time()` to shared utility~~ ✅

#### 7. ~~Unify button visual handlers with `ThemedButton` marker~~ ✅

#### 8. Re-enable or remove disconnected firework sounds
`FireworkSounds` resource is loaded (`fireworks.rs:70`) but playback is disconnected (`fireworks.rs:459`). The `let _ = &firework_sounds;` is a warning suppression hack.

- [ ] Either re-enable the sound playback or remove the resource, its loading system, and the `let _` line.

Files: `drone/fireworks.rs`, `drone/mod.rs`

### Phase 3 — Modular Drone Models
- [ ] Model part library in Blender: frames (5-6), arms (4), canopies (3-4), optional extras (prop guards, antennas)
- [ ] Single `.glb` with named nodes (follows obstacle library pattern)
- [ ] Runtime assembly: select parts per pilot's drone build descriptor, parent into entity hierarchy
- [ ] Apply pilot color scheme via `CelMaterial` color remapping
- [ ] ~6×4×4×3 = 288+ visual combos before color variation

#### Refactoring (during Phase 3)
- [ ] Replace `DroneAssets` with a part-aware `DronePartLibrary` — keyed by slot name (frame/arms/canopy/extra), with per-part transforms. Current flat `Vec<Handle<Mesh>>` + single `mesh_transform` cannot represent modular parts. (`drone/spawning.rs:27-31`)
- [ ] Expand `ColorScheme` to multi-color — add `#[serde(default)] secondary: Option<[f32; 3]>` (and possibly `accent`) for two-tone liveries. Currently only `primary`. (`pilot/mod.rs:55-57`)
- [ ] Wire `DroneBuildDescriptor` through to spawning — add `drone_build: DroneBuildDescriptor` to `SelectedPilot` so the spawn pipeline can read part selections. Currently the field exists on `Pilot` but is dropped at the selection bridge. (`pilot/mod.rs:118-122`, `drone/spawning.rs`)
- [ ] Bundle `crash_drone` parameters into a `CrashContext` struct — currently 13 params (`race/collision.rs:120-163`). Phase 3 may add modular debris, further stressing this signature.

### Phase 4 — Circuit Reputation & Pilot Attraction (Design TBD)
- [ ] Reputation system for the player's circuit
- [ ] Pilot attraction mechanics (higher reputation → higher-skilled pilots)
- [ ] Possible multi-factor preferences (course difficulty, track fame, etc.)
- [ ] Circuit management meta-game loop

#### Refactoring (before or during Phase 4)
- [ ] Bundle `spawn_obstacle` params into `SpawnObstacleContext` system param — currently 16 params, causing 15-16 param bloat in `load_course_into_editor`, `handle_load_button`, `auto_load_pending_course`. Collapse the 8 gltf/material handles into one struct. (`obstacle/spawning.rs`, `editor/course_editor/ui/load.rs`)
- [ ] Unify `PlacedFilter` usage — `type PlacedFilter` exists at `course_editor/mod.rs:94` but is inlined 5 more times in `save_delete.rs` and `load.rs`. Make `pub` and use everywhere. Critical before post-MVP "multiple obstacle types beyond gates" or terrain.
- [ ] Rename `EditorTab` in portrait editor to `PortraitEditorTab` — name collision with `editor/types.rs::EditorTab`. No runtime issue but confuses codebase search.
- [ ] Remove dead code: `catchphrases()` (`personality.rs:125`), `clear_complementary_for()` (`portrait_config.rs:210`), `MouthStyle::index()` (`slot_enums.rs:111`), `ObstacleMarker::id` field (`obstacle/spawning.rs:9`). Re-enable or remove disconnected `FireworkSounds` (`fireworks.rs:459`).
- [ ] Delete redundant single-line re-export files: `menu/discover.rs`, `editor/course_editor/ui/discover.rs`. Import from `crate::course::discovery` directly.
- [ ] Replace glob re-exports (`pub use submod::*`) with named re-exports in `workshop/ui/mod.rs` and `portrait_editor/mod.rs`.
- [ ] Deduplicate `PANEL_BG` — `dev_dashboard.rs:7` redefines the identical value from `ui_theme::PANEL_BG`. Import instead.

---

## Future (Post-MVP)
- [ ] Player-controlled drone (same throttle/pitch/roll/yaw interface as AI)
- [ ] Per-drone customization (motor thrust, weight, drag, frame size)
- [ ] Multiple obstacle types beyond gates
- [ ] Multi-lap races
- [ ] Terrain elevation
- [ ] Gamepad support
