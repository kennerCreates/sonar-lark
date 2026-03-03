# Choreographed Spline Racing — Implementation Plan

## Context

The game is a drone racing league organizer (management game). Races are non-interactive — the player watches. The current physics engine produces realistic flight but can't do acrobatic maneuvers (tilt clamp prevents inversion), and a previous attempt at physics-based acrobatics was reverted after 7 iterations because of fundamental control instability.

**New approach: "WWE, not true wrestling."** Predetermine the race outcome (finish order, crashes, acrobatics) from pilot skill + randomness, then play it out visually via spline-following with procedural banking and acrobatic rotation keyframes. The physics engine is bypassed during races; drones slide along their existing per-drone splines at curvature-based speeds with pacing adjustments to hit scripted finish times.

**What changes:**
- AI targeting + physics chain replaced by choreography during Race state
- Gate/collision detection replaced by scripted events from spline_t thresholds
- Acrobatic flips are rotation keyframes, not physics

**What stays unchanged:**
- Per-drone spline generation (`generate_drone_race_path`)
- Cameras (read same components: Transform, DroneDynamics.velocity, AIController.spline/spline_t)
- Leaderboard (reads RaceProgress)
- Explosions/fireworks (called by scripted events the same way)
- Interpolation pipeline (FixedFirst/PreUpdate/PostUpdate)
- Results state wandering (physics chain active during Results only)
- Pilot skill → DroneConfig mapping
- Race countdown flow (START RACE → 3-2-1 → GO)

---

## Architecture Overview

```
OnEnter(Race)
  │
  ├── spawn_drones (existing — entities + per-drone splines + visuals)
  ├── generate_race_script (NEW — predetermines outcome from pilot data + course)
  │
  ▼ Countdown (existing timer, no changes)
  │
  ▼ Racing phase: FixedUpdate each tick
  │
  ├── advance_choreography (NEW — advance spline_t at paced speed, write Transform)
  ├── compute_choreographed_rotation (NEW — bank angle + acrobatic keyframes → rotation)
  ├── apply_visual_noise (NEW — attitude jitter, dirty air wobble)
  ├── fire_scripted_events (NEW — gate passes, crashes, finishes from spline_t)
  │
  ▼ All drones finished/crashed → RacePhase::Finished (existing lifecycle)
  │
  ▼ Results (existing — wandering uses physics chain, stats updated, etc.)
```

---

## Implementation Phases

| Phase | What | New/Modified Files | Risk | Status |
|-------|------|--------------------|------|--------|
| **1** | Race script generator | NEW `race/script.rs`, mod `race/mod.rs` | Low — pure data | Not started |
| **2** | Core spline playback | NEW `drone/choreography.rs`, mod `drone/mod.rs` | Medium — replaces physics chain | Not started |
| **3** | Scripted events | Extend `drone/choreography.rs`, mod `race/mod.rs` | Low — mirrors existing event flow | Not started |
| **4** | Scripted crashes | Extend `race/script.rs` + `drone/choreography.rs` | Low — ballistic + crash_drone() | Not started |
| **5** | Acrobatic rotation | Extend `drone/choreography.rs` | Medium — curve math + visual tuning | Not started |
| **6** | Visual polish | Extend `drone/choreography.rs` | Low — noise layers | Not started |

Each phase is independently testable. Phase 2 is the pivotal one — once drones follow splines visually, everything else layers on top.

---

## Phase 1: Race Script Generator

### New file: `src/race/script.rs`

#### Data structures

