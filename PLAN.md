# Approach B: Maneuver Override System — Implementation Plan

## Context

AI drones currently use "car-like" banking for all turns: slow down based on curvature, bank at up to 83 degrees, fly a wide arc. Real racing drones use attitude-based maneuvers (Split-S, Power Loop) for tight turns — flipping the entire aircraft to reorient the thrust vector in ~160ms rather than flying a 14+ meter arc. This plan adds a **Maneuver Override System** that temporarily takes over from the position PID during acrobatic turns, producing visually correct and physically motivated tight-turn behavior.

## Banking Compatibility Assessment

**The inner attitude loop and all downstream physics are fully compatible.** The incompatibility is isolated to the outer `position_pid` system:

| System | Compatible? | Why |
|--------|-------------|-----|
| `attitude_controller` | Yes | Drives toward any target `Quat`, handles 180-degree rotations via short-path |
| `motor_lag`, `apply_forces`, `integrate_motion` | Yes | Physical reality — orientation-independent |
| `clamp_transform` | Yes | Floor collision works at any orientation |
| `dirty_air_perturbation` | Yes | Angular wobble works at any orientation |
| Gate detection (`race/gate.rs`) | Yes | Position-based plane crossing, not orientation-dependent |
| Obstacle collision (`race/collision.rs`) | Yes | Position-based swept segment test |
| **`position_pid` — tilt clamp** | **No** | 83-degree clamp blocks inversion (needs 180-degree for Split-S) |
| **`position_pid` — thrust calc** | **No** | `desired_accel.y / cos_tilt` breaks when inverted (`cos_tilt <= 0`) |
| **`position_pid` — position tracking** | **No** | Fights maneuver deviation from spline |
| **`compute_racing_line` — speed limit** | **No** | Would brake the drone at the maneuver entry point |

**Solution:** Skip `position_pid` entirely during flip maneuvers (Split-S, Power Loop) via `Without<ActiveManeuver>` query filter. The maneuver system writes directly to `DesiredAttitude`. For Aggressive Banking (70-120 degree turns), a lighter `TiltOverride` component raises the tilt limit while keeping the PID active.

---

## Architecture Overview

```
                  ┌──────────────────────────────────┐
                  │ trigger_maneuvers                 │
                  │ (detects tight turns, inserts     │
                  │  ActiveManeuver or TiltOverride)  │
                  └──────────┬───────────────────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │ Has ActiveManeuver?                │
           │                                   │
     ┌─────▼──────┐                    ┌───────▼──────┐
     │ execute_    │                   │ position_pid  │
     │ maneuvers   │                   │ (+ TiltOver-  │
     │ (writes     │                   │  ride if set) │
     │ Desired-    │                   │ (normal path) │
     │ Attitude    │                   └───────┬───────┘
     │ directly)   │                           │
     └─────┬───────┘                           │
           │                                   │
           └──────────┬────────────────────────┘
                      │
              DesiredAttitude
                      │
              attitude_controller → motor_lag → apply_forces → integrate_motion
```

Two integration paths, same downstream physics:
- **Split-S / Power Loop:** `ActiveManeuver` component, full PID bypass, direct attitude+thrust control
- **Aggressive Bank:** `TiltOverride` component, PID still runs with raised limit (~103 degrees)

---

## Phase 1: Data Types and Components

**Goal:** Define all types. No behavior changes — project compiles, all tests pass.

### New file: `src/drone/maneuver/mod.rs`

```rust
pub mod profiles;
pub mod detection;
pub mod trigger;
pub mod execution;
pub mod cleanup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverKind { SplitS, PowerLoop, AggressiveBank }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManeuverPhaseTag { Entry, Ballistic, Recovery }

#[derive(Component)]
pub struct ActiveManeuver {
    pub kind: ManeuverKind,
    pub phase: ManeuverPhaseTag,
    pub phase_progress: f32,          // 0.0 → 1.0 within current phase
    pub phase_start_time: f32,
    pub phase_duration: f32,
    pub entry_velocity: Vec3,
    pub entry_position: Vec3,
    pub exit_spline_t: f32,           // where to resume normal flight
    pub entry_yaw_dir: Vec3,
    pub entry_altitude: f32,
}

/// Lighter alternative for aggressive banking: PID still active, just with a raised tilt limit.
#[derive(Component)]
pub struct TiltOverride {
    pub max_tilt: f32,
    pub exit_spline_t: f32,
}
```

### Modified: `src/drone/components.rs`

