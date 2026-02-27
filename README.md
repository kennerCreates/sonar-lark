# Sonar Lark

A drone racing simulator with a built-in map editor, built in Rust using [Bevy 0.18](https://bevyengine.org/).

Define obstacle types by importing 3D models from Blender, build race courses by placing obstacles, then watch 12 AI drones race through the course.

## Features

- **Map Editor** — Obstacle Workshop for defining obstacle types from `.glb` models, and a Course Editor for placing obstacles, props, and gates with drag/rotate/scale controls
- **12 AI Drones** — Thrust-through-body quadrotor physics with cascaded PID control, per-drone variation (cornering aggression, braking, attitude damping), dirty air/prop wash perturbations, and battery sag
- **Race System** — Gate validation with plane-crossing detection (tunneling-proof), per-drone timing, DNF on missed gates, countdown sequence, and a results screen with standings
- **Cel-Shaded Rendering** — Custom WGSL shaders with halftone dot transitions, hue-shifted highlights/shadows, and a procedural TRON-style night skybox
- **Camera Modes** — Chase camera (pack-follow), FPV (drone-mounted with target cycling), and Spectator (RTS orbit)
- **Victory Effects** — Confetti fans and staggered shell bursts from course-placed firework emitter props
- **Dev Dashboard** — F4 toggleable tuning panel with 14 parameters for AI behavior, aerodynamics, and proximity avoidance

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)
- Assets: `assets/models/obstacles.glb` (obstacle meshes) and `assets/models/drone.glb` (drone model), exported from Blender

### Build & Run

```sh
cargo run
```

Dev builds use `opt-level = 1` for the crate with `opt-level = 3` for dependencies (fast incremental compiles, decent runtime performance). For a lean release build:

```sh
cargo run --release
```

### Data Files

- Obstacle definitions: `assets/library/default.obstacles.ron`
- Courses: `assets/courses/*.course.ron`
- Shaders: `assets/shaders/`

All persistent data is serialized as [RON](https://github.com/ron-rs/ron).

## Application Flow

```
Menu  ──►  Editor  ──►  Race  ──►  Results
              │                       │
              ▼                       └──► Menu
         ObstacleWorkshop
         CourseEditor
```

## Controls

| Context | Key | Action |
|---------|-----|--------|
| Race | 1 | Chase camera |
| Race | 2 | Spectator camera |
| Race | 3 | FPV camera (cycles target) |
| Race | F4 | Toggle dev dashboard |
| Editor | F | Flip gate direction |
| Editor | Q / E | Adjust height |

## Development Milestones

- **Skeleton** — State machine, common systems, main.rs wiring
- **Data Layer** — Obstacle and course data with RON serialization
- **Obstacle Workshop** — Scene browser, trigger gizmo, save/load/delete
- **Unit Tests** — 22 tests covering obstacle library, course data, and menu discovery
- **Course Editor** — Click-to-place, XZ drag, Q/E height, gate ordering, trigger gizmos, gate sequence lines, save/load
- **Drone Physics** — 12 drones with randomized PID/configs, thrust-through-body model in FixedUpdate, AI waypoint tracking
- **Drone Realism** — Motor lag, attitude underdamping, per-drone variation, dirty air, prop wash, battery sag, expanded dev dashboard
- **Race System** — Gate validation, timing, lifecycle (countdown, race clock, completion detection, crash/DNF)
- **Rendering Overhaul** — Cel-shaded materials, halftone gradients, hue shifting, procedural TRON night skybox, custom WGSL shaders
- **Results & Cameras** — Results screen, chase camera, FPV camera, camera mode switching, full gameplay loop
- **Course Props** — Firework emitter props, tabbed editor UI, confetti/shell burst effects at race finish
- **Code Health** — File splitting, async poll replacement, UI unit tests, spline rebuild optimization
- **Drone Models** — Blender-exported visual models replacing placeholders

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full module structure, data flow, and design decisions.

## License

Private project — all rights reserved.