```rust
/// Predetermined outcome for a single drone.
pub struct DroneScript {
    /// Pacing multiplier: >1.0 = faster than base, <1.0 = slower.
    pub pace_factor: f32,
    /// If Some, drone crashes near this gate (index into gate sequence).
    pub crash: Option<CrashScript>,
    /// Gate indices where this drone performs an acrobatic maneuver.
    pub acrobatic_gates: Vec<u32>,
}

pub struct CrashScript {
    pub gate_index: u32,
    /// 0.0..1.0 within the turn — how far past the gate the crash occurs.
    pub progress_past_gate: f32,
    pub crash_type: ScriptedCrashType,
}

pub enum ScriptedCrashType {
    /// Clips an obstacle/gate frame during a tight turn.
    ObstacleCollision,
    /// Two drones collide (paired — other drone_idx stored).
    DroneCollision { other_drone_idx: u8 },
}

/// Complete predetermined race outcome.
#[derive(Resource)]
pub struct RaceScript {
    pub drone_scripts: Vec<DroneScript>,
}
```

#### Generation algorithm: `generate_race_script()`

**Inputs:** course gate data (positions, forwards), per-drone splines + configs, pilot data (SkillProfile, PersonalityTrait), RaceSeed, AiTuningParams.

**Steps:**

1. **Estimate base lap times**: For each drone, dry-run spline traversal at curvature-based speeds (reuse `safe_speed_for_curvature` math with per-drone `cornering_aggression`). Just a loop: advance `t += speed * dt / tangent_len`, count ticks. Cheap — pure math, <1ms for 12 drones.

2. **Compute turn tightness per gate**: For consecutive gates (i, i+1), compute direction change angle. Classify: <60° = gentle, 60-100° = medium, >100° = tight. Store per-gate.

3. **Assign finish order**: Sort drones by estimated_time. Add random perturbation (±5-15% inversely proportional to `skill.consistency`). This is the target finish order.

4. **Assign DNFs** (0-3 per race): For each drone, compute crash probability from `(1.0 - skill.level) * course_difficulty * personality_risk_factor` where Reckless/Aggressive increase risk, Cautious/Methodical decrease. Roll against RaceSeed hash. Crashed drones get a `CrashScript` at a tight-turn gate with some randomized progress past the gate.

5. **Assign acrobatics**: For each non-DNF drone at each tight turn: if `skill.cornering > 0.6` AND personality is Flashy/Hotdog/Aggressive (or skill.cornering > 0.85 for anyone), mark that gate as acrobatic. Cap at ~3-5 acrobatic gates per drone to avoid visual fatigue.

6. **Compute pace factors**: `pace_factor = estimated_time / target_finish_time`. For DNF drones, pace_factor is based on skill (they fly normally until the crash).

#### File modifications

- **`src/race/mod.rs`** — add `pub mod script`, register generation system
- **`src/race/lifecycle.rs`** — call `generate_race_script()` during countdown start (when `tick_countdown` transitions to Racing, same place RaceProgress is created). Insert `RaceScript` resource.

#### Reused code

- `safe_speed_for_curvature` math from `src/drone/ai/mod.rs` — extract to a pure function or call directly
- `cyclic_curvature` from `src/drone/ai/mod.rs` — already a pure function
- `DroneConfig.cornering_aggression`, `.braking_distance` — read from existing configs
- Gate positions/forwards from `AIController` on spawned drones

---

## Phase 2: Core Spline Playback

### New file: `src/drone/choreography.rs`

This is the pivotal phase. Drones follow their splines visually instead of through physics.

#### System: `advance_choreography` (FixedUpdate, chained, run_if Race state)

For each drone, based on `DronePhase`:

**Idle** (pre-countdown):
```
position = start_position + hover_noise(t, drone_index, config)
rotation = initial_rotation  // face first gate
velocity = Vec3::ZERO
```
Reuse the hover noise pattern from `hover_target` in `physics.rs` — same layered sine waves from `config.hover_noise_amp/freq`.

**Racing**:
```
curvature = cyclic_curvature(spline, spline_t, cycle_t)
base_speed = safe_speed_for_curvature(curvature, config, tuning)
speed = base_speed * script.pace_factor
tangent = spline.velocity(spline_t % cycle_t)
spline_t += speed * dt / tangent.length().max(0.01)
position = spline.position(spline_t % cycle_t)
```