Add to `DroneConfig`:
```rust
/// Multiplier on maneuver trigger threshold.
/// <1.0 = triggers maneuvers at lower turn angles (more acrobatic).
/// >1.0 = prefers banking over flipping (conservative).
pub maneuver_threshold_mult: f32,  // 0.7–1.3, default 1.0
```

Add to `tuning_params!` macro:
```rust
maneuver_turn_threshold,  "Maneuver Thresh",  5.0,   30.0, 150.0, 90.0;
maneuver_altitude_min,    "Maneuver Alt",     0.5,   1.0,  10.0,  3.0;
maneuver_enabled,         "Maneuvers On",     1.0,   0.0,  1.0,   1.0;
```

### Modified: `src/pilot/personality.rs`

Add `maneuver_threshold` to `TraitModifiers`:
```rust
/// Additive adjustment to maneuver_threshold_mult.
/// Negative = flips earlier (more acrobatic). Positive = prefers banking.
pub maneuver_threshold: f32,
```

Values by personality:
| Personality | `maneuver_threshold` |
|-------------|---------------------|
| Aggressive  | -0.08               |
| Cautious    | +0.10               |
| Flashy      | -0.15               |
| Methodical  | +0.05               |
| Reckless    | -0.12               |
| Smooth      | +0.03               |
| Technical   | -0.03               |
| Hotdog      | -0.20               |

### Modified: `src/pilot/skill.rs`

Add `maneuver_threshold_mult` generation from skill profile:
```rust
let maneuver_range = 0.15 * range_factor(s_corner);
let maneuver_threshold_mult = 1.0 + rng.gen_range(-maneuver_range..=maneuver_range);
// Apply personality modifier, clamp to 0.7..1.3
```

### Modified: `src/drone/spawning.rs`

Add `maneuver_threshold_mult` to `randomize_drone_config()` (default range: `rng.gen_range(0.85..=1.15)`). Add field to neutral test config (value: 1.0). Update test assertions for bounds.

### Modified: `src/drone/mod.rs`

Add `pub mod maneuver;`.

**Verification:** `cargo build && cargo clippy && cargo test`

---

## Phase 2: PID Bypass and Execution System

**Goal:** Add the system that drives maneuver attitude/thrust and modify `position_pid` to skip maneuvering drones.

### New file: `src/drone/maneuver/execution.rs`

`execute_maneuvers` system — runs after `trigger_maneuvers`, before `position_pid`:
- Advances `phase_progress` from elapsed time
- Computes target orientation via `profiles::maneuver_target_orientation()`
- Computes thrust fraction via `profiles::maneuver_thrust_fraction()`
- Writes to `DesiredAttitude` directly (the key bypass — `attitude_controller` reads this as usual)
- Transitions phases: Entry → Ballistic → Recovery → complete

### New file: `src/drone/maneuver/cleanup.rs`

`cleanup_completed_maneuvers` system — runs after `clamp_transform`:
- When Recovery phase reaches progress 1.0: remove `ActiveManeuver`, reset `PositionPid.integral = Vec3::ZERO` (prevents windup kick), set `ai.spline_t = exit_spline_t`
- Also removes `ActiveManeuver` from crashed drones

`cleanup_tilt_overrides` system:
- When `ai.spline_t >= tilt.exit_spline_t`: remove `TiltOverride`

### Modified: `src/drone/physics.rs` — `position_pid`

Two changes:
1. Add `Without<ActiveManeuver>` filter to query (skips drones with full maneuvers)
2. Add `Option<&TiltOverride>` to query; use `tilt_override.map(|o| o.max_tilt).unwrap_or(tuning.max_tilt_angle)` for the tilt clamp

### Modified: `src/drone/ai/racing_line.rs` — `compute_racing_line`

Add `Option<&ActiveManeuver>` to query. When present, set `desired.max_speed = tuning.max_speed` (bypass curvature-based braking during maneuvers).

### Modified: `src/drone/mod.rs` — FixedUpdate chain

Restructure into two chained groups (12-system limit per `run_if` tuple):

```rust
// Group 1: AI + maneuver + physics (12 systems)
.add_systems(
    FixedUpdate,
    (
        ai::update_ai_targets.run_if(drones_are_active),
        ai::compute_racing_line.run_if(drones_are_active),
        ai::proximity_avoidance.run_if(drones_are_active),
        wander::update_wander_targets.run_if(drones_are_active),
        physics::hover_target.run_if(not(drones_are_active)),
        maneuver::trigger::trigger_maneuvers.run_if(drones_are_active),
        maneuver::execution::execute_maneuvers,
        physics::position_pid,
        physics::attitude_controller,
        physics::dirty_air_perturbation,
        physics::motor_lag,
        physics::apply_forces,
    )
        .chain()
        .run_if(in_race_or_results),
)

// Group 2: tail (3 systems, ordered after apply_forces)
.add_systems(
    FixedUpdate,
    (
        physics::integrate_motion,
        physics::clamp_transform,
        maneuver::cleanup::cleanup_completed_maneuvers,
    )
        .chain()
        .after(physics::apply_forces)
        .run_if(in_race_or_results),
)

// Tilt override cleanup (independent)
.add_systems(
    FixedUpdate,
    maneuver::cleanup::cleanup_tilt_overrides
        .after(maneuver::cleanup::cleanup_completed_maneuvers)
        .run_if(in_race_or_results),
)
```

