# Claude Instructions

## Project Context

This is a **drone racing simulator with a built-in map editor**, built in **Rust** using **Bevy 0.18.0**. See `ARCHITECTURE.md` for the full module structure, data flow, and design decisions.

Key facts:
- **State machine**: `AppState` (Menu → Editor → Race → Results) with `EditorMode` SubStates (ObstacleWorkshop, CourseEditor)
- **Physics**: Thrust-through-body quadrotor with cascaded PID (position → attitude) in `FixedUpdate`, 12 AI drones with per-drone variation. Quadratic drag, angular dynamics with moment of inertia, motor lag (25ms). Aerodynamic perturbations (dirty air, prop wash), battery sag.
- **Data**: Obstacle definitions and courses serialized as RON. Single `.glb` file for all obstacle 3D models, separate `.glb` for drone model.
- **Gate validation**: Trigger volumes (AABB), gates must be passed in order. Hit/miss = crash.

## MCP Tools

A `fetch` MCP server is configured for this project. Use it to look up current Bevy documentation before writing or suggesting Bevy code:

- API reference: `https://docs.rs/bevy/0.18.0/bevy/`
- Bevy book: `https://bevyengine.org/learn/book/`
- Migration guides: `https://bevyengine.org/learn/migration-guides/`

Do not rely solely on prior training knowledge for Bevy APIs — fetch and verify against the 0.18.0 docs, especially for systems, queries, rendering, and ECS patterns that change frequently between versions.

## Communication Style

- **Reports and communication**: Be thorough and detailed. Include context, reasoning, trade-offs, and relevant implications so decisions can be made with full information.
- **Code documentation**: Be concise. Prefer self-documenting code over comments. Only comment where logic is non-obvious.

## Handling Ambiguity

When a prompt is genuinely ambiguous, ask clarifying questions before proceeding. Format them as multiple choice where possible to keep responses quick. Do not ask for clarification on things that have a clear, obviously correct interpretation — use judgment and proceed.

## Performance

The performance target is a stable **60fps**. Performance is paramount.

- If a feature request would risk this target, **flag it immediately** before implementing. Explain specifically why it is a risk (e.g., expensive per-frame system queries, O(n²) algorithms in hot paths, excessive entity/component churn) and propose at least one alternative approach.
- If existing code is encountered that poses a performance risk, flag it proactively even if it wasn't the subject of the current task.
- Prefer solutions with predictable, bounded performance over those with better average-case but worse worst-case behavior.
- Be mindful of Bevy-specific pitfalls: over-querying, large archetypes, and unnecessary system ordering constraints that block parallelism.

## Project Conventions

