# Claude Instructions

## Project Context

This is a **drone racing simulator with a built-in map editor**, built in **Rust** using **Bevy 0.18.0**. See `ARCHITECTURE.md` for the module structure, data flow, and design decisions.

Key facts:
- **State machine**: `AppState` (Menu → Editor → Race → Results) with `EditorMode` SubStates (ObstacleWorkshop, CourseEditor)
- **Choreographed racing**: Race outcomes predetermined by script generator (`race/script.rs`), played back via spline-following (`drone/choreography.rs`). Physics only used for wandering drones. See [`docs/drone-system.md`](docs/drone-system.md), [`docs/race-system.md`](docs/race-system.md).
- **Physics**: Thrust-through-body quadrotor with 3-stage PID (position→acceleration→attitude), used for pre/post-race wandering. See [`docs/drone-physics.md`](docs/drone-physics.md).
- **Data**: Obstacle definitions and courses serialized as RON. Single `.glb` for obstacle models, separate `.glb` for drone.

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

## Core Conventions

- **Plugins**: each module exposes a single plugin (struct or fn). Plugins register their own systems with appropriate state guards via `run_if(in_state(...))`.
- **State scoping**: use `DespawnOnExit` on entities that should be cleaned up when leaving a state. Do not manually despawn in `OnExit` unless there's a reason.
- **Serialization**: all persistent data uses RON via serde (`assets/library/`, `assets/courses/`, `assets/pilots/`).
- **Asset loading**: obstacle models from single `.glb`, accessed by Blender name. See [`docs/editor-system.md`](docs/editor-system.md).
- **Dual FixedUpdate chains**: Choreography chain (Racing drones) and physics chain (Wandering/Idle drones) both in FixedUpdate with `.chain()`. Per-entity `DronePhase` guards determine which chain processes each drone. No physics in `Update`. See [`docs/drone-system.md`](docs/drone-system.md).
- **Rendering**: all visible geometry uses `CelMaterial`. Use `cel_material_from_color(base_color, light_dir)`. See [`docs/rendering.md`](docs/rendering.md).
- **Asset readiness**: `drone_gltf_ready()` / `obstacles_gltf_ready()` run-conditions gate glTF-dependent systems. See [`docs/drone-system.md`](docs/drone-system.md).
- **Bevy 0.18**: See `docs/bevy-018.md` for Bevy 0.18 API specifics — consult before writing any Bevy code.

## Detailed Docs

**Only read a doc when the current task directly involves that subsystem.** Do not read docs speculatively or "just in case". Use grep on `docs/types-reference.md` for type lookups rather than reading it in full.

| Document | ONLY read when... |
|----------|-------------------|
| [`docs/bevy-018.md`](docs/bevy-018.md) | Writing or modifying Bevy systems, queries, or ECS code |
| [`docs/drone-system.md`](docs/drone-system.md) | Changing drone spawning, choreography, physics systems, AI, paths, explosions, or fireworks |
| [`docs/drone-physics.md`](docs/drone-physics.md) | Tuning flight parameters or modifying physics code (quick ref; see `drone-physics-deep-dive.md` for real-world reference) |
| [`docs/race-system.md`](docs/race-system.md) | Changing race flow, script generation, scripted events, leaderboard, or results |
| [`docs/editor-system.md`](docs/editor-system.md) | Changing workshop, course editor, asset loading, props, or cameras |
| [`docs/pilot-system.md`](docs/pilot-system.md) | Changing pilot roster, portraits, or dev menu palette editor |
| [`docs/camera-system.md`](docs/camera-system.md) | Changing camera modes or switching logic |
| [`docs/rendering.md`](docs/rendering.md) | Changing CelMaterial, skybox, shaders, or light direction |
| [`docs/types-reference.md`](docs/types-reference.md) | Looking up a specific type — **grep, don't read in full** |
| [`docs/post-phase-checklist.md`](docs/post-phase-checklist.md) | Completing an implementation phase |
| [`docs/testing-conventions.md`](docs/testing-conventions.md) | Writing or modifying tests |

## Post-Phase Checklist

Follow [`docs/post-phase-checklist.md`](docs/post-phase-checklist.md) after completing each implementation phase (build, clippy, tests, manual testing form).

## Testing Conventions

Follow [`docs/testing-conventions.md`](docs/testing-conventions.md) when writing tests.
