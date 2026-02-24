# Drone Realism Improvements — Implementation Plan

## Overview

Seven changes to make drone racing look and feel authentic, based on real FPV racing drone research. Organized into 3 phases by dependency order. No new systems added — all changes extend existing code.

## Phase A: Per-Drone Personality & Tuning (components + spawning)

Foundational data changes that other phases depend on.

### A1. Extend `DroneConfig` with new per-drone fields

**File: `src/drone/components.rs`**

Add four new fields to `DroneConfig`:
```rust
pub cornering_aggression: f32,   // 0.8..1.2 — multiplier on safe_lateral_accel
pub braking_distance: f32,       // 0.8..1.2 — multiplier on speed_curvature_range
pub attitude_kp_mult: f32,       // 0.9..1.1 — per-drone attitude stiffness variation
pub attitude_kd_mult: f32,       // 0.9..1.1 — per-drone attitude damping variation
```

### A2. Update `AttitudePd` defaults for snappier, slightly underdamped response

**File: `src/drone/components.rs`**

Change `AttitudePd::default()`:
- `kp_roll_pitch`: 5.0 → **7.0** (crisper bank-in)
- `kd_roll_pitch`: 0.24 → **0.20** (allows slight overshoot = visible settle wobble)

### A3. Reduce motor lag for racing feel

**File: `src/drone/components.rs`**

Change `DroneDynamics::default()`:
- `motor_time_constant`: 0.040 → **0.025** (snappier spool-up)

### A4. Randomize new DroneConfig fields at spawn

**File: `src/drone/spawning.rs`**

In `randomize_drone_config()`, add:
```rust
cornering_aggression: rng.gen_range(0.8..=1.2),
braking_distance: rng.gen_range(0.8..=1.2),
attitude_kp_mult: rng.gen_range(0.9..=1.1),
attitude_kd_mult: rng.gen_range(0.9..=1.1),
```

### A5. Apply per-drone attitude gains at spawn

**File: `src/drone/spawning.rs`**

In `spawn_drones()`, after creating `AttitudePd::default()`, multiply by config multipliers:
```rust
let mut attitude_pd = AttitudePd::default();
attitude_pd.kp_roll_pitch *= config.attitude_kp_mult;
attitude_pd.kd_roll_pitch *= config.attitude_kd_mult;
```

### A6. Update existing tests

**File: `src/drone/spawning.rs`**

Update `randomize_drone_config_within_bounds` test to assert the 4 new fields are within their ranges. Update `return_path_produces_valid_spline` and other tests that construct `DroneConfig` literals to include the new fields.

---

## Phase B: AI Behavior & Physics Improvements

Uses the new per-drone fields. Extends existing systems.

### B1. Per-drone cornering aggression in `compute_racing_line`

**File: `src/drone/ai.rs`**

In `compute_racing_line()`, in the `DronePhase::Racing` branch where `safe_speed_for_curvature` is called, scale `safe_lateral_accel` by the drone's `cornering_aggression`:

Add `&DroneConfig` to the query. Then:
```rust
// Scale the per-drone speed limit by cornering aggression
let per_drone_accel = tuning.safe_lateral_accel * config.cornering_aggression;
// Also scale braking scan range by braking_distance
let per_drone_range = tuning.speed_curvature_range * config.braking_distance;
let max_k = max_curvature_ahead(&ai.spline, ai.spline_t, per_drone_range, cycle_t);
desired.max_speed = safe_speed_for_curvature_with(max_k, per_drone_accel, &tuning);
```

Add a helper variant (or modify `safe_speed_for_curvature` to accept lateral accel as a parameter):
```rust
pub fn safe_speed_for_curvature_with(curvature: f32, lateral_accel: f32, tuning: &AiTuningParams) -> f32 {
    if curvature > 0.001 {
        (lateral_accel / curvature).sqrt().clamp(tuning.min_curvature_speed, tuning.max_speed)
    } else {
        tuning.max_speed
    }
}
```

Update `safe_speed_for_curvature` to delegate to the new function (preserves existing callers like the flight spline preview).

