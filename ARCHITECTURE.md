# Architecture

## Overview

Sonar Lark is a drone racing simulator with a built-in map editor. Users define obstacle types (importing 3D models from Blender), build race courses by placing obstacles, then simulate 12 AI drones racing through the course.

## State Machine

```
AppState::Menu ──► AppState::Editor ──► AppState::Race ──► AppState::Results
                        │                                         │
                        │                                         └──► AppState::Menu
                        ▼
                   EditorMode (SubStates)
                   ├── ObstacleWorkshop
                   └── CourseEditor
```

## Module Structure

```
src/
├── main.rs              App builder, plugin registration
├── states.rs            AppState, EditorMode
├── rendering/           Custom shaders and materials
│   ├── mod.rs           RenderingPlugin, CelLightDir resource
│   ├── cel_material.rs  CelMaterial (cel-shading with halftone + hue shifting)
│   └── skybox.rs        SkyboxMaterial, procedural TRON night sky
├── ui_theme.rs          Shared button styling constants, interaction helpers, spawn helpers
├── common/              Environment setup (light, ground, skybox)
│   ├── mod.rs           CommonPlugin, drone_identity re-export
│   └── drone_identity.rs DRONE_COLORS, DRONE_NAMES, DRONE_COUNT (shared across modules)
├── menu/                Menu UI, state navigation
│   ├── mod.rs           MenuPlugin, system registration
│   ├── discover.rs      Re-exports from course::discovery
│   └── ui.rs            Menu setup, course selection, button handlers
├── obstacle/            Obstacle data layer
│   ├── definition.rs    ObstacleId, ObstacleDef, TriggerVolumeConfig, CollisionVolumeConfig
│   ├── library.rs       ObstacleLibrary resource, RON load/save
│   └── spawning.rs      Spawn obstacles from glTF nodes, TriggerVolume/ObstacleCollisionVolume components
├── course/              Course data layer
│   ├── data.rs          CourseData, ObstacleInstance, PropKind, PropInstance, CameraInstance
│   ├── discovery.rs     CourseEntry, discover_courses(), discover_courses_in() + tests
│   └── loader.rs        Load/save/spawn courses from RON
├── editor/              Map editor
│   ├── workshop/        Define new obstacle types from glb scenes
│   │   ├── mod.rs       WorkshopPlugin, WorkshopState, lifecycle, node list population
│   │   ├── preview.rs   spawn_preview(), spawn_placeholder_preview()
│   │   ├── gizmos.rs    draw_trigger_gizmo(), draw_collision_gizmo(), draw_ground_gizmo()
│   │   ├── widgets.rs   Move/resize widget drawing and handling
│   │   └── ui/          Workshop UI
│   │       ├── mod.rs   Re-exports
│   │       ├── build.rs UI hierarchy construction, marker components, constants
│   │       └── systems.rs Interaction handlers, text input, display updates
│   └── course_editor/   Place obstacles and props, set gate order
│       ├── mod.rs       CourseEditorPlugin, PlacementState, PlacedObstacle, PlacedProp, PlacedCamera, EditorTab, placement/selection
│       ├── overlays.rs  Visualization gizmos (trigger volumes, gate sequence, spline preview, prop gizmos, camera frustums)
│       ├── preview.rs   Camera PiP preview (render-to-texture, PreviewCamera, sync system)
│       ├── transform_gizmos/ Move/rotate/scale widget systems
│       │   ├── mod.rs       Widget resource types, constants, sample_ring_screen_dist()
│       │   ├── move_gizmo.rs  draw_move_gizmo(), handle_move_gizmo()
│       │   ├── rotate_gizmo.rs rotation_axis_from_modifiers(), angle_in_plane(), draw/handle + tests
│       │   └── scale_gizmo.rs draw_scale_gizmo(), handle_scale_gizmo()
│       └── ui/          Course editor UI
│           ├── mod.rs   Re-exports
│           ├── types.rs Marker components, resources, re-exports CourseEntry
│           ├── discover.rs Re-exports from course::discovery
│           ├── left_panel.rs build_course_editor_ui(), build_left_panel(), tab/prop palette buttons
│           ├── right_panel.rs build_right_panel(), palette/course/action buttons, dividers
│           ├── data.rs  build_course_data(), next_gate_order_from_instances() + tests
│           ├── load.rs  load_course_into_editor(), handle_load_button(), auto_load_pending_course()
│           ├── save_delete.rs Navigation, save/delete flows, gate ordering
│           └── systems.rs Interaction handlers, display updates, prop color
├── dev_menu/            Development tools (accessible via Dev button on main menu)
│   ├── mod.rs           DevMenuPlugin, system registration
│   ├── portrait_config.rs PortraitColorSlot, PortraitPaletteConfig, RON persistence
│   ├── color_picker_data.rs PALETTE_COLORS (64 named sRGB colors from palette)
│   └── portrait_editor/ Portrait palette editor
│       ├── mod.rs       PortraitEditorState, EditorTab, component markers, setup/cleanup
│       ├── build.rs     UI hierarchy construction
│       ├── interaction.rs Button/click handlers (tabs, variants, colors, save, reset, pairing)
│       ├── grid.rs      Color grid rebuilding, spawn_color_cell, pairing picker
│       └── display.rs   Preview update, tab visuals, variant panel, warnings
├── pilot/               Procedural pilot system
│   ├── mod.rs           PilotPlugin, Pilot, PilotId, SelectedPilots, PilotConfigs, ColorScheme
│   ├── personality.rs   PersonalityTrait enum, trait modifiers, catchphrase pools
│   ├── skill.rs         SkillProfile, skill+personality → DroneConfig mapping
│   ├── gamertag.rs      Combinatorial gamertag generation
│   ├── roster.rs        PilotRoster resource, RON persistence, initial generation, portrait migration
│   └── portrait/        Portrait generation from master Inkscape SVG
│       ├── mod.rs       Re-exports from slot_enums + descriptor
│       ├── slot_enums.rs FaceShape, EyeStyle, MouthStyle, HairStyle, ShirtStyle, Accessory + ALL_* arrays
│       ├── descriptor.rs SecondaryColor, PortraitDescriptor, serde compat, color helpers, generate + tests
│       ├── loader.rs    PortraitParts resource, master SVG layer parser, hot-reload (F6)
│       ├── fragments.rs Per-layer hex color replacement, assemble_svg(), PortraitColors, LayerType
│       ├── rasterize.rs resvg pipeline: SVG → tiny_skia::Pixmap → Bevy Image (48×48)
│       └── cache.rs     PortraitCache resource (HashMap<PilotId, Handle<Image>>), setup system
├── drone/               Drone simulation
│   ├── components.rs    Drone, DroneIdentity, PositionPid, AttitudePd, DesiredAttitude, DroneDynamics, DroneConfig, AIController, DesiredPosition
│   ├── physics.rs       hover_target, position_pid, attitude_controller, motor_lag, apply_forces, integrate_motion, clamp_transform (FixedUpdate)
│   ├── ai/              AI targeting and racing line (FixedUpdate, spline-based)
│   │   ├── mod.rs       update_ai_targets, spline math helpers, curvature/speed functions
│   │   ├── racing_line.rs compute_racing_line (desired position from spline + gate correction)
│   │   └── proximity.rs proximity_avoidance (lateral dodge when drones are nearby)
│   ├── wander.rs        WanderBounds, wander_waypoint(), update_wander_targets(), transition_to_wandering()
│   ├── dev_dashboard.rs Toggleable UI panel (F4) for live-tuning AiTuningParams during races
│   ├── explosion.rs     Crash effects: debris + two-layer smoke (hot/dark) + audio
│   ├── fireworks.rs     Victory fireworks on first finish: placed emitter-based or auto gate 0
│   ├── interpolation.rs Visual transform interpolation (FixedFirst restore, FixedPostUpdate save, PostUpdate interpolate)
│   ├── paths/           Race path spline generation
│   │   ├── mod.rs       RacePath struct, extract_sorted_gates(), re-exports
│   │   ├── generation.rs generate_race_path(), generate_drone_race_path(), adaptive_approach_offset()
│   │   └── start_positions.rs compute_start_positions()
│   └── spawning.rs      DroneAssets/DroneGltfHandle resources, load/setup/spawn systems
├── race/                Race mechanics
│   ├── gate.rs          GateIndex, GateForward, GatePlanes, plane-crossing gate detection
│   ├── collision.rs     ObstacleCollisionCache, swept segment vs OBB collision, crash_drone helper
│   ├── collision_math.rs segment_obb_intersection(), point_in_gate_opening() — pure geometry
│   ├── progress.rs      RaceProgress, per-drone state tracking
│   ├── timing.rs        RaceClock
│   ├── lifecycle.rs     Countdown, finish detection
│   ├── start_button.rs  StartRaceButton, setup_race_ui(), countdown text
│   ├── overlays.rs      Race clock display, no-gates banner, open-editor button
│   ├── leaderboard.rs   Leaderboard panel (12 rows), fmt_time()
│   └── camera_hud.rs    Camera mode HUD overlay
├── camera/              Camera modes
│   ├── chase.rs         Broadcast-style pack-follow camera (Chase mode, default in Race)
│   ├── fpv.rs           Stabilized close-follow camera on target drone (FPV mode)
│   ├── spectator.rs     RTS-style orbit controls: middle-mouse orbit, scroll zoom, WASD pan
│   ├── switching.rs     CameraMode/CameraState/CourseCameras, key switching
│   ├── orbit.rs         Orbit math (shared between Spectator and Course Editor)
│   └── settings.rs      CameraSettings resource (FOV, sensitivity, zoom)
└── results/             Race results display
    ├── mod.rs           ResultsPlugin, cleanup
    └── ui.rs            Results screen UI (standings, RACE AGAIN, MAIN MENU buttons)
```