Write to entity:
- `Transform.translation = position`
- `DroneDynamics.velocity = tangent.normalize() * speed` (for cameras)
- `AIController.spline_t = spline_t` (for FPV look-ahead, leaderboard)

**VictoryLap**: Same as Racing but cyclic (modulo cycle_t), no finish check.

**Crashed**: Handled in Phase 4. For now, skip crashed drones.

#### System: `compute_choreographed_rotation` (FixedUpdate, after advance_choreography)

Derive rotation from spline curvature:
```
tangent = spline.velocity(t).normalize()
accel = spline.acceleration(t)
centripetal = accel - tangent * tangent.dot(accel)  // perpendicular to tangent

kappa = centripetal.length() / spline.velocity(t).length_squared()
bank_angle = atan(speed² * kappa / GRAVITY).clamp(0, max_tilt)

// Bank direction: which way the turn curves
left = Vec3::Y.cross(tangent).normalize_or(Vec3::X)
bank_sign = centripetal.dot(left).signum()

// Build rotation: forward along tangent, tilted by bank
banked_up = Quat::from_axis_angle(tangent, -bank_sign * bank_angle) * Vec3::Y
rotation = look rotation toward -tangent with banked_up as up
```

Write `Transform.rotation = rotation`. (Acrobatic override added in Phase 5.)

#### System scheduling changes

**`src/drone/mod.rs`:**

The current FixedUpdate chain is:
```
AI systems → physics chain (11 systems)
```

Change to:
```
IF in Race state:
    choreography systems (.chain()): advance_choreography → compute_choreographed_rotation → (Phase 3: fire_scripted_events)
IF in Results state:
    existing AI + physics chain (for wandering)
```

The run conditions use `in_state(AppState::Race)` vs `in_state(AppState::Results)`.

The interpolation pipeline (FixedFirst/PreUpdate/PostUpdate) runs for both states — unchanged. The choreography writes to `Transform` in FixedUpdate at the same point the physics chain would, so interpolation works identically.

**Key component writes for camera compatibility:**

| Component | Written by choreography | Read by |
|-----------|------------------------|---------|
| `Transform.translation` | spline.position(t) | Chase, FPV, leaderboard (via RaceProgress) |
| `Transform.rotation` | curvature-based bank | Visual rendering |
| `DroneDynamics.velocity` | tangent * speed | Chase (heading, FOV), FPV (heading, FOV) |
| `AIController.spline_t` | advanced each tick | FPV (spline look-ahead), sync_spline_progress |
| `AIController.target_gate_index` | advanced on gate pass | Leaderboard display |
| `DronePhase` | transitions on events | Camera selection, all systems |

#### Results state transition

On `OnEnter(Results)`, `transition_to_wandering` converts VictoryLap → Wandering. The physics chain takes over for wandering behavior. For clean handoff:
- `DroneDynamics.velocity` is already set by choreography (tangent * speed)
- Zero `PositionPid.integral` for all drones in a new `OnEnter(Results)` system to prevent stale integral windup

---

## Phase 3: Scripted Events

Extend `src/drone/choreography.rs`.

#### System: `fire_scripted_events` (FixedUpdate, after advance_choreography)

Replace the current gate_trigger_check + obstacle_collision_check + miss_detection chain.

For each Racing drone, check spline_t against thresholds:

**Gate pass**: When `spline_t` crosses `gate_index * POINTS_PER_GATE` (specifically, previous_t < threshold AND current_t >= threshold):
- Call `progress.record_gate_pass(drone_idx, gate_idx)`
- Advance `AIController.target_gate_index`

**Finish**: When `spline_t >= gate_count * POINTS_PER_GATE + FINISH_EXTENSION`:
- Call `progress.record_finish(drone_idx, race_clock.elapsed)`
- Set `DronePhase::VictoryLap`

**Race complete**: When all drones finished or crashed → existing `check_race_finished` handles this (reads RaceProgress, unchanged).

#### File modifications

