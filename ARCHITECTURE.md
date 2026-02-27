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
├── common/              Environment setup (light, ground, skybox)
├── menu/                Menu UI, state navigation
├── obstacle/            Obstacle data layer
│   ├── definition.rs    ObstacleId, ObstacleDef, TriggerVolumeConfig, CollisionVolumeConfig
│   ├── library.rs       ObstacleLibrary resource, RON load/save
│   └── spawning.rs      Spawn obstacles from glTF nodes, TriggerVolume/ObstacleCollisionVolume components
├── course/              Course data layer
│   ├── data.rs          CourseData, ObstacleInstance, PropKind, PropInstance, CameraInstance
│   └── loader.rs        Load/save/spawn courses from RON
├── editor/              Map editor
│   ├── workshop/        Define new obstacle types from glb scenes
│   │   ├── mod.rs       WorkshopPlugin, WorkshopState, preview spawning, gizmo
│   │   └── ui/          Workshop UI
│   │       ├── mod.rs   Re-exports
│   │       ├── build.rs UI hierarchy construction, marker components, constants
│   │       └── systems.rs Interaction handlers, text input, display updates
│   └── course_editor/   Place obstacles and props, set gate order
│       ├── mod.rs       CourseEditorPlugin, PlacementState, PlacedObstacle, PlacedProp, PlacedCamera, EditorTab, placement/selection
│       ├── overlays.rs  Visualization gizmos (trigger volumes, gate sequence, spline preview, prop gizmos, camera frustums)
│       ├── preview.rs   Camera PiP preview (render-to-texture, PreviewCamera, sync system)
│       ├── transform_gizmos.rs Move/rotate/scale widget systems
│       └── ui/          Course editor UI
│           ├── mod.rs   Re-exports
│           ├── types.rs Marker components, resources, color constants
│           ├── build.rs UI hierarchy construction (palette, panels)
│           ├── file_ops.rs Save/load/delete, navigation, gate ordering
│           └── systems.rs Interaction handlers, display updates, prop color
├── drone/               Drone simulation
│   ├── components.rs    Drone, PositionPid, AttitudePd, DesiredAttitude, DroneDynamics, DroneConfig, AIController, DesiredPosition
│   ├── physics.rs       hover_target, position_pid, attitude_controller, motor_lag, apply_forces, integrate_motion, clamp_transform (FixedUpdate)
│   ├── ai.rs            update_ai_targets, compute_racing_line, proximity_avoidance, update_wander_targets (FixedUpdate, spline-based)
│   ├── dev_dashboard.rs Toggleable UI panel (F4) for live-tuning AiTuningParams during races
│   ├── explosion.rs     Crash effects: debris + two-layer smoke (hot/dark) + audio (ExplosionParticle, ParticleKind, ExplosionSounds, ExplosionMeshes)
│   ├── fireworks.rs     Victory fireworks on first finish: placed emitter-based or auto gate 0 confetti + shell bursts (FireworkParticle, FireworkEmitter, FireworkMeshes, FireworkSounds, PendingShell)
│   ├── interpolation.rs PreviousTranslation/PreviousRotation, PhysicsTranslation/PhysicsRotation, visual transform interpolation (FixedFirst restore, FixedPostUpdate save, PostUpdate interpolate)
│   ├── paths.rs         RacePath, spline generation (race/drone/return), compute_start_positions, adaptive_approach_offset
│   └── spawning.rs      DroneAssets/DroneGltfHandle resources, load/setup/spawn systems, DRONE_COLORS/DRONE_NAMES
├── race/                Race mechanics
│   ├── gate.rs          GateIndex, GateForward, GatePlanes, plane-crossing gate detection
│   ├── collision.rs     ObstacleCollisionCache, swept segment vs OBB collision, crash_drone helper
│   ├── progress.rs      RaceProgress, per-drone state tracking
│   ├── timing.rs        RaceClock
│   └── lifecycle.rs     Countdown, finish detection
├── camera/              Camera modes
│   ├── chase.rs         Broadcast-style pack-follow camera (Chase mode, default in Race)
│   ├── fpv.rs           Stabilized close-follow camera on target drone (FPV mode)
│   ├── spectator.rs     RTS-style orbit controls: middle-mouse orbit, scroll zoom, WASD pan
│   ├── switching.rs     CameraMode/CameraState/CourseCameras, key switching (1-0=CourseCamera, 2=Chase, Shift+F=FPV, Shift+S=Spectator)
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

## Key Types