## Data Flow

### Obstacle Pipeline
```
Blender ──► obstacles.glb ──► Obstacle Workshop ──► default.obstacles.ron
                                                           │
                                                    ObstacleLibrary (Resource)
```

Each obstacle definition maps a human-readable ID to a named node (Blender object) inside the glb, plus an optional trigger volume configuration.

### Course Pipeline
```
ObstacleLibrary + Course Editor ──► *.course.ron
                                          │
                                    CourseData (Resource)
                                          │
                                    spawn_course() ──► Obstacle entities + TriggerVolume children
```

A course file stores obstacle references (by ObstacleId) with per-instance transforms and gate ordering, plus optional `props` (firework emitter placements) and `cameras` (placed camera positions with primary flag and labels). It does not duplicate obstacle definitions.

### Race Pipeline
```
CourseData ──► spawn obstacles + firework emitters + drones + build CourseCameras
                      │
              FixedFirst: restore authoritative transforms
              FixedPreUpdate: snapshot Previous* transforms
              FixedUpdate: AI targets → PID → forces → integration
              FixedPostUpdate: save authoritative Physics* transforms
                      │
              Update (chained): tick_countdown → tick_race_clock
                  → gate_trigger_check → obstacle_collision_check
                  → miss_detection → check_race_finished
                      │
              PostUpdate: interpolate drone transforms (Previous* → Physics*, alpha)
              All finished/crashed → RacePhase::Finished
```

