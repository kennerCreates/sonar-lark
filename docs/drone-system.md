# Drone System

## Asset Loading

`DroneGltfHandle` loaded `OnEnter(Race)`, `DroneAssets` extracted via `run_if(drone_gltf_ready)` + `run_if(not(resource_exists::<DroneAssets>))`, `spawn_drones` gated by `run_if(resource_exists::<DroneAssets>)` + `run_if(resource_exists::<CourseData>)`. 12 drones with `DespawnOnExit(AppState::Race)`. Resources cleaned up `OnExit(Results)`.

## Asset Readiness Pattern

`drone_gltf_ready()` and `obstacles_gltf_ready()` are run-condition functions using `AssetServer::is_loaded_with_dependencies()`. Used to gate systems that depend on loaded glTF assets, replacing per-frame polling with `run_if` conditions.

## Drone Lifecycle

`DronePhase` (defined in `common::race_participant`, re-exported from `drone::components`) tracks state:

```
OnEnter(Race) → spawn_drones → DronePhase::Idle
  Idle → Wandering (pre-race wandering near start area, physics-based)
  START RACE → convergence window → countdown → GO
  GO → DronePhase::Racing (choreography takes over)
  Racing → finish → DronePhase::Wandering (immediate, per-entity)
  Racing → crash → DronePhase::Crashed (ballistic arc → explosion)
  OnEnter(Results) → transition_to_wandering (any remaining non-wandering drones)
```

**No VictoryLap phase** — finished drones return to wandering immediately. Crashed drones: hidden after ground impact, zero velocity, skipped by all systems.

## Dual-Chain Architecture

Two system chains run in FixedUpdate during Race/Results states, separated by per-entity `DronePhase` filtering:

```
FixedUpdate:
  Choreography chain (DronePhase::Racing only):
    advance_choreography → compute_choreographed_rotation
    → apply_visual_noise → fire_scripted_events

  Physics chain (skips DronePhase::Racing):
    AI: update_ai_targets → compute_racing_line → proximity_avoidance → update_wander_targets
    Physics: sync_tilt_clamp → position_to_acceleration → acceleration_to_attitude
    → attitude_controller → dirty_air_perturbation → motor_lag
    → apply_forces → integrate_motion → clamp_transform
```

The choreography chain reads from `RaceScript` (per-drone pacing, crashes, acrobatics) and writes `Transform`, `DroneDynamics.velocity`, `AIController.spline_t`. The physics chain handles pre-race wandering, post-finish wandering, and Results state.

## Choreography (Racing Drones)

See [`PLAN.md`](../PLAN.md) for the full choreographed spline racing design.

**`advance_choreography`**: Advances `spline_t` at `base_speed * segment_pace[current_segment]` where base speed comes from `safe_speed_for_curvature()`. Writes `Transform.translation` from spline position (with acrobatic offsets), `DroneDynamics.velocity` from tangent, `AIController.spline_t`. Also handles ballistic arcs for crashed drones.

**`compute_choreographed_rotation`**: Derives rotation from spline curvature — bank angle from `atan(speed² × κ / g)` with exponential smoothing. Acrobatic rotation keyframes (Split-S, Power Loop) at marked gates. Smoothstep blend at entry/exit.

**`apply_visual_noise`**: Attitude jitter (layered sine waves), dirty air wobble (proximity-based amplitude boost), position micro-drift (~1cm).

**`fire_scripted_events`**: Checks `spline_t` against pre-computed `gate_pass_t` thresholds. Fires gate passes, overtakes, crashes, and finishes. Updates `RaceProgress`, `RaceEventLog`, `AIController.target_gate_index`. Calls `crash_drone()` for scripted crashes.

### Convergence & Start

1. Player presses START RACE → `begin_convergence` sets `DesiredPosition` to spline start positions
2. Drones fly there naturally via physics chain (~5s window or until all within 2m)
3. 3-2-1 countdown (3s)
4. GO → `snap_to_start_positions` places drones exactly on spline start → `DronePhase::Racing`

### Components

- `ChoreographyState`: `previous_spline_t` (for crossing detection), `consistency` (pilot skill, cached), `smoothed_bank` (exponential smoothing)
- `BallisticState`: `velocity` (inserted on crash, removed on ground impact)

## Physics Pipeline (Wandering Drones)

The physics model uses **thrust-through-body** architecture with a 3-stage decomposed PID. See [`drone-physics.md`](drone-physics.md) for parameters and tuning.

```
DesiredPosition (from AI or hover_target)
       |
  position_to_acceleration   Stage 1: position error → desired acceleration (with gravity compensation, anti-windup)
       |
  DesiredAcceleration
       |
  acceleration_to_attitude   Stage 2: acceleration → body orientation + thrust (reads TiltClamp)
       |
  DesiredAttitude
       |
  attitude_controller        Stage 3: orientation error → torque → angular velocity → rotation
       |
  motor_lag → apply_forces → integrate_motion → clamp_transform
```

Per-drone variation via `DroneConfig` (generated from pilot skill+personality when `PilotConfigs` is available, else random). `RaceSeed` resource randomizes outcomes between races.

## Visual Transform Interpolation

`PhysicsTranslation`/`PhysicsRotation` store authoritative post-tick state; `restore_physics_transforms` (FixedFirst) undoes visual interpolation before physics/choreography, `save_physics_transforms` (FixedPostUpdate) captures result, `interpolate_visual_transforms` (PostUpdate) blends `Previous*` → `Physics*` using `overstep_fraction`. Works identically for both choreography and physics — both write `Transform` at the same FixedUpdate insertion point.

## AI Path Following

Each drone follows a unique cyclic Catmull-Rom spline (3 control points per gate). Per-drone variation: gate pass offset, approach scaling, midleg lateral bias (all deterministic via Fibonacci hashing + `RaceSeed`). `POINTS_PER_GATE = 3.0`. Full lap: all gates + gate 0 again. Curvature-aware speed limiting + gate correction blending + adaptive look-ahead. Requires >= 2 gates. Wandering logic in `wander.rs` (`WanderBounds`, `wander_waypoint()`, `update_wander_targets()`, `transition_to_wandering()`, `build_wander_bounds()`).

## Explosion Effects

Three particle layers (debris, hot smoke, dark smoke) using pre-allocated `ExplosionMeshes`. 4 random explosion sounds. Loaded `OnEnter(Race)`, cleaned up `OnExit(Results)`. All `StandardMaterial` (unlit emissive).

## Firework Effects

Confetti fan + staggered shell bursts on first finish. `FireworkEmitter` entities from course props (or auto at gate 0). Pre-allocated `FireworkMeshes`. `FireworksTriggered` prevents re-fire. Particles use `DespawnOnExit(AppState::Results)`.

## Dev Dashboard

`AiTuningParams` resource (14 tunable params, persists across restarts). F4 toggles dashboard UI in Race. `PARAM_META` defines display names, step sizes, ranges. Index-based get/set for UI.

## Debug Draw

F3 toggles visualization: spline paths (color-coded by speed), gate markers, gate planes, drone state indicators, progress indicators.

## Flight Spline Preview

In CourseEditor, `draw_flight_spline_preview` generates the race spline and draws it as gizmo lines colored green (fast) → red (slow curvature). Uses `cyclic_curvature()`/`safe_speed_for_curvature()` from `drone::ai`. Requires >= 2 gates.