### B2. Fake dirty air / pack dynamics

**File: `src/drone/physics.rs`**

Add a new system `dirty_air_perturbation` that runs in FixedUpdate, inserted into the chain **after** `attitude_controller` and **before** `motor_lag`. This is the one new system — it must exist separately because it queries all drones pairwise.

Algorithm (O(n²) with n=12, so 144 cheap checks per tick):
```
For each drone A:
  For each other drone B:
    let to_a = A.position - B.position
    let dist = to_a.length()
    if dist > DIRTY_AIR_RADIUS (5.0m): skip
    if dist < 0.1: skip  (same position edge case)

    // Check if A is behind B (in B's wake cone)
    let b_vel_dir = B.velocity.normalize_or(skip)
    let behind_dot = (-to_a.normalize()).dot(b_vel_dir)
    if behind_dot < WAKE_CONE_COS (cos(45°) ≈ 0.707): skip

    // Strength: falls off with distance, scales with leader's speed
    let strength = (1.0 - dist / DIRTY_AIR_RADIUS) * B.speed / tuning.max_speed

    // Apply random angular perturbation to A
    let perturbation = pseudo_random_vec3(time, A.index) * DIRTY_AIR_TORQUE * strength
    A.angular_velocity += perturbation * dt
```

Constants (could add `dirty_air_strength` to `AiTuningParams`):
- `DIRTY_AIR_RADIUS`: 5.0
- `WAKE_CONE_COS`: 0.707 (45° half-angle)
- `DIRTY_AIR_TORQUE`: 8.0 (rad/s² base perturbation)

The pseudo-random uses layered sin waves keyed on drone index + time (same pattern as hover noise) to avoid needing `rand` in FixedUpdate.

### B3. Fake prop wash (descent perturbation)

**File: `src/drone/physics.rs`**

Extend the `dirty_air_perturbation` system (or add it there for simplicity). After the pairwise loop, for each drone independently:
```rust
// Prop wash: perturbation when descending
let descent_rate = (-dynamics.velocity.y).max(0.0);
if descent_rate > 2.0 {
    let wash_strength = ((descent_rate - 2.0) / 10.0).min(1.0);
    let wash_noise = pseudo_random_vec3(time, drone.index + 100) * PROP_WASH_TORQUE * wash_strength;
    dynamics.angular_velocity += wash_noise * dt;
}
```
`PROP_WASH_TORQUE`: 5.0 (rad/s² base, tuned to be visible but not destabilizing)

### B4. Battery sag

**File: `src/drone/components.rs`** — Add to `AiTuningParams`:
```rust
pub battery_sag_factor: f32,  // Default: 0.15 (15% thrust loss over full race)
```
Add corresponding `PARAM_META` entry, update `get()`/`set()` match arms, bump array size to 10.

**File: `src/drone/physics.rs`** — In `apply_forces()`:

Read `RaceClock` (optional resource). If running, compute race progress as `(clock.elapsed / RACE_DURATION_ESTIMATE).min(1.0)` where `RACE_DURATION_ESTIMATE = 90.0` seconds. Scale effective max_thrust:
```rust
let sag = if let Some(clock) = race_clock {
    let progress = (clock.elapsed / 90.0).min(1.0);
    1.0 - tuning.battery_sag_factor * progress
} else {
    1.0
};
let effective_thrust = dynamics.thrust * sag;
// Use effective_thrust instead of dynamics.thrust for the thrust_force calculation
```

This requires adding `Option<Res<RaceClock>>` to the `apply_forces` system signature and importing the type.

### B5. Scaled approach offset

**File: `src/drone/spawning.rs`** — In `generate_race_path()`:

Replace the fixed `APPROACH_OFFSET = 12.0` with per-gate adaptive offset:
```rust
const MAX_APPROACH_OFFSET: f32 = 12.0;
const APPROACH_FRACTION: f32 = 0.3;

// Inside the loop:
let next = (i + 1) % n;
let gate_dist = (gate_positions[next] - gate_positions[i]).length();
let approach_offset = (gate_dist * APPROACH_FRACTION).min(MAX_APPROACH_OFFSET);
```