**Verification:** `cargo build && cargo clippy && cargo test`, then manual flight test — drones should fly normally (no maneuvers triggered yet).

---

## Phase 3: Detection and Triggering

**Goal:** Implement the detection algorithm and trigger system that inserts `ActiveManeuver`/`TiltOverride`.

### New file: `src/drone/maneuver/detection.rs`

Pure function `detect_maneuver()`:
1. Sample 8 tangent vectors over the next 1.5 `POINTS_PER_GATE` of spline
2. Accumulate total direction change (sum angles between successive tangents)
3. If turn angle exceeds threshold (scaled by `cornering_aggression * maneuver_threshold_mult`):
   - Turn > 120 degrees + altitude > `maneuver_altitude_min` → **Split-S**
   - Turn > 120 degrees + low altitude → **Power Loop**
   - Turn 70–120 degrees at high speed → **Aggressive Bank**
4. Compute `exit_t`: spline parameter past the tight section where the tangent realigns

Guards:
- **Gate proximity:** Don't trigger within 2m of a gate center
- **Finish line:** Don't trigger if `exit_t` would exceed `finish_t`
- **Minimum speed:** Don't trigger below 10 m/s
- **Cooldown:** Don't trigger if `spline_t` is within 0.5 of a recently completed maneuver's `exit_t`

Return type:
```rust
pub struct ManeuverTrigger {
    pub kind: ManeuverKind,
    pub exit_t: f32,
    pub turn_angle: f32,
}
```

### New file: `src/drone/maneuver/trigger.rs`

`trigger_maneuvers` system:
- Query: drones `Without<ActiveManeuver>` and `Without<TiltOverride>` (prevent re-triggering)
- Runs only in Racing/VictoryLap phases
- Calls `detect_maneuver()` per drone
- For Split-S/Power Loop: inserts `ActiveManeuver` via `commands`
- For Aggressive Bank: inserts `TiltOverride` via `commands`

**Personality-driven variation:** A Reckless pilot (aggression ~1.15, threshold_mult ~0.80) triggers Split-S at ~72 degree turns. A Cautious pilot (aggression ~0.90, threshold_mult ~1.10) only at ~99 degrees. Same corner, different behavior — automatic visual variety.

**Verification:** `cargo build && cargo clippy && cargo test`, then manual flight test on a course with tight hairpins.

---

## Phase 4: Attitude Profiles

**Goal:** Implement the specific orientation and thrust curves that make maneuvers look correct.

### New file: `src/drone/maneuver/profiles.rs`

All pure functions — fully unit-testable.

**Split-S profile:**
| Phase | Duration | Orientation | Thrust |
|-------|----------|-------------|--------|
| Entry | ~0.08–0.12s | Roll 180 degrees around velocity axis | 20% |
| Ballistic | ~0.3–0.5s | Slerp from inverted to level-exit (nose pulls through) | `0.3 + 0.6 * progress^2` |
| Recovery | ~0.15–0.25s | Level flight, exit direction = opposite of entry | 100% |

**Power Loop profile:**
| Phase | Duration | Orientation | Thrust |
|-------|----------|-------------|--------|
| Entry | ~0.15–0.2s | Pitch up 45 degrees | 100% |
| Ballistic | ~0.4–0.6s | Pitch backward through 360 degrees (over the top) | ~40% at top, ramp to 80% |
| Recovery | ~0.15s | Level flight, original heading | 90% |

**Key implementation details:**
- **Smoothstep interpolation** (`t^2 * (3 - 2t)`) for all phase transitions — zero derivative at endpoints prevents angular velocity spikes
- **Durations scale with entry speed:** Faster = shorter entry phase
- **Exit orientation for Split-S:** Heading reversed 180 degrees from entry (drone exits going the opposite way)
- **Exit orientation for Power Loop:** Same heading as entry (drone loops back to original direction)

