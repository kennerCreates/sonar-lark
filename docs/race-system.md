# Race System

## Race Validation

`RacePhase`: `WaitingToStart → Countdown → Racing → Finished`. Race logic: 7-system `.chain()` in Update. Gate detection uses plane-crossing (line segment vs gate plane, directional, tunneling-proof). `obstacle_collision_check` tests swept segments vs OBBs (gate openings exempted). `miss_detection` is a safety net for drones that skip gates. `GatePlanes` + `ObstacleCollisionCache` resources cached at race start. Gate 0 is both start and finish (full lap).

## Gate Detection

`TriggerVolumeConfig.forward` (default `NEG_Z`), per-instance `gate_forward_flipped` flag. `GateForward` component stores world-space forward. Editor shows cyan arrow gizmo; `F` key flips direction.

## Obstacle Collision

`CollisionVolumeConfig` in RON (`#[serde(default)]`), `ObstacleCollisionVolume` component on parent entity. `ObstacleCollisionCache` built at race start (world-space OBBs). Swept segment vs OBB slab test with drone radius expansion (`DRONE_COLLISION_RADIUS = 0.35`). Gate openings exempted via `point_in_gate_opening` (infinite-depth tube). `crash_drone()` shared helper (used by both `miss_detection` and `obstacle_collision_check`). `DnfReason::ObstacleCollision` variant. Pure geometry functions (`segment_obb_intersection`, `point_in_gate_opening`) extracted to `collision_math.rs`.

## Race Leaderboard

`LeaderboardRoot` panel (top-left, `DespawnOnExit`). 12 rows with `LbColorBar`/`LbNameText`/`LbTimeText`. Updated from `RaceProgress::standings()` each frame. Names/colors sourced from `SelectedPilots` resource (falls back to `DRONE_NAMES`/`DRONE_COLORS` in `drone/spawning.rs` if no pilots). Race UI split into: `start_button.rs`, `overlays.rs` (clock, no-gates banner, open-editor button), `leaderboard.rs`, `camera_hud.rs`.

## Results Pattern

`RaceResults` snapshot built from `RaceProgress::to_race_results()` before `Race → Results`. `ResultsTransitionTimer(0.5s)` delays transition. UI with `DespawnOnExit(AppState::Results)`. `SelectedCourse` persists for "RACE AGAIN".

## Course Obstacle Cleanup

`spawn_obstacle()` adds `DespawnOnExit(AppState::Race)`. `CourseSpawned` marker removed `OnExit(Race)`.