## Assets

```
assets/
├── models/obstacles.glb              Single Blender file, all obstacle meshes
├── models/drone.glb                  Drone model (named node "Drone"), materials from glb
├── shaders/cel_halftone.wgsl         Cel-shading fragment shader (halftone dots, hue shifting)
├── shaders/tron_skybox.wgsl          Procedural TRON night skybox shader
├── sounds/explosion_{1..4}.wav       Crash explosion audio variants
├── sounds/firework.wav               Victory firework burst audio
├── library/default.obstacles.ron     Obstacle type definitions
├── courses/*.course.ron              Saved race courses
└── pilots/roster.pilots.ron          Pilot roster (auto-generated on first run)
```

## Performance Design

- All drone physics in `FixedUpdate` (64Hz default), `.chain()`-ed for correctness. Visual rendering decoupled via PostUpdate transform interpolation
- Gate trigger checks: O(12) plane-crossing tests per frame (each drone only checks its next expected gate)
- Obstacle collision checks: O(12 × ~15) = ~180 slab tests/frame (~2000 flops, negligible)
- AI spline sampling: O(12) per fixed tick (polynomial eval per drone, 5 curvature samples for speed limiting)
- Dirty air + proximity avoidance: O(12²) = O(144) distance checks per fixed tick (negligible)
- Firework particles: ~180 total (one-shot on first finish), peak ~80 alive at once, O(n) update
- No system ordering constraints between unrelated plugins — maximum parallelism
- `DespawnOnExit` for automatic entity cleanup on state transitions

