# Race System

## Race Flow

`RacePhase`: `WaitingToStart → Countdown → Racing → Finished`.

```
OnEnter(Race) → spawn obstacles, drones, UI
  Drones wander near start (physics, collision suppressed)
  │
  ▼ Player presses START RACE
  Convergence window → 3-2-1 countdown
  generate_race_script() → RaceScript + RaceEventLog resources
  GO → DronePhase::Racing
  │
  ▼ Racing (choreography drives drones, fire_scripted_events fires events)
  Per-drone: finish → Wandering, crash → ballistic → explosion
  │
  ▼ All done → RacePhase::Finished → ResultsTransitionTimer(0.5s) → Results
```

Race logic chain in Update: `tick_countdown → generate_race_script_system → tick_race_clock → sync_spline_progress → check_race_finished`.

## Race Script Generation

`generate_race_script()` in `race/script.rs` predetermines the entire race outcome from pilot data, course geometry, and randomness. See [`PLAN.md`](../PLAN.md) Phase 1 for the full algorithm.

**Inputs:** per-drone splines + configs, pilot data (SkillProfile, PersonalityTrait), RaceSeed, AiTuningParams, gate positions/forwards.

**Outputs:** `RaceScript` resource containing:
- `DroneScript` per drone: `segment_pace[]`, `crash: Option<CrashScript>`, `acrobatic_gates[]`, `gate_pass_t[]`
- `ScriptedOvertake[]`: pre-computed position swaps for event timing

**Key steps:** estimate base segment times → assign finish order → assign DNFs (0-3, min 4 finishers) → assign acrobatics → compute per-segment pace profiles → drama pass (tighten close finishes, cluster mid-pack, record overtakes).

## Scripted Events

Gate detection, collision, and race events are **not** live physics checks — they fire from pre-computed `spline_t` thresholds in `fire_scripted_events` (FixedUpdate, in choreography chain).

- **Gate pass**: when `spline_t` crosses `gate_pass_t[gate_index]` → `progress.record_gate_pass()`, advance `target_gate_index`
- **Overtake**: when overtaker reaches `gate_pass_t[overtake.gate_index]` → log to `RaceEventLog`
- **Crash**: when `spline_t` reaches crash point → insert `BallisticState`, call `crash_drone()`
- **Finish**: when `spline_t >= total_length + FINISH_EXTENSION` → `reset_physics_for_wandering()`, `DronePhase::Wandering`

`RaceEventLog` accumulates timestamped events (gate passes, overtakes, acrobatics, crashes, finishes) for future highlight reel.

## Gate Infrastructure

`GatePlanes` and `ObstacleCollisionCache` are still built at race start (used by script generator for gate positions/forwards). `GateForward` component stores world-space forward per gate. Editor shows cyan arrow gizmo; `F` key flips direction.

## Obstacle Collision

`CollisionVolumeConfig` in RON, `ObstacleCollisionVolume` component. `ObstacleCollisionCache` built at race start. `crash_drone()` shared helper (sets phase, zeros velocity, hides entity, spawns explosion, plays sound, updates RaceProgress). `DnfReason` variants: `MissedGate(u32)`, `ObstacleCollision`, `DroneCollision`. Pure geometry functions in `collision_math.rs`.

## Race Leaderboard

`LeaderboardRoot` panel (top-left, `DespawnOnExit`). 12 rows with `LbColorBar`/`LbNameText`/`LbTimeText`. Updated from `RaceProgress::standings()` each frame. Names/colors sourced from `SelectedPilots` resource. Race UI split into: `start_button.rs`, `overlays.rs`, `leaderboard.rs`, `camera_hud.rs`.

## Results Pattern

`RaceResults` snapshot built from `RaceProgress::to_race_results()` before `Race → Results`. `ResultsTransitionTimer(0.5s)` delays transition. UI with `DespawnOnExit(AppState::Results)`. `SelectedCourse` persists for "RACE AGAIN".

## Course Obstacle Cleanup

`spawn_obstacle()` adds `DespawnOnExit(AppState::Race)`. `CourseSpawned` marker removed `OnExit(Race)`.