Each gate gets its own offset based on distance to the next gate. This prevents the 24m of "committed direction" from consuming too much of a short inter-gate segment.

**File: `src/drone/debug_draw.rs`** — The `draw_gate_markers` function has its own `APPROACH_OFFSET = 12.0` constant. Since offsets are now per-gate and baked into the spline, update this to compute the same adaptive offset. Two approaches:
1. Store the per-gate approach offsets in `RacePath` and propagate to `AIController`
2. Recompute in debug_draw using the same formula

Option 2 is simpler and avoids expanding AIController. Add the same `MAX_APPROACH_OFFSET`/`APPROACH_FRACTION` constants and compute per-gate in the draw loop using `gate_positions`.

**File: `src/editor/course_editor/mod.rs`** — The flight spline preview calls `generate_race_path()` which will automatically use the new adaptive offset. No changes needed.

---

## Phase C: Dashboard + Documentation Update

### C1. Add new tuning params to dev dashboard

**File: `src/drone/components.rs`**:

New PARAM_META entries (expanding from 9 to 11):
```rust
ParamMeta { name: "Battery Sag",     step: 0.05, min: 0.0,  max: 0.4  },  // Index 9
ParamMeta { name: "Dirty Air Str",   step: 1.0,  min: 0.0,  max: 20.0 },  // Index 10
```

Add `dirty_air_strength: f32` (default 8.0) to `AiTuningParams`.

Update `get()` and `set()` match arms for indices 9 and 10.

### C2. Update documentation

- **CLAUDE.md**: Add conventions for the new per-drone config fields, dirty air system, battery sag pattern, and adaptive approach offset.
- **ARCHITECTURE.md**: Update drone module section if needed.
- **TODO.md**: No change (this is an enhancement, not a phase from the todo list).

---

## System Chain Update

**File: `src/drone/mod.rs`**

Insert `dirty_air_perturbation` into the FixedUpdate chain after `attitude_controller`:
```rust
(
    ai::update_ai_targets.run_if(drones_are_active),
    ai::compute_racing_line.run_if(drones_are_active),
    physics::hover_target.run_if(not(drones_are_active)),
    physics::position_pid,
    physics::attitude_controller,
    physics::dirty_air_perturbation,  // NEW — after attitude, before motor lag
    physics::motor_lag,
    physics::apply_forces,
    physics::integrate_motion,
    physics::clamp_transform,
)
    .chain()
    .run_if(in_state(AppState::Race)),
```

This keeps the chain at 10 systems (under the ~12 tuple limit).

---

## Summary of Changes by File

| File | Changes |
|------|---------|
| `components.rs` | 4 new DroneConfig fields, AttitudePd default tune, DroneDynamics motor lag, 2 new AiTuningParams fields, PARAM_META → [11] |
| `spawning.rs` | Randomize 4 new config fields, apply attitude multipliers at spawn, adaptive approach offset |
| `ai.rs` | Per-drone cornering aggression + braking distance in compute_racing_line, new safe_speed helper |
| `physics.rs` | New `dirty_air_perturbation` system (dirty air + prop wash), battery sag in apply_forces |
| `mod.rs` | Insert dirty_air_perturbation in chain |
| `debug_draw.rs` | Adaptive approach offset in gate marker debug draw |
| `dev_dashboard.rs` | No structural changes (auto-picks up new PARAM_META entries) |

## Performance Analysis

- **Dirty air**: 12×12 = 144 distance checks + dot products per FixedUpdate tick. Each is ~5 arithmetic ops. Negligible.
- **Prop wash**: 12 checks per tick (one per drone). Negligible.
- **Battery sag**: 1 multiply per drone per tick. Negligible.
- **Adaptive offset**: Computed once at race start during `generate_race_path`. Zero runtime cost.
- **Per-drone cornering**: Same cost as before — just uses a different multiplier. Zero additional cost.
- **Attitude tuning**: Same cost — just different default values.

All changes are well within the 60fps budget.
