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
│   └── spawning.rs      Spawn obstacles from glTF, TriggerVolume component
├── course/              Course data layer
│   ├── data.rs          CourseData, ObstacleInstance
│   └── loader.rs        Load/save/spawn courses from RON
├── editor/              Map editor
│   ├── workshop/        Define new obstacle types from glb scenes
│   └── course_editor/   Place obstacles, set gate order
├── drone/               Drone simulation
│   ├── components.rs    Drone, PidController, DroneDynamics, DroneConfig, AIController
│   ├── physics.rs       PID-lite physics (FixedUpdate)
│   ├── ai.rs            AI target tracking, racing line variation
│   └── spawning.rs      Spawn 12 drones with randomized configs
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

Each obstacle definition maps a human-readable ID to a named scene inside the glb, plus an optional trigger volume configuration.

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

## Assets

```
assets/
├── models/obstacles.glb              Single Blender file, all obstacle meshes
├── library/default.obstacles.ron     Obstacle type definitions
└── courses/*.course.ron              Saved race courses
```

## Performance Design

- All drone physics in `FixedUpdate` (64Hz default), `.chain()`-ed for correctness
- Gate trigger checks: O(drones × gates) = O(12 × ~20) = O(240) AABB tests per frame
- AI waypoint updates: O(12) per fixed tick
- No system ordering constraints between unrelated plugins — maximum parallelism
- `DespawnOnExit` for automatic entity cleanup on state transitions