- **`src/race/mod.rs`** — remove `gate_trigger_check`, `obstacle_collision_check`, `miss_detection` from the Update chain during Race state. These systems no longer needed (events come from script). Keep `sync_spline_progress` and `check_race_finished` in Update (they read RaceProgress which the scripted events update).
- Note: `build_gate_planes` and `build_obstacle_collision_cache` can be skipped (not needed without live detection). But keeping them is harmless and they're gated by `not(resource_exists)`.

---

## Phase 4: Scripted Crashes

Both obstacle/gate crashes and drone-on-drone collisions are implemented from the start.

#### Script generation (in `generate_race_script()`)

**Obstacle crashes**: Assigned to drones with high crash probability at tight-turn gates. The crash point is `gate_t + progress_past_gate * POINTS_PER_GATE` — partway through the turn.

**Drone-on-drone crashes**: After computing all pace_factors, identify proximity windows — pairs of drones whose spline_t values will be close at the same real time (similar pace on the same gate segment). Check world-space proximity by sampling both splines at those t values. If positions are within ~3m, this is a collision candidate. Assign `DroneCollision` to one or both drones at that spline_t. The `other_drone_idx` field links paired crashes so both trigger at the same tick.

#### New component

```rust
#[derive(Component)]
pub struct BallisticState {
    pub velocity: Vec3,
}
```

#### Crash execution in `advance_choreography`

**Obstacle crash**: When a drone's `spline_t` reaches the crash point from its `CrashScript`:
1. Record current position + velocity (tangent * speed)
2. Compute crash trajectory: rotate velocity slightly toward nearest gate edge or obstacle position. Add a small lateral deflection (±2-4 m/s) so the drone visibly veers off-course.
3. Insert `BallisticState { velocity: crash_velocity }`
4. Fire `RaceEvent` to update RaceProgress with DNF

**Drone-on-drone crash**: When a drone's `spline_t` reaches a `DroneCollision` crash point:
1. Find the paired drone (from `other_drone_idx`)
2. Compute collision normal between the two drones' positions
3. Apply diverging velocities: each drone gets a velocity kick away from the other (±3-5 m/s lateral + upward component)
4. Insert `BallisticState` on both drones (if both are scripted to crash) or just the one that loses the encounter
5. Fire crash events for both/one

**Ballistic update** (in `advance_choreography` for drones with `BallisticState`):
```
velocity.y -= GRAVITY * dt
position += velocity * dt
if position.y <= GROUND_HEIGHT:
    call crash_drone()  // explosion, hide, zero velocity, update RaceProgress
    remove BallisticState
```

This produces a visible arc + explosion rather than an instant disappearance. For drone-on-drone crashes, both drones tumble away from each other before hitting the ground.

#### crash_drone() reuse

The existing `crash_drone()` in `src/race/collision.rs` handles everything: set phase, zero velocity, hide, spawn explosion, play sound, update RaceProgress. Call it exactly as the current collision system does.

---

## Phase 5: Acrobatic Maneuvers (Rotation + Position Offset)

Extend `compute_choreographed_rotation` and `advance_choreography` in `src/drone/choreography.rs`.

Acrobatics affect both **rotation** (the flip) and **position** (altitude dip/climb). In real drone racing, a Split-S causes a visible altitude dip because thrust points away from the ground while inverted, and gravity pulls the drone down. A power loop causes a visible climb. Without the position offset, flips look like a rotating sprite on rails.

#### Acrobatic window

For each gate marked in `DroneScript.acrobatic_gates`:
- Entry: `gate_t - ACRO_ENTRY_OFFSET` (e.g., 0.4 * POINTS_PER_GATE = 1.2 parameter units before gate)
- Exit: `gate_t + ACRO_EXIT_OFFSET` (e.g., 0.4 * POINTS_PER_GATE after gate)

Within this window, apply both rotation keyframes and position offset.

#### Position offset (in `advance_choreography`)

