# Drone System

## Asset Loading

`DroneGltfHandle` loaded `OnEnter(Race)`, `DroneAssets` extracted via `run_if(drone_gltf_ready)` + `run_if(not(resource_exists::<DroneAssets>))`, `spawn_drones` gated by `run_if(resource_exists::<DroneAssets>)` + `run_if(resource_exists::<CourseData>)`. 12 drones with `DespawnOnExit(AppState::Race)`. Resources cleaned up `OnExit(Race)`.

## Asset Readiness Pattern

`drone_gltf_ready()` and `obstacles_gltf_ready()` are run-condition functions using `AssetServer::is_loaded_with_dependencies()`. Used to gate systems that depend on loaded glTF assets, replacing per-frame polling with `run_if` conditions.

## Drone Lifecycle

`DronePhase` tracks state: `Idle → Racing → Returning → Idle` (or `Racing → Crashed` on DNF). On Results entry: `VictoryLap → Wandering` (deterministic waypoints via Fibonacci hashing, `WanderState` component, `WanderBounds` resource from course extents). Crashed drones: hidden, zero velocity, skipped by physics/AI. `ReturnPath` component holds return spline. `drones_are_active()` run condition keeps AI alive during Racing + Returning + Wandering.

## Physics Pipeline

11-system `.chain()` in FixedUpdate (thrust-through-body model). Each drone gets a unique spline via `generate_drone_race_path()`. Per-drone variation via `DroneConfig` (generated from pilot skill+personality when `PilotConfigs` is available, else random). `RaceSeed` resource randomizes outcomes between races.

```
Blender ──► drone.glb ──► DroneGltfHandle (OnEnter(Race) load)
                                │
                          DroneAssets (run_if drone_gltf_ready)
                                │
CourseData ──► generate_race_path() ──► base Catmull-Rom CubicCurve (editor preview)
        (paths.rs)  │
                    └──► generate_drone_race_path() ──► per-drone unique CubicCurve (12x)
                         (midleg lateral shift + gate 2D offset + approach scaling)
                                                                   │
                                                        spawn_drones() ──► 12 Drone entities
                                                                   │
                                                        FixedFirst: restore_physics_transforms (undo visual interp)
                                                        FixedPreUpdate: save_previous_transforms
                                                        FixedUpdate chain (11-system, thrust-through-body):
                                                        update_ai_targets → compute_racing_line
                                                        → proximity_avoidance → update_wander_targets
                                                        → hover_target → position_pid
                                                        → attitude_controller → dirty_air_perturbation → motor_lag
                                                        → apply_forces → integrate_motion → clamp_transform
                                                        FixedPostUpdate: save_physics_transforms
                                                        PostUpdate: interpolate_visual_transforms

                                                        Post-race: Racing → Returning (per-drone)
                                                        → generate_return_path() → non-cyclic spline
                                                        → smoothstep deceleration → return to start
                                                        → Returning → Idle (hover)

                                                        Results state: VictoryLap → Wandering
                                                        → deterministic waypoint generation (Fibonacci hash)
                                                        → dwell at waypoint → pick next → continuous flight
```

The physics model uses a **thrust-through-body** architecture: the drone's orientation determines its thrust direction (always body-up). A cascaded controller (outer position PID → inner attitude PD) drives orientation, and motor lag filters thrust changes. Quadratic drag and angular dynamics with moment of inertia produce realistic banking, braking, and hover behavior. Aerodynamic perturbations (dirty air from leading drones, prop wash on descent) add angular wobble that the PD must fight, producing visible instability in dirty air. Battery sag linearly reduces max thrust over the race duration.

## Visual Transform Interpolation

`PhysicsTranslation`/`PhysicsRotation` store authoritative post-tick state; `restore_physics_transforms` (FixedFirst) undoes visual interpolation before physics, `save_physics_transforms` (FixedPostUpdate) captures result, `interpolate_visual_transforms` (PostUpdate) blends `Previous*` → `Physics*` using `overstep_fraction` for smooth rendering.

## AI Path Following

Each drone follows a unique cyclic Catmull-Rom spline (3 control points per gate). Per-drone variation: gate pass offset, approach scaling, midleg lateral bias (all deterministic via Fibonacci hashing + `RaceSeed`). `POINTS_PER_GATE = 3.0`. Full lap: all gates + gate 0 again. Curvature-aware speed limiting + gate correction blending + adaptive look-ahead. Requires >= 2 gates.

## Aerodynamic Perturbations

Dirty air (wake cone wobble behind leading drones) + prop wash (descent wobble). Deterministic sine waves, no RNG. Battery sag reduces thrust linearly over race. All tunable via dev dashboard.

## Proximity Avoidance

Lateral dodge when drones are nearby. O(12²) per tick. Gate-proximity suppression prevents dodging near gate openings. Tunable via dev dashboard.

## Explosion Effects

Three particle layers (debris, hot smoke, dark smoke) using pre-allocated `ExplosionMeshes`. 4 random explosion sounds. Loaded `OnEnter(Race)`, cleaned up `OnExit(Race)`. All `StandardMaterial` (unlit emissive).

## Firework Effects

Confetti fan + staggered shell bursts on first finish. `FireworkEmitter` entities from course props (or auto at gate 0). Pre-allocated `FireworkMeshes`. `FireworksTriggered` prevents re-fire. Particles use `DespawnOnExit(AppState::Results)`.

## Dev Dashboard

`AiTuningParams` resource (14 tunable params, persists across restarts). F4 toggles dashboard UI in Race. `PARAM_META` defines display names, step sizes, ranges. Index-based get/set for UI.

## Flight Spline Preview

In CourseEditor, `draw_flight_spline_preview` generates the race spline and draws it as gizmo lines colored green (fast) → red (slow curvature). Uses `cyclic_curvature()`/`safe_speed_for_curvature()` from `drone::ai`. Requires >= 2 gates.
