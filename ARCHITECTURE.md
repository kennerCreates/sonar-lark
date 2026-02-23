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
│   ├── ai.rs            update_ai_targets, compute_racing_line (FixedUpdate, spline-based)
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
              Update: gate trigger checks → RaceProgress updates
                      │
              All finished/crashed → AppState::Results
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
| `RaceProgress` | Resource | race/progress | Per-drone gate/finish tracking |
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
| `DesiredPosition` | Component | drone/components | AI→PID bridge: target position + velocity hint |

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
- AI spline sampling: O(12) per fixed tick (polynomial eval per drone)
- No system ordering constraints between unrelated plugins — maximum parallelism
- `DespawnOnExit` for automatic entity cleanup on state transitions

## Drone Pipeline
```
Blender ──► drone.glb ──► DroneGltfHandle (Startup load)
                                │
                          DroneAssets (Update poll until loaded)
                                │
CourseData ──► generate_race_path() ──► Catmull-Rom CubicCurve (cyclic)
                                                               │
                                                    spawn_drones() ──► 12 Drone entities
                                                               │
                                                    FixedUpdate chain (9-system, thrust-through-body):
                                                    AI targets (spline projection) → racing line (spline sampling)
                                                    → hover_target → position_pid → attitude_controller
                                                    → motor_lag → apply_forces → integration → clamp
```

The physics model uses a **thrust-through-body** architecture: the drone's orientation determines its thrust direction (always body-up). A cascaded controller (outer position PID → inner attitude PD) drives orientation, and motor lag filters thrust changes. Quadratic drag and angular dynamics with moment of inertia produce realistic banking, braking, and hover behavior.

Drone spawning uses an async polling pattern: `setup_drone_assets` and `spawn_drones` run every `Update` frame, returning early until the glTF asset and `CourseData` are both available. Once drones spawn, the early-return guards make them no-ops.

## Testing

Unit tests cover the pure-logic data layers. Run with `cargo test`.

| Module | Tests | What's covered |
|--------|-------|----------------|
| `obstacle::library` | 8 | Insert/get, overwrite, save/load roundtrip, error cases, existing RON format |
| `course::loader` | 9 | Save/load roundtrip, empty course, transform preservation, error cases, existing RON format, delete course |
| `menu::ui` | 5 | Course discovery, filtering, sorting, path storage, missing directory |
| `camera::orbit` | 3 | Orbit distance, transform computation, look-at verification |
| `drone::spawning` | 13 | Race path/spline generation (sort, filter, empty, single gate, passes-through-gates, tangent nonzero), start positions (count, behind gate, no overlap), config randomization bounds, PID variation |

Functions used by tests:
- `ObstacleLibrary::load_from_file` / `save_to_file` — pure file I/O, no Bevy systems
- `load_course_from_file` / `save_course` / `delete_course` — pure file I/O, no Bevy systems
- `discover_courses_in(path)` — parameterized version of `discover_courses()` for testability
- `generate_race_path(course)` / `compute_start_positions(...)` — pure geometry, no ECS
