# Claude Instructions

## Project Context

This is a **drone racing simulator with a built-in map editor**, built in **Rust** using **Bevy 0.18.0**. See `ARCHITECTURE.md` for the full module structure, data flow, and design decisions.

Key facts:
- **State machine**: `AppState` (Menu → Editor → Race → Results) with `EditorMode` SubStates (ObstacleWorkshop, CourseEditor)
- **Physics**: PID-lite quadrotor in `FixedUpdate`, 12 AI drones with per-drone variation
- **Data**: Obstacle definitions and courses serialized as RON. Single `.glb` file for all 3D models.
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
- **Serialization**: all persistent data uses RON via serde. Obstacle library lives at `assets/library/default.obstacles.ron`, courses at `assets/courses/*.course.ron`.
- **Asset loading**: all obstacle models come from a single `assets/models/obstacles.glb`. Individual objects are accessed via `Gltf::named_nodes` → `GltfNode` → `GltfMesh` → primitives, using the Blender object name. Each obstacle spawns a parent entity with child `Mesh3d`/`MeshMaterial3d` per primitive.
- **Physics**: all physics systems run in `FixedUpdate` with `.chain()` for ordering. No physics in `Update`.
- **Bevy 0.18 specifics**: `set_parent_in_place()` (not `set_parent()`), `SceneRoot` (not `SceneBundle`), `Mesh3d`/`MeshMaterial3d` components, `ChildSpawnerCommands` (not `ChildBuilder`) for `with_children` closures in commands context, `AccumulatedMouseMotion`/`AccumulatedMouseScroll` from `bevy::input::mouse` (not in prelude). `MessageReader<T>` (not `EventReader<T>`) for reading events. `KeyboardInput` from `bevy::input::keyboard`. System tuples max ~12 elements for `run_if`; split larger groups into multiple `add_systems` calls. `Gltf::named_nodes`/`named_scenes` use `Box<str>` keys (not `String`).
- **Menu pattern**: `AvailableCourses` resource is created on `OnEnter(Menu)` and removed on `OnExit(Menu)`. `SelectedCourse` resource persists across states to carry the user's course selection into Race.
- **Workshop pattern**: `WorkshopState` resource is created on `OnEnter(ObstacleWorkshop)` and removed on `OnExit`. Preview entities use `PreviewObstacle` component and are manually despawned on exit (not part of UI hierarchy). glTF node list populates asynchronously once the asset is loaded. If no glb scene matches, a placeholder cube is spawned.

## Post-Phase Checklist

After completing each implementation phase:

1. **Update documentation**: Review and update `TODO.md`, `ARCHITECTURE.md`, and `CLAUDE.md` with any new types, patterns, or conventions introduced.
2. **Review warnings**: Run `cargo build` and review all warnings. Fix any that indicate real issues (unused imports, unnecessary mut, etc.). Warnings for types/functions that are planned for upcoming phases in the current sprint are acceptable and should be left alone — do not suppress them with `#[allow(dead_code)]`.
3. **Run tests**: Run `cargo test` and verify all tests pass. Add tests for new pure-logic functions.

## Testing Conventions

- Tests live in `#[cfg(test)] mod tests` at the bottom of each source file (idiomatic Rust).
- Use `tempfile` crate (dev-dependency) for filesystem tests — never write to the real `assets/` directory.
- Test pure logic and serialization (file I/O, data structures, discovery). ECS systems are tested manually.
- When adding file I/O functions, provide a parameterized version that accepts a `&Path` so tests can use temp directories (e.g., `discover_courses_in(path)` vs `discover_courses()`).
