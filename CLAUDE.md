# Claude Instructions

## Project Context

This is a **drone racing simulator with a built-in map editor**, built in **Rust** using **Bevy 0.18.0**. See `ARCHITECTURE.md` for the full module structure, data flow, and design decisions.

Key facts:
- **State machine**: `AppState` (Menu → Editor → Race → Results) with `EditorMode` SubStates (ObstacleWorkshop, CourseEditor)
- **Physics**: Thrust-through-body quadrotor with cascaded PID (position → attitude) in `FixedUpdate`, 12 AI drones with per-drone variation. Quadratic drag, angular dynamics with moment of inertia, motor lag.
- **Data**: Obstacle definitions and courses serialized as RON. Single `.glb` file for all obstacle 3D models, separate `.glb` for drone model.
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
- **Drone pattern**: `DroneGltfHandle` is loaded `OnEnter(Race)`. `DroneAssets` is extracted from the glTF in an `Update` polling system (async pattern). `spawn_drones` polls until both `DroneAssets` and `CourseData` are available, then spawns 12 drones with `DespawnOnExit(AppState::Race)`. Resources are cleaned up `OnExit(Race)`. Physics runs in `FixedUpdate` as a 9-system `.chain()` using a thrust-through-body model: `hover_target → position_pid → attitude_controller → motor_lag → apply_forces → integrate_motion → clamp_transform` (plus 2 AI systems when drones are active). Key components: `PositionPid` (outer loop), `AttitudePd` (inner loop), `DesiredAttitude` (bridge between loops), `DroneDynamics` (velocity, angular velocity, thrust with motor lag, quadratic drag). If the drone glb is missing, a red placeholder cube is used.
- **Drone lifecycle**: `DronePhase` component tracks per-drone state: `Idle` (hovering at start), `Racing` (following race spline), `Returning` (flying back to start). Transitions: `Idle → Racing` on race start button, `Racing → Returning` when individual drone's `spline_t >= cycle_t + FINISH_EXTENSION` (past gate 0 on the second pass, completing a full lap), `Returning → Idle` when drone reaches end of return spline. `ReturnPath` component (inserted on `Racing → Returning`, removed on `Returning → Idle`) holds a non-cyclic Catmull-Rom spline from the finish area back to start with per-drone variation. `drones_are_active()` run condition (replaces `race_is_running` for AI scheduling) returns true during `Racing` or when any drone is `Returning`, keeping AI systems alive during the post-race return flight. `hover_target` only writes to `Idle` drones.
- **Course delete pattern**: `PendingCourseDelete` resource tracks a pending deletion. Clicking the "X" button on a course entry shows an inline confirmation (Yes/Cancel) inside the `ExistingCoursesContainer`. Confirming calls `delete_course()` and rebuilds the list. If the deleted course matches the currently loaded course name, the editor state is reset. If it matches `LastEditedCourse`, that resource is removed.
- **AI path following**: Drones follow a **cyclic Catmull-Rom spline** (`CubicCurve<Vec3>` from Bevy's `CubicCardinalSpline::new_catmull_rom().to_curve_cyclic()`) with 2 control points per gate (approach, departure). Approach/departure waypoints are offset from the gate center along the gate forward direction (`APPROACH_OFFSET = 8.0`), ensuring drones fly through each gate from the correct side. `POINTS_PER_GATE = 2.0` scales the spline parameter space: gate i midpoint is at `spline_t = i * 2 + 0.5`. `AIController` stores the spline and a continuous `spline_t` parameter. The race is a **full lap**: drones start behind gate 0, fly through all gates, then back through gate 0 again (the finish line). `spline_t` extends past one full cycle (`cycle_t + FINISH_EXTENSION = gate_count * 2 + 1.5`) using `rem_euclid(cycle_t)` wrapping for spline sampling. Progress is updated via local tangent projection in `update_ai_targets()`, with a **minimum speed-based advancement** (`MIN_ADVANCE_SPEED_FRACTION = 0.25`) that prevents deadlocks when drones overshoot sharp turns and the tangent projection drops to zero. `compute_racing_line()` samples the spline at an **adaptive look-ahead** that shrinks on high-curvature sections (`CURVATURE_LOOK_AHEAD_SCALE = 30.0`, clamped to `[0.33, 1.0]` of base `LOOK_AHEAD_T`). **Curvature-aware speed limiting**: `DesiredPosition.max_speed` is set per-drone per-frame by scanning spline curvature over the next `SPEED_CURVATURE_RANGE = 2.0` parameter units (5 samples), computing `v_safe = sqrt(SAFE_LATERAL_ACCEL / κ)` clamped to `[8, 45]` m/s. `apply_forces` respects this per-drone limit instead of using only the global `MAX_SPEED`. `RacePath` struct (in `spawning.rs`) bundles the spline with gate positions and gate forwards. Requires >= 2 gates.
- **Dev dashboard pattern**: `AiTuningParams` resource (inserted via `init_resource`, persists across race restarts) holds 9 runtime-tunable AI/physics constants. All AI and physics systems read from this resource instead of `const` values. The dev dashboard UI (`drone/dev_dashboard.rs`) toggles with **F4** during Race state. Panel spawns top-right with per-parameter +/- buttons, value display (green when modified from default), and a Reset All button. Panel entity uses `DespawnOnExit(AppState::Race)`. Systems: `toggle_dev_dashboard`, `handle_param_buttons`, `handle_reset_button`, `update_param_labels`, `update_button_colors` (all in `Update`). `PARAM_META` const array stores display names, step sizes, and min/max ranges. `AiTuningParams::get(i)` / `set(i, value)` provide index-based access for the UI.
- **Gate directionality**: Each gate has a canonical forward direction. `TriggerVolumeConfig.forward` defines the local-space forward (default `Vec3::NEG_Z`). Per-instance `ObstacleInstance.gate_forward_flipped: bool` flips it 180 degrees. World-space forward = `rotation * (if flipped { -forward } else { forward })`. `GateForward` component (on gate entities at race time) stores the computed world-space forward. `RacePath.gate_forwards` and `AIController.gate_forwards` carry per-gate forward vectors for AI and validation. Gate forwards determine approach/departure waypoint placement in spline generation, so flipping a gate's direction changes the drone approach side. In the course editor, gates with `gate_order` show a **cyan arrow gizmo** pointing in the forward direction. Press `F` with a gate selected to flip its direction.
- **Flight spline preview**: In CourseEditor mode, `draw_flight_spline_preview` builds a temporary `CourseData` from placed obstacles and calls `generate_race_path()` to produce the same cyclic Catmull-Rom spline used at race time. The spline is sampled every `0.1` parameter units and drawn as colored gizmo line segments. Color is based on curvature: **green** (drones can fly near max speed) → **red** (curvature forces drones to minimum speed). Uses `cyclic_curvature()` and `safe_speed_for_curvature()` from `drone::ai` (made pub for this purpose) with the live `AiTuningParams` resource. Requires >= 2 gates; draws nothing otherwise.

## Post-Phase Checklist

After completing each implementation phase:

1. **Update documentation**: Review and update `TODO.md`, `ARCHITECTURE.md`, and `CLAUDE.md` with any new types, patterns, or conventions introduced.
2. **Review warnings**: Run `cargo build` and review all warnings. Fix any that indicate real issues (unused imports, unnecessary mut, etc.). Warnings for types/functions that are planned for upcoming phases in the current sprint are acceptable and should be left alone — do not suppress them with `#[allow(dead_code)]`.
3. **Run tests**: Run `cargo test` and verify all tests pass. Add tests for new pure-logic functions.
4. **Manual testing feedback form**: After all automated checks pass, present a structured feedback form for manual testing. The form must include:
   - A checklist of every manually-testable behavior introduced or changed in the phase (specific actions, expected results).
   - Edge cases and error scenarios to verify (e.g., invalid input, rapid state transitions, boundary values).
   - Performance observations to watch for (frame drops, hitches, visual artifacts).
   - A "Pass / Fail / Notes" column for each item so the user can report results inline.
   - Regression checks: key existing behaviors that should still work unchanged.

   Format the form as a markdown table or checklist that can be filled out directly in chat.

## Testing Conventions

- Tests live in `#[cfg(test)] mod tests` at the bottom of each source file (idiomatic Rust).
- Use `tempfile` crate (dev-dependency) for filesystem tests — never write to the real `assets/` directory.
- Test pure logic and serialization (file I/O, data structures, discovery). ECS systems are tested manually.
- When adding file I/O functions, provide a parameterized version that accepts a `&Path` so tests can use temp directories (e.g., `discover_courses_in(path)` vs `discover_courses()`).
