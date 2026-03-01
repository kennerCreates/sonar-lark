# Sonar Lark

A drone racing simulator with a built-in map editor, built in Rust using [Bevy 0.18](https://bevyengine.org/).

Define obstacle types by importing 3D models from Blender, build race courses by placing obstacles, then watch 12 AI drones race through the course.

## Features

- **Map Editor** — Obstacle Workshop for defining obstacle types from `.glb` models, and a Course Editor for placing obstacles, props, and gates with drag/rotate/scale controls
- **12 AI Drones** — Thrust-through-body quadrotor physics with cascaded PID control, per-drone variation (cornering aggression, braking, attitude damping), dirty air/prop wash perturbations, and battery sag
- **Race System** — Gate validation with plane-crossing detection (tunneling-proof), per-drone timing, DNF on missed gates, countdown sequence, and a results screen with standings
- **Cel-Shaded Rendering** — Custom WGSL shaders with halftone dot transitions, hue-shifted highlights/shadows, and a procedural TRON-style night skybox
- **Obstacle Collision** — Swept segment vs OBB collision detection with gate opening exemption, editable collision volumes per obstacle type in the Workshop
- **Camera Modes** — Editor-placed course cameras (up to 9), chase camera (pack-follow), FPV (drone-mounted with target cycling), and Spectator (RTS orbit), with PiP preview in the editor
- **Post-Race Wandering** — Drones transition to ambient wandering with deterministic Fibonacci-hashed waypoints after results, replacing static victory laps
- **Victory Effects** — Confetti fans and staggered shell bursts from course-placed firework emitter props
- **Procedural Pilots** — 24 generated pilots with unique gamertags (6 naming styles), 8 personality traits that modify flying behavior, skill profiles (level/speed/cornering/consistency), and persistent stats (races, wins, crashes, best times)
- **Procedural Portraits** — SVG-assembled pilot portraits with 7 visual layers (face, eyes, mouth, hair, shirt, accessory, drone accent), dynamic color replacement, and per-pilot deterministic generation from seeded RNG
- **Sound Design** — Ambient drone sound loops with cross-fading and volume scaling by active drone count, crash sounds, race start/end audio, and firework victory effects
- **Dev Menu** — F4 toggleable tuning dashboard with 14 parameters for AI behavior, aerodynamics, and proximity avoidance; portrait palette editor with primary/secondary color pickers, complementary color pairing, live preview, and make-unique button

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
- Pilot roster: `assets/pilots/roster.pilots.ron`
- Portrait palettes: `assets/pilots/portrait_palette.ron`
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
| Race | 1 | Primary course camera (Chase fallback if none) |
| Race | 2 | Chase camera |
| Race | 3–9, 0 | Course cameras 2–9 (if placed) |
| Race | Shift+F | FPV camera (cycles target on repeat) |
| Race | Shift+S | Spectator camera (RTS orbit) |
| Race | F4 | Toggle dev dashboard |
| Editor | 1 / 2 / 3 | Move / Rotate / Scale mode |
| Editor | Shift (move) | Y-axis movement instead of XZ plane |
| Editor | Shift (rotate) | Z-axis rotation instead of Y |
| Editor | Ctrl (rotate) | X-axis rotation |
| Editor | Shift (scale) | Per-axis scale instead of uniform |
| Editor | F | Flip gate direction |
| Editor | Q / E | Adjust height |

## Development Milestones

### Foundation
- **Skeleton** — State machine, common systems, main.rs wiring
- **Data Layer** — Obstacle and course data with RON serialization
- **Unit Tests** — 22 tests covering obstacle library, course data, and menu discovery

### Editor
- **Obstacle Workshop** — Scene browser, trigger gizmo, save/load/delete
- **Course Editor** — Click-to-place, drag, height adjust, gate ordering, save/load
- **Course Props** — Firework emitter props, tabbed editor UI, confetti/shell effects
- **Course Cameras** — Editor-placed cameras with frustum gizmos and PiP preview
- **Editor Gizmo Rework** — Entity-local axes, move/rotate/scale modes, modifier keys, snapping

### Flight & Racing
- **Drone Physics** — 12 drones, thrust-through-body model, cascaded PID, AI waypoint tracking
- **Drone Realism** — Motor lag, dirty air, prop wash, battery sag, per-drone variation
- **Drone Models** — Blender-exported visual models replacing placeholders
- **Obstacle Collision** — Swept OBB detection, gate opening exemption, crash effects
- **Race System** — Gate validation, timing, countdown, completion detection, crash/DNF
- **Post-Race Wandering** — Ambient wandering with Fibonacci-hashed waypoints after results

### Presentation
- **Rendering Overhaul** — Cel-shaded materials, halftone gradients, procedural TRON skybox
- **Results & Cameras** — Results screen, chase/FPV/spectator cameras, full gameplay loop
- **Sound Effects** — Ambient drone loops, crash/start/end audio, firework sounds
- **Visual Interpolation** — Lerp/slerp between physics ticks for smooth 60fps rendering
- **Victory Effects** — Confetti fans and staggered shell bursts from firework emitter props

### Pilots & Portraits
- **Procedural Pilots** — 8 personality traits, skill profiles, gamertag generation, persistent roster
- **Procedural Portraits** — SVG fragment assembly with 7 layers, color replacement, per-pilot caching
- **Dev Menu & Portrait Editor** — Dev mode, palette editor with color pickers and live preview

### Code Health
- **Code Health I** — File splitting, async poll replacement, UI unit tests, spline optimization
- **Code Health II** — UI theme consolidation, cross-module decoupling, course discovery unification

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full module structure, data flow, and design decisions.

## License

Private project — all rights reserved.