**Tests:**
- Split-S entry at progress=1.0 → inverted (body-up points down)
- Split-S ballistic at progress=1.0 → level, heading reversed
- Power Loop ballistic at progress=0.5 → fully inverted at the top
- All thrust fractions in [0.0, 1.0]
- Smoothstep(0) = 0, smoothstep(1) = 1, smoothstep(0.5) = 0.5

**Verification:** `cargo build && cargo clippy && cargo test`, manual flight test. Split-S should look like a half-flip + dive-through. Power Loop should look like a backward loop gaining altitude.

---

## Phase 5: Debug Visualization and Polish

**Goal:** Add debug gizmos, handle remaining edge cases.

### Modified: `src/drone/debug_draw.rs`

New `draw_maneuver_state` system (behind F3 toggle):
- Colored ring around maneuvering drones (red = Split-S, blue = Power Loop, yellow = Aggressive Bank)
- Arrow showing target orientation
- Line from drone to exit point on spline

### Edge cases to verify:
| Edge Case | Handling |
|-----------|----------|
| Ground collision during Split-S | `clamp_transform` prevents going below ground + altitude guard in detection |
| Gate trigger mid-flip | Position-based plane crossing — works unchanged |
| Crash during maneuver | `execute_maneuvers` skips crashed drones; cleanup removes `ActiveManeuver` |
| Back-to-back maneuvers | After cleanup, next tick can trigger again (cooldown guard prevents instant re-trigger) |
| Cycle boundary in VictoryLap | `exit_spline_t` wraps cyclically |
| Dev dashboard toggle | `maneuver_enabled = 0` stops new triggers; in-progress maneuvers complete naturally |
| `spline_t` during maneuver | Frozen (not advanced by `update_ai_targets` since forward projection ≈ 0 during flip); jumps to `exit_spline_t` on cleanup |
| Miss detection during maneuver | `spline_t` frozen → won't exceed miss threshold; cleanup advances past the gate |

### Performance budget:
- `trigger_maneuvers`: ~12 drones × 8 polynomial evals ≈ 5–10 microseconds/tick
- `execute_maneuvers`: 0–3 drones × (1 slerp + 1 polynomial) ≈ 1–2 microseconds/tick
- Total: well within the 15ms/tick budget at 64Hz

**Verification:** Full post-phase checklist — `cargo build && cargo clippy && cargo test` + manual testing on multiple course layouts with varying turn tightness.

---

## Files Summary

### New (6 files)
| File | Purpose |
|------|---------|
| `src/drone/maneuver/mod.rs` | Types: `ActiveManeuver`, `TiltOverride`, `ManeuverKind`, `ManeuverPhaseTag` |
| `src/drone/maneuver/profiles.rs` | Pure fns: orientation + thrust curves per maneuver kind |
| `src/drone/maneuver/detection.rs` | Pure fn: spline analysis → maneuver selection |
| `src/drone/maneuver/trigger.rs` | System: inserts `ActiveManeuver`/`TiltOverride` on drones |
| `src/drone/maneuver/execution.rs` | System: advances phases, writes `DesiredAttitude` directly |
| `src/drone/maneuver/cleanup.rs` | Systems: removes completed maneuvers, resets PID integral |

### Modified (8 files)
| File | Changes |
|------|---------|
| `src/drone/mod.rs` | Add `pub mod maneuver`, restructure FixedUpdate chain |
| `src/drone/components.rs` | Add `maneuver_threshold_mult` to DroneConfig, 3 tuning params |
| `src/drone/physics.rs` | `Without<ActiveManeuver>` on `position_pid`, `Option<&TiltOverride>` for tilt limit |
| `src/drone/ai/racing_line.rs` | Bypass speed limiting during maneuvers |
| `src/drone/spawning.rs` | Add `maneuver_threshold_mult` to config generation |
| `src/drone/debug_draw.rs` | `draw_maneuver_state` gizmo system |
| `src/pilot/personality.rs` | Add `maneuver_threshold` to `TraitModifiers` |
| `src/pilot/skill.rs` | Add `maneuver_threshold_mult` generation |

### Unchanged (verified compatible)
| File | Why unchanged |
|------|---------------|
| `src/drone/physics.rs` (attitude_controller, motor_lag, apply_forces, integrate_motion, clamp_transform) | Orientation-independent physical simulation |
| `src/race/gate.rs` | Position-based plane crossing |
| `src/race/collision.rs` | Position-based swept segment test |
| `src/race/lifecycle.rs` | Phase transitions unaffected |
| `src/drone/interpolation.rs` | Works for any transform |
| `src/camera/*.rs` | Follow position/rotation — flips create desired dramatic camera effect |