Applied on top of the spline position during the acrobatic window:

**Split-S** (tight turn, next gate at same or lower altitude):
```
t_local = (spline_t - entry_t) / (exit_t - entry_t)
dip = -dip_amount * sin(t_local * PI)
position = spline.position(spline_t) + Vec3::Y * dip
```

**Power loop** (tight turn, next gate is higher):
```
climb = climb_amount * sin(t_local * PI)
position = spline.position(spline_t) + Vec3::Y * climb
```

`dip_amount` / `climb_amount` scale with:
- **Speed**: faster drones carry more momentum → bigger displacement (speed / max_speed * base_dip)
- **Turn duration**: longer acrobatic windows → more time under gravity → bigger dip
- **Base values**: ~2-5 meters. Tunable constant.

Optional lateral offset (drift wider during the flip, snap back on exit):
```
lateral_dir = Vec3::Y.cross(tangent).normalize()
lateral_offset = drift_amount * sin(t_local * PI) * roll_sign
position += lateral_dir * lateral_offset
```
`drift_amount` ~0.5-1.5 meters. Subtle but adds realism — the drone slides slightly wide while inverted.

The velocity written to `DroneDynamics.velocity` should include the offset derivative so cameras track smoothly:
```
velocity = tangent.normalize() * speed + Vec3::Y * (-dip_amount * PI * cos(t_local * PI) * dt_local_per_sec)
```

#### Rotation keyframes (in `compute_choreographed_rotation`)

**Split-S:**
```
t_local = (spline_t - entry_t) / (exit_t - entry_t)  // 0.0 to 1.0

keyframes:
  0.00: entry_rotation (normal bank at entry)
  0.10: begin roll (rotate ~90° around velocity axis)
  0.25: inverted (180° roll)
  0.50: nose-down, pulling through the bottom of the arc
  0.75: recovering (roll back toward level)
  0.90: nearly level
  1.00: exit_rotation (normal bank at exit)
```

**Power loop:**
```
keyframes:
  0.00: entry_rotation (normal bank)
  0.15: pitch up ~45°
  0.35: nose-up, climbing over the top
  0.50: inverted at apex
  0.65: nose-down, pulling through
  0.85: recovering
  1.00: exit_rotation (normal bank)
```

Slerp between adjacent keyframes with smoothstep easing.

#### Maneuver type selection

The script generator (Phase 1) marks which gates get acrobatics. In Phase 5, the **maneuver type** is chosen based on course geometry:
- Next gate is at similar or lower altitude → **Split-S** (dip through the turn)
- Next gate is significantly higher → **Power loop** (climb over the turn)
- Medium turn (70-100°) with high skill → **Aggressive bank** (no flip, just bank past 90° with a small dip). Uses simpler keyframes: just deeper bank angle, ~1m altitude offset.

#### Variation per drone

- **Roll direction**: from `DroneConfig.racing_line_bias` sign (positive = roll right, negative = roll left)
- **Smoothness**: high skill = clean slerp. Low skill = add small random rotation noise to intermediate keyframes (±5-10° wobble)
- **Aggressiveness**: high `cornering_aggression` = deeper bank on entry/exit, larger dip/climb amount
- **Dip magnitude**: scales with speed (fast drones dip more — they carry more momentum through the maneuver)

#### Blend zones

4-8 frames at entry and exit: slerp between normal bank rotation and acrobatic rotation using smoothstep. Position offset uses `sin(t_local * PI)` which naturally starts and ends at zero, so no position blending needed.

---

## Phase 6: Visual Polish

Extend `src/drone/choreography.rs`.

#### System: `apply_visual_noise` (FixedUpdate, after compute_choreographed_rotation)

