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
├── common/              Environment setup (light, ground)
├── menu/                Menu UI, state navigation
├── obstacle/            Obstacle data layer
│   ├── definition.rs    ObstacleId, ObstacleDef, TriggerVolumeConfig
│   ├── library.rs       ObstacleLibrary resource, RON load/save
│   └── spawning.rs      Spawn obstacles from glTF nodes, TriggerVolume component
├── course/              Course data layer
│   ├── data.rs          CourseData, ObstacleInstance
│   └── loader.rs        Load/save/spawn courses from RON
├── editor/              Map editor
│   ├── workshop/        Define new obstacle types from glb scenes
│   │   ├── mod.rs       WorkshopPlugin, WorkshopState, preview spawning, gizmo
│   │   └── ui.rs        Workshop UI layout, interaction handlers, text input
│   └── course_editor/   Place obstacles, set gate order
│       ├── mod.rs       CourseEditorPlugin, PlacementState, PlacedObstacle, placement/drag/gizmo systems
│       └── ui.rs        Palette UI, save/load, gate order mode, name field
├── drone/               Drone simulation
│   ├── components.rs    Drone, PositionPid, AttitudePd, DesiredAttitude, DroneDynamics, DroneConfig, AIController, DesiredPosition
│   ├── physics.rs       hover_target, position_pid, attitude_controller, motor_lag, apply_forces, integrate_motion, clamp_transform (FixedUpdate)
│   ├── ai.rs            update_ai_targets, compute_racing_line, proximity_avoidance (FixedUpdate, spline-based)
│   ├── dev_dashboard.rs Toggleable UI panel (F4) for live-tuning AiTuningParams during races
│   ├── explosion.rs     Crash explosion particles + audio (ExplosionParticle, ExplosionSounds, spawn/update systems)
│   └── spawning.rs      DroneAssets/DroneGltfHandle resources, load/setup/spawn systems, RacePath/spline generation
├── race/                Race mechanics
│   ├── gate.rs          GateIndex, trigger volume overlap detection
│   ├── progress.rs      RaceProgress, per-drone state tracking
│   ├── timing.rs        RaceClock
│   └── lifecycle.rs     Countdown, finish detection
├── camera/              Camera modes
│   ├── spectator.rs     Free-fly WASD camera
│   ├── fpv.rs           First-person drone-mounted
│   ├── chase.rs         Third-person follow
│   └── switching.rs     CameraMode/CameraState, mode switching
└── results/             Race results display
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

A course file stores obstacle references (by ObstacleId) with per-instance transforms and gate ordering. It does not duplicate obstacle definitions.