- **Plugins**: each module exposes a single plugin (struct or fn). Plugins register their own systems with appropriate state guards via `run_if(in_state(...))`.
- **State scoping**: use `DespawnOnExit` on entities that should be cleaned up when leaving a state. Do not manually despawn in `OnExit` unless there's a reason.
- **Serialization**: all persistent data uses RON via serde. Obstacle library lives at `assets/library/default.obstacles.ron`, courses at `assets/courses/*.course.ron`, pilot roster at `assets/pilots/roster.pilots.ron`.
- **Asset loading**: all obstacle models come from a single `assets/models/obstacles.glb`. Individual objects are accessed via `Gltf::named_nodes` → `GltfNode` → `GltfMesh` → primitives, using the Blender object name. Each obstacle spawns a parent entity with child `Mesh3d`/`MeshMaterial3d` per primitive.
- **Physics**: all physics systems run in `FixedUpdate` with `.chain()` for ordering. No physics in `Update`. Visual transform interpolation in `PostUpdate` smooths drone rendering between 64Hz physics ticks (does not affect physics determinism).
- **Rendering**: All visible geometry (ground, obstacles, drones) uses `CelMaterial` — a custom material with cel-shading, halftone dot transitions, and hue-shifted highlights/shadows. `CelLightDir` resource stores the world-space light direction, computed once at startup. Use `cel_material_from_color(base_color, light_dir)` to create materials. Explosion particles remain `StandardMaterial` (unlit emissive). Skybox is a `SkyboxMaterial` on an inverted sphere (front-face culled), procedural TRON-style night sky. Custom WGSL shaders live in `assets/shaders/`.
- **Bevy 0.18 specifics**: `set_parent_in_place()` (not `set_parent()`), `SceneRoot` (not `SceneBundle`), `Mesh3d`/`MeshMaterial3d` components, `ChildSpawnerCommands` (not `ChildBuilder`) for `with_children` closures in commands context, `AccumulatedMouseMotion`/`AccumulatedMouseScroll` from `bevy::input::mouse` (not in prelude). `MessageReader<T>` (not `EventReader<T>`) for reading events. `KeyboardInput` from `bevy::input::keyboard`. System tuples max ~12 elements for `run_if`; split larger groups into multiple `add_systems` calls. `Gltf::named_nodes`/`named_scenes` use `Box<str>` keys (not `String`).
- **Menu pattern**: `AvailableCourses` created `OnEnter(Menu)`, removed `OnExit`. `SelectedCourse` persists across states into Race.
- **Workshop pattern**: `WorkshopState` created `OnEnter(ObstacleWorkshop)`, removed `OnExit`. `PreviewObstacle` entities manually despawned on exit. Node list populated via `run_if(obstacles_gltf_ready)` + `run_if(workshop_nodes_pending)`; placeholder cube if no glb match.
- **Pilot pattern**: `PilotRoster` resource loaded at `Startup` (persists entire app lifetime). `SelectedPilots` + `PilotConfigs` created `OnEnter(Race)` (12 random pilots from roster), removed `OnExit(Results)`. `SelectedPilots` provides gamertags/colors to UI; `PilotConfigs` provides pre-computed `DroneConfig` values to `spawn_drones()`. After each race, `update_pilot_stats_after_race` updates pilot stats (wins, finishes, crashes, best times) and saves roster to disk. `DroneIdentity` component on each drone entity carries name + color. Personality traits (`PersonalityTrait` enum) map to `DroneConfig` modifiers via `TraitModifiers`. Skill profiles (`SkillProfile`) compress randomization ranges toward optimal values at higher skill levels. Gamertags are combinatorial (prefixes + roots + suffixes + style variations).
- **Portrait pattern**: `portrait/` submodule with 5 files. `PortraitDescriptor` has 6 slot enums (FaceShape/EyeStyle/MouthStyle/HairStyle/ShirtStyle/Accessory) + color fields (including `shirt_color` derived via `derive_shirt_color()`). `PortraitDescriptor::generate(rng, primary_color)` creates randomized portraits. Accessory has 4 variants: EarringRound, EarringRing, NecklaceChain, NecklacePendant (with serde aliases for old names: Necklace, SpikedCollar, Piercings, Earring, etc.). `loader.rs` parses a single master Inkscape SVG (`assets/portraits/pilot-portraits.svg`) at startup, extracting layers by `inkscape:label` into `PortraitParts` resource. `fragments.rs` assembles portrait SVGs from `PortraitParts` fragments with per-layer hex color replacement: BLACK (#000000) = primary color, WHITE (#ffffff) = secondary color, per layer type (face: skin_tone/skin_highlight, hair: hair_color, eyes: hair_color/eye_color, mouth: skin_highlight/vanilla, shirt: shirt_color, accessory: acc_color/acc_shadow). Global replacements: #808080 → VANILLA (#f2f2da), #333333 → BLACK (#000000). Background layer: #808080 → bg_color. ViewBox is `"0 0 20 20"`. Layer order (back to front): bg, hair_back, face, shirt, eyes, mouth, hair_front, accessory. Fallback mappings collapse old enum variants to new ones (e.g., Long->Oval, Goggles->Wide, Helmet->Beanie). `rasterize.rs` uses `resvg` 0.47 to render SVG -> `tiny_skia::Pixmap` -> Bevy `Image` (48x48). Hot-reload: F6 re-reads master SVG and invalidates `PortraitCache`. `PortraitCache` resource (`HashMap<PilotId, Handle<Image>>`) built `OnEnter(Race)` via chained `setup_portrait_cache` system (after `select_pilots_for_race`), persists across races. Roster migration: `backfill_empty_portraits()` auto-generates portraits for Phase 1 pilots using deterministic RNG seeded by `PilotId`. Portraits displayed in leaderboard (`LbPortrait` component, 16x16) and results screen (20x20 `ImageNode`), with solid-color fallback.
- **Drone pattern**: `DroneGltfHandle` loaded `OnEnter(Race)`, `DroneAssets` extracted via `run_if(drone_gltf_ready)` + `run_if(not(resource_exists::<DroneAssets>))`, `spawn_drones` gated by `run_if(resource_exists::<DroneAssets>)` + `run_if(resource_exists::<CourseData>)`. 12 drones with `DespawnOnExit(AppState::Race)`. Physics: 11-system `.chain()` in FixedUpdate (thrust-through-body model). Each drone gets a unique spline via `generate_drone_race_path()`. Per-drone variation via `DroneConfig` (generated from pilot skill+personality when `PilotConfigs` is available, else random). `RaceSeed` resource randomizes outcomes between races. Resources cleaned up `OnExit(Race)`. Visual transform interpolation: `PhysicsTranslation`/`PhysicsRotation` store authoritative post-tick state; `restore_physics_transforms` (FixedFirst) undoes visual interpolation before physics, `save_physics_transforms` (FixedPostUpdate) captures result, `interpolate_visual_transforms` (PostUpdate) blends `Previous*` → `Physics*` using `overstep_fraction` for smooth rendering.
- **Asset readiness pattern**: `drone_gltf_ready()` and `obstacles_gltf_ready()` are run-condition functions using `AssetServer::is_loaded_with_dependencies()`. Used to gate systems that depend on loaded glTF assets, replacing per-frame polling with `run_if` conditions.
- **Course obstacle cleanup**: `spawn_obstacle()` adds `DespawnOnExit(AppState::Race)`. `CourseSpawned` marker removed `OnExit(Race)`.
- **Drone lifecycle**: `DronePhase` tracks state: `Idle → Racing → Returning → Idle` (or `Racing → Crashed` on DNF). On Results entry: `VictoryLap → Wandering` (deterministic waypoints via Fibonacci hashing, `WanderState` component, `WanderBounds` resource from course extents). Crashed drones: hidden, zero velocity, skipped by physics/AI. `ReturnPath` component holds return spline. `drones_are_active()` run condition keeps AI alive during Racing + Returning + Wandering.
- **Race leaderboard**: `LeaderboardRoot` panel (top-left, `DespawnOnExit`). 12 rows with `LbColorBar`/`LbNameText`/`LbTimeText`. Updated from `RaceProgress::standings()` each frame. Names/colors sourced from `SelectedPilots` resource (falls back to `DRONE_NAMES`/`DRONE_COLORS` in `drone/spawning.rs` if no pilots).
- **Race validation**: `RacePhase`: `WaitingToStart → Countdown → Racing → Finished`. Race logic: 7-system `.chain()` in Update. Gate detection uses plane-crossing (line segment vs gate plane, directional, tunneling-proof). `obstacle_collision_check` tests swept segments vs OBBs (gate openings exempted). `miss_detection` is a safety net for drones that skip gates. `GatePlanes` + `ObstacleCollisionCache` resources cached at race start. Gate 0 is both start and finish (full lap).
- **Obstacle collision**: `CollisionVolumeConfig` in RON (`#[serde(default)]`), `ObstacleCollisionVolume` component on parent entity. `ObstacleCollisionCache` built at race start (world-space OBBs). Swept segment vs OBB slab test with drone radius expansion (`DRONE_COLLISION_RADIUS = 0.35`). Gate openings exempted via `point_in_gate_opening` (infinite-depth tube). `crash_drone()` shared helper (used by both `miss_detection` and `obstacle_collision_check`). `DnfReason::ObstacleCollision` variant.
- **Course delete pattern**: `PendingCourseDelete` tracks deletion with inline Yes/Cancel confirmation. Resets editor state if deleted course is currently loaded.
- **AI path following**: Each drone follows a unique cyclic Catmull-Rom spline (3 control points per gate). Per-drone variation: gate pass offset, approach scaling, midleg lateral bias (all deterministic via Fibonacci hashing + `RaceSeed`). `POINTS_PER_GATE = 3.0`. Full lap: all gates + gate 0 again. Curvature-aware speed limiting + gate correction blending + adaptive look-ahead. Requires >= 2 gates.
- **Dev dashboard**: `AiTuningParams` resource (14 tunable params, persists across restarts). F4 toggles dashboard UI in Race. `PARAM_META` defines display names, step sizes, ranges. Index-based get/set for UI.
- **Gate directionality**: `TriggerVolumeConfig.forward` (default `NEG_Z`), per-instance `gate_forward_flipped` flag. `GateForward` component stores world-space forward. Editor shows cyan arrow gizmo; `F` key flips direction.
- **Aerodynamic perturbations**: Dirty air (wake cone wobble behind leading drones) + prop wash (descent wobble). Deterministic sine waves, no RNG. Battery sag reduces thrust linearly over race. All tunable via dev dashboard.
- **Proximity avoidance**: Lateral dodge when drones are nearby. O(12²) per tick. Gate-proximity suppression prevents dodging near gate openings. Tunable via dev dashboard.
- **Explosion effects**: Three particle layers (debris, hot smoke, dark smoke) using pre-allocated `ExplosionMeshes`. 4 random explosion sounds. Loaded `OnEnter(Race)`, cleaned up `OnExit(Race)`. All `StandardMaterial` (unlit emissive).
- **Firework effects**: Confetti fan + staggered shell bursts on first finish. `FireworkEmitter` entities from course props (or auto at gate 0). Pre-allocated `FireworkMeshes`. `FireworksTriggered` prevents re-fire. Particles use `DespawnOnExit(AppState::Results)`.
- **Course props**: `PropInstance` in `CourseData.props` with `PropKind` (ConfettiEmitter/ShellBurstEmitter), optional `color_override`. `#[serde(default)]` for backward compat. Editor uses tabbed UI (`EditorTab::Obstacles`/`Props`/`Cameras`), `PlacedProp` component, `PlacedFilter` type alias for shared queries.
- **Course cameras**: `CameraInstance` in `CourseData.cameras` with `translation`, `rotation`, `is_primary`, optional `label`. `#[serde(default)]` for backward compat. Editor "Cameras" tab with `PlacedCamera` component, frustum gizmo visualization (primary=sunshine, normal=sky). `CameraEditorMeshes` resource for placeholder cubes. Primary toggle enforces single-primary. Soft cap warning at >9 cameras. PiP preview (384x216 render-to-texture) appears when a `PlacedCamera` is selected — `PreviewCamera` entity with `RenderTarget::Image`, `CameraPreview` resource, auto-hidden when deselected. `preview.rs` module in `editor/course_editor/`.
- **Flight spline preview**: In CourseEditor, `draw_flight_spline_preview` generates the race spline and draws it as gizmo lines colored green (fast) → red (slow curvature). Uses `cyclic_curvature()`/`safe_speed_for_curvature()` from `drone::ai`. Requires >= 2 gates.
- **Camera switching**: `CameraState` holds `CameraMode` (Chase/Fpv/Spectator/CourseCamera(usize)). Keys: 1-9,0=CourseCamera(0..8) if present (else 1=fallback Chase), 2=Chase always, Shift+F=FPV (cycles target on repeat), Shift+S=Spectator. `CourseCameras` resource built from `CourseData.cameras` at race start (primary at index 0). Default mode: `CourseCamera(0)` if cameras exist, else Chase. Each mode has its own update system gated by `camera_mode_is()` / `camera_mode_is_course_camera()`. Chase follows leader with pack blending. FPV is stabilized close-follow. Spectator is RTS orbit (middle-mouse, scroll, WASD). CourseCamera snaps to stored transform. `CameraHudRoot` shows mode/hints with dynamic camera labels.
- **Results pattern**: `RaceResults` snapshot built from `RaceProgress::to_race_results()` before `Race → Results`. `ResultsTransitionTimer(0.5s)` delays transition. UI with `DespawnOnExit(AppState::Results)`. `SelectedCourse` persists for "RACE AGAIN".
- **Dev menu pattern**: `AppState::DevMenu` accessed via "Dev" button on main menu. `dev_menu/` module with `portrait_config.rs` (data model + persistence) and `portrait_editor.rs` (UI + systems). `PortraitPaletteConfig` resource with per-slot vetoed colors and complementary mappings, persisted to `assets/dev/portrait_palette.ron`. `PortraitEditorState` resource tracks active tab, variant/color selections, dirty flag. Editor has part tabs (Face/Eyes/Mouth/Hair/Shirt/Accessory), variant radio buttons, 8x8 primary color grid (left-click select, right-click veto), secondary color grid for slots with `has_secondary()` (Skin/Eye/Accessory), live 128x128 preview. `generate_with_config()` on `PortraitDescriptor` picks from non-vetoed `PALETTE_COLORS` and respects complementary mappings. `generate_initial_roster()` loads palette config from disk.

## Post-Phase Checklist

After completing each implementation phase:

1. **Update documentation**: Review and update `TODO.md`, `ARCHITECTURE.md`, and `CLAUDE.md` with any new types, patterns, or conventions introduced.
2. **Review warnings**: Run `cargo build` and review all warnings. Fix any that indicate real issues (unused imports, unnecessary mut, etc.). Warnings for types/functions that are planned for upcoming phases in the current sprint are acceptable and should be left alone — do not suppress them with `#[allow(dead_code)]`.
3. **Run clippy**: Run `cargo clippy -- -D warnings` and fix all lints. Clippy catches idiomatic issues, performance pitfalls, and common mistakes that `rustc` alone misses.
4. **Run tests**: Run `cargo test` and verify all tests pass. Add tests for new pure-logic functions.
5. **Manual testing feedback form**: After all automated checks pass, present a structured feedback form for manual testing. The form must include:
   - A checklist of every manually-testable behavior introduced or changed in the phase (specific actions, expected results).
   - Edge cases and error scenarios to verify (e.g., invalid input, rapid state transitions, boundary values).
   - Performance observations to watch for (frame drops, hitches, visual artifacts).
   - A "Pass / Fail / Notes" column for each item so the user can report results inline.
   - Regression checks: key existing behaviors that should still work unchanged.

   Format the form as a markdown table or checklist that can be filled out directly in chat.

## Testing Conventions

- Tests live in `#[cfg(test)] mod tests` at the bottom of each source file (idiomatic Rust).
- Use `tempfile` crate (dev-dependency) for filesystem tests — never write to the real `assets/` directory.
- Test pure logic and serialization (file I/O, data structures, discovery). ECS systems are tested manually.
- When adding file I/O functions, provide a parameterized version that accepts a `&Path` so tests can use temp directories (e.g., `discover_courses_in(path)` vs `discover_courses()`).