| Type | Kind | Module | Purpose |
|------|------|--------|---------|
| `ObstacleId` | Data | obstacle/definition | Unique string ID for obstacle types |
| `ObstacleDef` | Data | obstacle/definition | glb scene name + trigger volume config |
| `ObstacleLibrary` | Resource | obstacle/library | All loaded obstacle definitions |
| `CourseData` | Resource | course/data | All obstacle placements for a course |
| `TriggerVolume` | Component | obstacle/spawning | AABB hitbox on gate entities |
| `GateIndex` | Component | race/gate | Gate sequence order |
| `GateForward` | Component | race/gate | World-space forward direction for gate validation |
| `GatePlanes` | Resource | race/gate | Cached per-gate plane data (center, normal, axes, half-extents) built once at race start for plane-crossing detection |
| `CollisionVolumeConfig` | Data | obstacle/definition | Local-space AABB (offset + half_extents) for obstacle collision volumes |
| `ObstacleCollisionVolume` | Component | obstacle/spawning | Runtime collision volume on obstacle entities (offset, half_extents, is_gate) |
| `ObstacleCollisionCache` | Resource | race/collision | Cached world-space OBBs for all obstacles with collision volumes, built once at race start |
| `RaceProgress` | Resource | race/progress | Per-drone gate/finish/crash tracking |
| `DroneRaceState` | Data | race/progress | Per-drone state: next_gate, gates_passed, finished, finish_time, crashed, dnf_reason |
| `RacePhase` | Resource | race/lifecycle | WaitingToStart → Countdown → Racing → Finished |
| `CountdownTimer` | Resource | race/lifecycle | 3-second countdown timer (inserted on Countdown, removed on Racing) |
| `RaceClock` | Resource | race/timing | Elapsed race time, running flag |
| `CelMaterial` | Asset | rendering/cel_material | Cel-shading material with halftone transition and hue-shifted highlights/shadows |
| `SkyboxMaterial` | Asset | rendering/skybox | Procedural TRON night sky (stars, moon, neon horizon glow) |
| `CelLightDir` | Resource | rendering/mod | World-space light direction shared by all CelMaterial instances |
| `SkyboxEntity` | Component | rendering/skybox | Marker on the skybox sphere entity |
| `CameraState` | Resource | camera/switching | Current camera mode + FPV target drone standings index |
| `CameraMode` | Enum | camera/switching | Chase (pack follow), Fpv (drone-mounted), Spectator (free-fly), CourseCamera(usize) (placed cameras) |
| `CourseCameras` | Resource | camera/switching | Course camera entries built from CourseData at race start (primary first) |
| `CourseCameraEntry` | Data | camera/switching | Pre-computed Transform + optional label for a placed course camera |
| `CameraInstance` | Data | course/data | Serialized camera placement: translation, rotation, is_primary, optional label |
| `ChaseState` | Resource | camera/chase | Smoothed center/velocity for broadcast-style pack-follow camera |
| `SpectatorSettings` | Resource | camera/spectator | Movement speed + mouse sensitivity |
| `RaceResults` | Resource | race/progress | Snapshot of final standings, persists Race→Results state transition |
| `RaceResultEntry` | Data | race/progress | Per-drone result: index, finished, finish_time, crashed, gates_passed |
| `ResultsTransitionTimer` | Resource | race/lifecycle | Brief delay (0.5s) before auto-transitioning Race→Results |
| `AvailableCourses` | Resource | menu/ui | Discovered course files (Menu state only) |
| `SelectedCourse` | Resource | course/loader | User's course selection for racing |
| `WorkshopState` | Resource | editor/workshop | Current obstacle being edited (scene, trigger config, preview) |
| `PreviewObstacle` | Component | editor/workshop | Marker on the 3D preview entity in the workshop |
| `PlacementState` | Resource | editor/course_editor | Selected palette obstacle/prop, active tab, dragging entity, drag height, gate order mode |
| `PlacedObstacle` | Component | editor/course_editor | Marker on every obstacle entity spawned in the course editor; carries `obstacle_id` and `gate_order` |
| `PlacedProp` | Component | editor/course_editor | Marker on every prop entity spawned in the course editor; carries `PropKind` and optional `color_override` |
| `PlacedCamera` | Component | editor/course_editor | Marker on every camera entity spawned in the course editor; carries `is_primary` and optional `label` |
| `EditorTab` | Enum | editor/course_editor | Obstacles (default), Props, or Cameras — switches the left-panel palette |
| `PropEditorMeshes` | Resource | editor/course_editor/ui | Shared mesh+material handles for prop placeholder cubes in the editor |
| `CameraEditorMeshes` | Resource | editor/course_editor/ui | Shared mesh+material handles for camera placeholder cubes in the editor (sky/sunshine colors) |
| `CameraPreview` | Resource | editor/course_editor/preview | Holds camera entity for render-to-texture PiP preview |
| `PreviewCamera` | Component | editor/course_editor/preview | Marker on the secondary Camera3d used for PiP render-to-texture |
| `PropKind` | Enum | course/data | ConfettiEmitter or ShellBurstEmitter — firework emitter type |
| `PropInstance` | Data | course/data | Per-prop placement: kind, translation, rotation, optional color_override |
| `FireworkEmitter` | Component | drone/fireworks | Race-time marker entity spawned from course props; carries `PropKind` and optional `Color` override |
| `PreviousTranslation` | Component | drone/interpolation | Drone translation from previous FixedUpdate tick (for visual interpolation) |
| `PreviousRotation` | Component | drone/interpolation | Drone rotation from previous FixedUpdate tick (for visual interpolation) |
| `PhysicsTranslation` | Component | drone/interpolation | Authoritative physics translation saved after each FixedUpdate tick |
| `PhysicsRotation` | Component | drone/interpolation | Authoritative physics rotation saved after each FixedUpdate tick |
| `DroneAssets` | Resource | drone/spawning | Shared mesh/material handles for all drone entities (from glTF or placeholder) |
| `DroneGltfHandle` | Resource | drone/spawning | Handle to the loaded drone glTF asset |
| `DesiredPosition` | Component | drone/components | AI→PID bridge: target position + velocity hint + curvature-aware speed limit |
| `DronePhase` | Component | drone/components | Per-drone lifecycle: Idle, Racing, Returning, Wandering, Crashed |
| `WanderState` | Component | drone/components | Per-drone wandering state: target position, dwell timer, step counter |
| `WanderBounds` | Resource | drone/ai | Bounding box for post-race wandering area (computed from course obstacle positions + padding) |
| `ExplosionParticle` | Component | drone/explosion | Velocity, lifetime, remaining time, and `ParticleKind` (Debris/HotSmoke/DarkSmoke) for crash particles |
| `ExplosionSounds` | Resource | drone/explosion | 4 handles to explosion audio variants (assets/sounds/explosion_{1..4}.wav) |
| `ExplosionMeshes` | Resource | drone/explosion | Pre-allocated mesh handles for debris (3 sizes), hot smoke, dark smoke — shared across all explosions |
| `FireworkParticle` | Component | drone/fireworks | Velocity, lifetime, remaining time, and `FireworkKind` (Spark/Willow/Confetti) for victory firework particles |
| `FireworkMeshes` | Resource | drone/fireworks | Pre-allocated mesh handles for spark, willow, confetti particle sizes |
| `FireworkSounds` | Resource | drone/fireworks | Handle to firework burst audio (assets/sounds/firework.wav) |
| `FireworksTriggered` | Resource | drone/fireworks | Marker preventing re-triggering of fireworks after first drone finishes |
| `PendingShell` | Component | drone/fireworks | Staggered detonation timer for overhead shell bursts (position, delay, colors) |
| `ReturnPath` | Component | drone/components | Non-cyclic spline for post-race return flight (inserted Racing→Returning, removed Returning→Idle) |
| `AiTuningParams` | Resource | drone/components | Runtime-tunable AI/physics constants (14 params: speed, curvature, look-ahead, tilt, battery sag, dirty air strength, proximity avoidance radius/strength, velocity feedforward blend). Persists across race restarts. Exposed via dev dashboard (F4) |
| `LeaderboardRoot` | Component | race/ui | Marker on the race leaderboard panel (top-left standings display, 12 rows with color bars, names, times) |

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
└── courses/*.course.ron              Saved race courses
```

## Performance Design

- All drone physics in `FixedUpdate` (64Hz default), `.chain()`-ed for correctness. Visual rendering decoupled via PostUpdate transform interpolation (Previous* → Physics* slerp/lerp) for smooth motion between ticks without compromising physics determinism
- Gate trigger checks: O(drones) = O(12) plane-crossing tests per frame (each drone only checks its next expected gate)
- Obstacle collision checks: O(drones × obstacles) = O(12 × ~15) = ~180 slab tests/frame (~2000 flops, negligible)
- AI spline sampling: O(12) per fixed tick (polynomial eval per drone, 5 curvature samples for speed limiting)
- Dirty air perturbation: O(12²) = O(144) distance/dot-product checks per fixed tick (negligible)
- Proximity avoidance: O(12²) = O(144) distance checks per fixed tick (negligible)
- Firework particles: ~180 total (one-shot on first finish), peak ~80 alive at once, O(n) update
- No system ordering constraints between unrelated plugins — maximum parallelism
- `DespawnOnExit` for automatic entity cleanup on state transitions

## Drone Pipeline
```
Blender ──► drone.glb ──► DroneGltfHandle (OnEnter(Race) load)
                                │
                          DroneAssets (run_if drone_gltf_ready)
                                │
CourseData ──► generate_race_path() ──► base Catmull-Rom CubicCurve (editor preview)
        (paths.rs)  │
                    └──► generate_drone_race_path() ──► per-drone unique CubicCurve (12x)
                         (midleg lateral shift + gate 2D offset + approach scaling)
                                                                   │
                                                        spawn_drones() ──► 12 Drone entities
                                                                   │
                                                        FixedFirst: restore_physics_transforms (undo visual interp)
                                                        FixedPreUpdate: save_previous_transforms
                                                        FixedUpdate chain (11-system, thrust-through-body):
                                                        update_ai_targets → compute_racing_line
                                                        → proximity_avoidance → update_wander_targets
                                                        → hover_target → position_pid
                                                        → attitude_controller → dirty_air_perturbation → motor_lag
                                                        → apply_forces → integrate_motion → clamp_transform
                                                        FixedPostUpdate: save_physics_transforms
                                                        PostUpdate: interpolate_visual_transforms

                                                        Post-race: Racing → Returning (per-drone)
                                                        → generate_return_path() → non-cyclic spline
                                                        → smoothstep deceleration → return to start
                                                        → Returning → Idle (hover)

                                                        Results state: VictoryLap → Wandering
                                                        → deterministic waypoint generation (Fibonacci hash)
                                                        → dwell at waypoint → pick next → continuous flight
```

The physics model uses a **thrust-through-body** architecture: the drone's orientation determines its thrust direction (always body-up). A cascaded controller (outer position PID → inner attitude PD) drives orientation, and motor lag filters thrust changes. Quadratic drag and angular dynamics with moment of inertia produce realistic banking, braking, and hover behavior. Aerodynamic perturbations (dirty air from leading drones, prop wash on descent) add angular wobble that the PD must fight, producing visible instability in dirty air. Battery sag linearly reduces max thrust over the race duration.

Drone spawning uses `run_if` conditions with `AssetServer::is_loaded_with_dependencies()`: `setup_drone_assets` only runs when the glTF is fully loaded and `DroneAssets` doesn't yet exist; `spawn_drones` only runs when both `DroneAssets` and `CourseData` are available. The same pattern applies to course obstacle spawning (`obstacles_gltf_ready`) and workshop node list population.

## Testing

Unit tests cover the pure-logic data layers. Run with `cargo test`.

| Module | Tests | What's covered |
|--------|-------|----------------|
| `obstacle::library` | 8 | Insert/get, overwrite, save/load roundtrip, error cases, existing RON format |
| `course::loader` | 11 | Save/load roundtrip, empty course, transform preservation, error cases, existing RON format, delete course, camera roundtrip, backward compat (no cameras field) |
| `menu::ui` | 5 | Course discovery, filtering, sorting, path storage, missing directory |
| `camera::orbit` | 3 | Orbit distance, transform computation, look-at verification |
| `drone::paths` | 25 | Race path/spline generation (sort, filter, empty, single gate, passes-through-gates, tangent nonzero/alignment, flipped gate), per-drone path generation (paths differ, passes near gates, tangent alignment, gate offset 2D spread, neutral matches base), start positions (count, behind gate, hover height, gate width, no overlap, rotation), return path generation (valid spline, per-drone variation) |
| `drone::spawning` | 2 | Config randomization bounds (100-iteration stress test), PID variation application |
| `drone::ai` | 8 | safe_speed_for_curvature (zero/tiny/high/moderate curvature, lateral accel scaling), cyclic_curvature (circle constancy, straight-line low) |
| `race::progress` | 15 | Gate pass advancement, crash/finish recording, idempotency, is_active, standings sorting (finished by time, finished before crashed, crashed by gates passed) |
| `race::gate` | 16 | Point-in-trigger-volume AABB: identity, translated, rotated, scaled transforms (inside + outside). Plane-crossing: front-to-back pass, back-to-front rejection, horizontal/vertical out-of-bounds, same-side no-crossing, edge graze with margin, rotated gate, flipped gate |
| `race::collision` | 15 | segment_obb_intersection (through center, miss, parallel inside/outside, starts inside, too short, rotated OBB hit/miss, expansion widens hit, hit point on surface), point_in_gate_opening (center, inside bounds, outside width/height, different depth, rotated axes), integration (gate opening exempted, frame not exempted, miss entirely) |
| `rendering::cel_material` | 3 | Hue-shift algorithm: highlight warmth, shadow coolness, color clamping |

Functions used by tests:
- `ObstacleLibrary::load_from_file` / `save_to_file` — pure file I/O, no Bevy systems
- `load_course_from_file` / `save_course` / `delete_course` — pure file I/O, no Bevy systems
- `discover_courses_in(path)` — parameterized version of `discover_courses()` for testability
- `generate_race_path(course)` / `generate_drone_race_path(course, config, index)` / `compute_start_positions(...)` / `generate_return_path(...)` — pure geometry, no ECS (in `drone::paths`)
- `cyclic_curvature(spline, t, cycle_t)` / `safe_speed_for_curvature(κ, tuning)` — pure math (in `drone::ai`)