### Race Pipeline
```
CourseData ──► spawn obstacles + drones
                      │
              FixedUpdate: AI targets → PID → forces → integration
                      │
              Update (chained): tick_countdown → tick_race_clock
                  → gate_trigger_check → miss_detection → check_race_finished
                      │
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
| `RaceProgress` | Resource | race/progress | Per-drone gate/finish/crash tracking |
| `DroneRaceState` | Data | race/progress | Per-drone state: next_gate, gates_passed, finished, finish_time, crashed, dnf_reason |
| `RacePhase` | Resource | race/lifecycle | WaitingToStart → Countdown → Racing → Finished |
| `CountdownTimer` | Resource | race/lifecycle | 3-second countdown timer (inserted on Countdown, removed on Racing) |
| `RaceClock` | Resource | race/timing | Elapsed race time, running flag |
| `CameraState` | Resource | camera/switching | Current mode + target drone |
| `SpectatorSettings` | Resource | camera/spectator | Movement speed + mouse sensitivity |
| `AvailableCourses` | Resource | menu/ui | Discovered course files (Menu state only) |
| `SelectedCourse` | Resource | course/loader | User's course selection for racing |
| `WorkshopState` | Resource | editor/workshop | Current obstacle being edited (scene, trigger config, preview) |
| `PreviewObstacle` | Component | editor/workshop | Marker on the 3D preview entity in the workshop |
| `PlacementState` | Resource | editor/course_editor | Selected palette obstacle, dragging entity, drag height, gate order mode |
| `PlacedObstacle` | Component | editor/course_editor | Marker on every obstacle entity spawned in the course editor; carries `obstacle_id` and `gate_order` |
| `DroneAssets` | Resource | drone/spawning | Shared mesh/material handles for all drone entities (from glTF or placeholder) |
| `DroneGltfHandle` | Resource | drone/spawning | Handle to the loaded drone glTF asset |
| `DesiredPosition` | Component | drone/components | AI→PID bridge: target position + velocity hint + curvature-aware speed limit |
| `DronePhase` | Component | drone/components | Per-drone lifecycle: Idle, Racing, Returning, Crashed |
| `ExplosionParticle` | Component | drone/explosion | Velocity, lifetime, remaining time for crash particles |
| `ExplosionSounds` | Resource | drone/explosion | 4 handles to explosion audio variants (assets/sounds/explosion_{1..4}.wav) |
| `ReturnPath` | Component | drone/components | Non-cyclic spline for post-race return flight (inserted Racing→Returning, removed Returning→Idle) |
| `AiTuningParams` | Resource | drone/components | Runtime-tunable AI/physics constants (14 params: speed, curvature, look-ahead, tilt, battery sag, dirty air strength, proximity avoidance radius/strength, velocity feedforward blend). Persists across race restarts. Exposed via dev dashboard (F4) |

## Assets

```
assets/
├── models/obstacles.glb              Single Blender file, all obstacle meshes
├── models/drone.glb                  Drone model (named node "Drone"), materials from glb
├── library/default.obstacles.ron     Obstacle type definitions
└── courses/*.course.ron              Saved race courses
```

## Performance Design

- All drone physics in `FixedUpdate` (64Hz default), `.chain()`-ed for correctness
- Gate trigger checks: O(drones × gates) = O(12 × ~20) = O(240) AABB tests per frame
- AI spline sampling: O(12) per fixed tick (polynomial eval per drone, 5 curvature samples for speed limiting)
- Dirty air perturbation: O(12²) = O(144) distance/dot-product checks per fixed tick (negligible)
- Proximity avoidance: O(12²) = O(144) distance checks per fixed tick (negligible)
- No system ordering constraints between unrelated plugins — maximum parallelism
- `DespawnOnExit` for automatic entity cleanup on state transitions

## Drone Pipeline
```
Blender ──► drone.glb ──► DroneGltfHandle (Startup load)
                                │
                          DroneAssets (Update poll until loaded)
                                │
CourseData ──► generate_race_path() ──► base Catmull-Rom CubicCurve (editor preview)
                │
                └──► generate_drone_race_path() ──► per-drone unique CubicCurve (12x)
                     (midleg lateral shift + approach scaling from DroneConfig)
                                                               │
                                                    spawn_drones() ──► 12 Drone entities
                                                               │
                                                    FixedUpdate chain (11-system, thrust-through-body):
                                                    AI targets (spline projection) → racing line (spline sampling)
                                                    → proximity_avoidance → hover_target → position_pid
                                                    → attitude_controller → dirty_air_perturbation → motor_lag
                                                    → apply_forces → integration → clamp

                                                    Post-race: Racing → Returning (per-drone)
                                                    → generate_return_path() → non-cyclic spline
                                                    → smoothstep deceleration → return to start
                                                    → Returning → Idle (hover)
```

The physics model uses a **thrust-through-body** architecture: the drone's orientation determines its thrust direction (always body-up). A cascaded controller (outer position PID → inner attitude PD) drives orientation, and motor lag filters thrust changes. Quadratic drag and angular dynamics with moment of inertia produce realistic banking, braking, and hover behavior. Aerodynamic perturbations (dirty air from leading drones, prop wash on descent) add angular wobble that the PD must fight, producing visible instability in dirty air. Battery sag linearly reduces max thrust over the race duration.

Drone spawning uses an async polling pattern: `setup_drone_assets` and `spawn_drones` run every `Update` frame, returning early until the glTF asset and `CourseData` are both available. Once drones spawn, the early-return guards make them no-ops.

## Testing

Unit tests cover the pure-logic data layers. Run with `cargo test`.

| Module | Tests | What's covered |
|--------|-------|----------------|
| `obstacle::library` | 8 | Insert/get, overwrite, save/load roundtrip, error cases, existing RON format |
| `course::loader` | 9 | Save/load roundtrip, empty course, transform preservation, error cases, existing RON format, delete course |
| `menu::ui` | 5 | Course discovery, filtering, sorting, path storage, missing directory |
| `camera::orbit` | 3 | Orbit distance, transform computation, look-at verification |
| `drone::spawning` | 19 | Race path/spline generation (sort, filter, empty, single gate, passes-through-gates, tangent nonzero, adaptive approach offset), per-drone path generation (paths differ, passes near gates, tangent alignment, neutral matches base), start positions (count, behind gate, no overlap), config randomization bounds (incl. cornering/braking/attitude/racing-line variation), PID variation, return path generation (valid spline, per-drone variation) |
| `race::progress` | 15 | Gate pass advancement, crash/finish recording, idempotency, is_active, standings sorting (finished by time, finished before crashed, crashed by gates passed) |
| `race::gate` | 8 | Point-in-trigger-volume AABB: identity, translated, rotated, scaled transforms (inside + outside) |

Functions used by tests:
- `ObstacleLibrary::load_from_file` / `save_to_file` — pure file I/O, no Bevy systems
- `load_course_from_file` / `save_course` / `delete_course` — pure file I/O, no Bevy systems
- `discover_courses_in(path)` — parameterized version of `discover_courses()` for testability
- `generate_race_path(course)` / `generate_drone_race_path(course, config, index)` / `compute_start_positions(...)` / `generate_return_path(...)` — pure geometry, no ECS