## Detailed References

| Document | Contents |
|----------|----------|
| [`docs/types-reference.md`](docs/types-reference.md) | Full type/resource/component table (80+ entries) |
| [`docs/drone-system.md`](docs/drone-system.md) | Drone pipeline diagram, physics model, AI, effects |
| [`docs/race-system.md`](docs/race-system.md) | Gate detection, collision, race flow details |
| [`docs/editor-system.md`](docs/editor-system.md) | Workshop, course editor, props, cameras |
| [`docs/pilot-system.md`](docs/pilot-system.md) | Pilot roster, portraits, dev menu |
| [`docs/camera-system.md`](docs/camera-system.md) | Camera modes and switching |

## Testing

Unit tests cover the pure-logic data layers. Run with `cargo test`.

| Module | Tests | What's covered |
|--------|-------|----------------|
| `obstacle::library` | 8 | Insert/get, overwrite, save/load roundtrip, error cases, existing RON format |
| `course::loader` | 11 | Save/load roundtrip, empty course, transform preservation, error cases, existing RON format, delete course, camera roundtrip, backward compat (no cameras field) |
| `course::discovery` | 8 | Course discovery, filtering, sorting, path storage, gate counting, missing directory |
| `camera::orbit` | 3 | Orbit distance, transform computation, look-at verification |
| `drone::paths` | 23 | Race path/spline generation, per-drone path generation, start positions (split: generation.rs + start_positions.rs) |
| `drone::spawning` | 2 | Config randomization bounds, PID variation application |
| `drone::ai` | 9 | safe_speed_for_curvature, cyclic_curvature (split: mod.rs + racing_line.rs + proximity.rs) |
| `dev_menu::color_picker_data` | 2 | PALETTE_COLORS count and value range |
| `race::progress` | 15 | Gate pass advancement, crash/finish recording, idempotency, standings sorting |
| `race::gate` | 16 | Point-in-trigger-volume AABB, plane-crossing detection |
| `race::collision` | 15 | segment_obb_intersection, point_in_gate_opening, integration tests |
| `pilot::gamertag` | 4 | Unique tags generation, non-empty, reasonable length, leetspeak |
| `pilot::personality` | 5 | Catchphrase pools, modifier bounds, incompatibility symmetry |
| `pilot::skill` | 4 | Config within bounds, high skill tighter ranges, traits modify config |
| `pilot::roster` | 10 | Save/load roundtrip, roster size, unique IDs, backward compat, migration |
| `pilot::mod` | 1 | ColorScheme roundtrip |
| `pilot::portrait::descriptor` | 10 | Generation bounds, HSL roundtrip, serde compat |
| `pilot::portrait::slot_enums` | 6 | Enum reachability (all variants covered by ALL_* arrays) |
| `pilot::portrait::loader` | 9 | Master SVG layer parsing, PortraitParts get/get_by_label |
| `pilot::portrait::fragments` | 18 | Color helpers, SVG assembly, per-layer color replacement |
| `pilot::portrait::rasterize` | 7 | Solid color images, unpremultiply alpha, rasterization output |
| `rendering::cel_material` | 3 | Hue-shift algorithm: highlight warmth, shadow coolness |

Functions used by tests:
- `ObstacleLibrary::load_from_file` / `save_to_file` — pure file I/O, no Bevy systems
- `load_course_from_file` / `save_course` / `delete_course` — pure file I/O, no Bevy systems
- `discover_courses_in(path)` — parameterized version of `discover_courses()` for testability (in `course::discovery`)
- `generate_race_path(course)` / `generate_drone_race_path(course, config, index)` / `compute_start_positions(...)` — pure geometry, no ECS (in `drone::paths`)
- `cyclic_curvature(spline, t, cycle_t)` / `safe_speed_for_curvature(κ, tuning)` — pure math (in `drone::ai`)
