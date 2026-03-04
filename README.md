# Sonar Lark

A drone racing league organizer with a built-in map editor, built in Rust using [Bevy 0.18](https://bevyengine.org/).

Define obstacle types by importing 3D models from Blender, build race courses by placing obstacles, then watch 12 AI drones race through choreographed spline-following with scripted outcomes, acrobatic maneuvers, and dramatic overtakes.

## Features

- **Map Editor** — Obstacle Workshop for defining obstacle types from `.glb` models, and a Course Editor for placing obstacles, props, and gates with drag/rotate/scale controls
- **Choreographed Racing** — Race outcomes predetermined by a script generator (pilot skill + course geometry + randomness), played back via spline-following with curvature-based banking, per-segment pacing that produces natural overtakes, and a drama pass for photo finishes
- **12 AI Drones** — Per-drone unique racing lines, physics-based wandering (thrust-through-body quadrotor with 3-stage PID), choreographed spline playback during races
- **Acrobatic Maneuvers** — Split-S dips and power loops at tight turns for skilled pilots, with rotation keyframes, altitude offsets, and smooth blend zones
- **Race System** — Scripted gate passes from spline progress thresholds, 0-3 crashes per race (obstacle + drone-on-drone collisions), per-drone timing, countdown with convergence sequence, results screen with standings
- **Cel-Shaded Rendering** — Custom WGSL shaders with halftone dot transitions, hue-shifted highlights/shadows, and a procedural TRON-style night skybox
- **Obstacle Collision** — Editable collision volumes per obstacle type in the Workshop, scripted crash events with ballistic arcs and explosions
- **Camera Modes** — Editor-placed course cameras (up to 9), chase camera (pack-follow), FPV (drone-mounted with target cycling), and Spectator (RTS orbit), with PiP preview in the editor
- **Natural Drone Behavior** — Drones wander freely pre-race (warm-up feel), converge to start positions on countdown, return to ambient wandering immediately after finishing (no victory laps)
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
- **Drone Physics** — 12 drones, thrust-through-body model, 3-stage cascaded PID, AI waypoint tracking
- **Drone Realism** — Motor lag, dirty air, prop wash, battery sag, per-drone variation
- **Drone Models** — Blender-exported visual models replacing placeholders
- **Obstacle Collision** — Swept OBB detection, gate opening exemption, crash effects
- **Race System** — Gate validation, timing, countdown, completion detection, crash/DNF
- **Post-Race Wandering** — Ambient wandering with Fibonacci-hashed waypoints after results
- **Choreographed Racing** — Physics-based AI replaced by scripted spline playback during races: race script generator, per-segment pacing, curvature-based banking, acrobatic maneuvers (Split-S, power loops), scripted crashes (obstacle + drone-on-drone), visual noise (attitude jitter, dirty air wobble, micro-drift), drama pass for close finishes and mid-pack clustering, pre-race convergence sequence

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