**Attitude jitter**: Small-amplitude rotation noise simulating flight controller hunting.
```
jitter_amp = BASE_JITTER * (1.0 + (1.0 - skill.consistency) * SKILL_JITTER_SCALE)
noise = Vec3::new(
    sin(t * freq1 + phase) * jitter_amp,
    sin(t * freq2 + phase) * jitter_amp * 0.3,  // less yaw jitter
    sin(t * freq3 + phase) * jitter_amp,
)
rotation *= Quat::from_euler(EulerRot::XYZ, noise.x, noise.y, noise.z)
```

**Dirty air wobble**: When drone B is within proximity of drone A (check world-space distance), increase B's jitter amplitude. Same visual effect as current dirty_air_perturbation but applied to rotation directly instead of through angular velocity.

**Position micro-drift**: Tiny lateral noise (1-3cm) so drones don't look rigidly locked to the spline. Same layered-sine pattern as existing hover noise, scaled down to ~1cm amplitude.

---

## File Change Summary

| File | Change |
|------|--------|
| **NEW `src/race/script.rs`** | RaceScript, DroneScript, generate_race_script(), estimate_lap_time() |
| **NEW `src/drone/choreography.rs`** | advance_choreography, compute_choreographed_rotation, fire_scripted_events, apply_visual_noise, BallisticState |
| `src/race/mod.rs` | Add `pub mod script`. Remove gate/collision/miss detection from Race chain. Keep sync_spline_progress + check_race_finished. Register script generation system. |
| `src/drone/mod.rs` | Add `pub mod choreography`. Register choreography systems for Race state. Gate existing AI+physics chain to Results state only. Keep interpolation for both states. |
| `src/race/lifecycle.rs` | Insert `RaceScript` resource alongside `RaceProgress` when race starts. Clean up `RaceScript` on race exit. |
| `src/drone/components.rs` | Add `BallisticState` component. |

## Files explicitly NOT changed

- `src/camera/chase.rs` — reads Transform, DroneDynamics.velocity, DronePhase, RaceProgress (all provided by choreography)
- `src/camera/fpv.rs` — reads Transform, DroneDynamics.velocity, AIController.spline/spline_t (all provided)
- `src/race/leaderboard.rs` — reads RaceProgress (updated by fire_scripted_events)
- `src/race/progress.rs` — RaceProgress resource unchanged
- `src/drone/explosion.rs` — spawn_explosion() called by crash_drone() unchanged
- `src/drone/fireworks.rs` — detects finish from RaceProgress unchanged
- `src/drone/interpolation.rs` — interpolation pipeline unchanged
- `src/drone/wander.rs` — wandering during Results unchanged (physics chain active)
- `src/results/` — reads RaceResults unchanged
- `src/pilot/` — skill/personality system unchanged
- `src/drone/paths/` — spline generation unchanged

---

## Verification

### Per-phase

**Phase 1**: Unit tests for script generation — finish order respects skill ranking (statistically), crash count within expected range, acrobatic gates only on tight turns.

**Phase 2**: Run a race — drones visibly follow their splines, banking into turns. Cameras track correctly. No physics artifacts. Leaderboard shows progress via sync_spline_progress.

**Phase 3**: Gates register, finishers appear on leaderboard with times, fireworks trigger on first finish, race ends and transitions to Results.

**Phase 4**: Some drones crash during tight turns — visible ballistic arc then explosion. RaceProgress shows DNFs. Results screen shows correct crash count.

**Phase 5**: Skilled drones flip at tight turns. Unskilled drones bank normally. Entry/exit blending is smooth (no rotation pop).

**Phase 6**: Drones have visible attitude jitter. Drones in close proximity wobble more. Micro-drift prevents robotic look.

### End-to-end

- `cargo build && cargo clippy && cargo test`
- Play 5+ races on courses with varying turn tightness
- Verify: finish order roughly correlates with pilot skill
- Verify: cameras (Chase, FPV, CourseCamera) all work
- Verify: crashes look dramatic, not instant-disappear
- Verify: acrobatic flips look smooth at tight turns
- Verify: Results screen shows correct standings
- Verify: wandering drones fly normally in Results state
- Verify: RACE AGAIN button works (fresh script + race)
