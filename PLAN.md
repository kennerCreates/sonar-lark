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
- Wandering behavior (physics chain active for non-Racing drones during Race state and all drones during Results)
- Pilot skill → DroneConfig mapping
- Race countdown flow (START RACE → convergence → 3-2-1 → GO)

---

## Architecture Overview

```
OnEnter(Race)
  │
  ├── spawn_drones (existing — entities + per-drone splines + visuals)
  ├── Drones wander near start area (existing physics/wander, collision suppressed)
  │
  ▼ Player presses START RACE → convergence window (~3-5s)
  │   Wander targets set to spline start positions, drones fly there naturally
  │
  ▼ generate_race_script (NEW — predetermines outcome from pilot data + course)
  ▼ 3-2-1 Countdown → GO → DronePhase::Racing
  │
  ▼ Racing phase: FixedUpdate each tick (both chains run, phase-filtered)
  │
  ├── advance_choreography (NEW — Racing drones: advance spline_t, write Transform)
  ├── compute_choreographed_rotation (NEW — Racing drones: bank angle + acrobatic keyframes)
  ├── apply_visual_noise (NEW — Racing drones: attitude jitter, dirty air wobble)
  ├── fire_scripted_events (NEW — gate passes, overtakes, crashes, finishes from spline_t)
  ├── wander + physics chain (existing — non-Racing drones: Idle/Wandering, collision suppressed)
  │
  ├── On finish → reset physics state → DronePhase::Wandering (immediate, per-entity)
  │
  ▼ All drones finished/crashed → RacePhase::Finished (existing lifecycle)
  │
  ▼ Results (existing — all drones already wandering, stats updated, etc.)
```

---

## Implementation Phases

| Phase | What | New/Modified Files | Risk | Status |
|-------|------|--------------------|------|--------|
| **1** | Race script generator | NEW `race/script.rs`, mod `race/mod.rs` | Low — pure data | Not started |
| **2** | Core spline playback + wandering lifecycle | NEW `drone/choreography.rs`, mod `drone/mod.rs`, physics chain phase guards | Medium — replaces physics chain for Racing drones, dual-chain scheduling | Not started |
| **3** | Scripted events + RaceEventLog | Extend `drone/choreography.rs`, NEW `race/script.rs::RaceEventLog`, mod `race/mod.rs` | Low — mirrors existing event flow | Not started |
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
    /// Per-segment pacing multiplier (one entry per gate-to-gate segment,
    /// gate_count entries total). Clamped at boundaries: first entry applies
    /// before the first gate, last entry applies after the last gate.
    /// >1.0 = faster than base curvature speed, <1.0 = slower.
    /// Varies per segment to produce overtakes from pilot strengths/weaknesses.
    pub segment_pace: Vec<f32>,
    /// If Some, drone crashes near this gate (index into gate sequence).
    pub crash: Option<CrashScript>,
    /// Gate indices where this drone performs an acrobatic maneuver.
    pub acrobatic_gates: Vec<u32>,
    /// Per-gate spline_t values where this drone is closest to each gate's
    /// world-space position. Per-drone because each drone's racing line spline
    /// is unique — the t-value at a gate differs per drone.
    pub gate_pass_t: Vec<f32>,
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

/// A position swap between two drones at a known gate.
pub struct ScriptedOvertake {
    pub gate_index: u32,
    /// Drone gaining a position.
    pub overtaker_idx: u8,
    /// Drone losing a position.
    pub overtaken_idx: u8,
}

/// Complete predetermined race outcome.
#[derive(Resource)]
pub struct RaceScript {
    pub drone_scripts: Vec<DroneScript>,
    /// Pre-computed overtake moments from the drama-pass simulation (step 7).
    /// Used by fire_scripted_events to emit overtake events at the right time.
    pub overtakes: Vec<ScriptedOvertake>,
}
```

#### Generation algorithm: `generate_race_script()`

**Inputs:** course gate data (positions, forwards), per-drone splines + configs, pilot data (SkillProfile, PersonalityTrait), RaceSeed, AiTuningParams.

**Steps:**

1. **Estimate base segment times**: For each drone, dry-run spline traversal at curvature-based speeds using the **global** `safe_speed_for_curvature(curvature, tuning)` — the same speed function for all drones, but applied to each drone's unique spline. Since racing lines differ (wider lines have gentler curvature → higher speed), base segment times vary per drone. Record **per-gate-segment** times (time to traverse each `POINTS_PER_GATE` interval). Just a loop: advance `t += speed * dt / tangent_len`, count ticks per segment. Cheap — pure math, <1ms for 12 drones. Sum segment times for per-drone `estimated_total_time`.

2. **Compute turn tightness per drone per gate**: For each drone, sample peak spline curvature near each gate (within the gate's `POINTS_PER_GATE` interval on that drone's spline). Classify from curvature magnitude: below low threshold = gentle, between low and high = medium, above high = tight. Thresholds derived from `safe_speed_for_curvature` speed bands (the curvature where speed drops below X% of max is the gentle/medium boundary, etc. — tune empirically). Store per-drone per-gate. This uses the drone's actual racing line rather than gate-to-gate angles, so a drone with a wider line through a tight gate correctly gets a lower tightness classification.

3. **Assign finish order and target times**: Sort drones by estimated_time. Add random perturbation (±5-15% inversely proportional to `skill.consistency`). Each drone's perturbed time becomes its `target_finish_time` — the time the script aims to have that drone cross the finish line. The ranking of `target_finish_time` values is the target finish order.

4. **Assign DNFs** (0-3 per race, minimum 4 finishers): Per-drone difficulty from step 2: `drone_difficulty = drone_tight_turn_count as f32 / total_gates as f32` — fraction of gates classified as tight *for this drone's racing line*, optionally boosted by elevation variance. For each drone, compute crash probability from `(1.0 - skill.level) * drone_difficulty * personality_risk_factor` where Reckless/Aggressive increase risk, Cautious/Methodical decrease. Roll against RaceSeed hash. Crashed drones get a `CrashScript` at a tight-turn gate with some randomized progress past the gate. **Hard cap**: after assigning all crashes, if fewer than 4 drones would finish, remove the least-dramatic crashes (lowest turn tightness at crash gate) until the minimum is met. Never exceed 3 DNFs total.

5. **Assign acrobatics**: For each non-DNF drone, check each gate classified as tight *for that drone's racing line* (from step 2): if `skill.cornering > 0.6` AND personality is Flashy/Hotdog/Aggressive (or skill.cornering > 0.85 for anyone), mark that gate as acrobatic for this drone. Cap at ~3-5 acrobatic gates per drone to avoid visual fatigue.

6. **Compute per-segment pace profiles**: Instead of a single `pace_factor`, compute `segment_pace: Vec<f32>` — one multiplier per gate-to-gate segment. This is what produces overtakes.

   **Pilot strength model:** Each pilot has two derived attributes:
   - **Straight-line speed factor** — from `skill.level` + personality bias. Reckless/Aggressive pilots push harder on straights (+5-15%). Cautious/Methodical pilots are conservative (-5-10%).
   - **Cornering efficiency factor** — from `cornering_aggression` + `skill.consistency`. This is the **sole place** cornering ability is expressed in the pace profile (base segment times use the same speed function for all drones — see step 1 — so per-drone variation in base times comes only from spline geometry, not from skill). Aggressive cornering pilots lose less time in turns. Inconsistent pilots have high variance (±10%) per corner, seeded from `(RaceSeed, drone_idx, gate_idx)`.

   **Per-segment pace computation:**
   ```
   for each segment i:
       if drone_turn_tightness[drone_idx][i] is gentle:
           segment_pace[i] = base_pace * straight_line_factor
       else (medium/tight — from per-drone curvature classification):
           segment_pace[i] = base_pace * cornering_efficiency * (1.0 + consistency_noise)
   ```
   Where `base_pace = estimated_total_time / target_finish_time` (per-drone — each drone's own base time divided by its target finish time from step 3).

   This naturally produces overtakes: a Reckless pilot with high straight speed but poor cornering will lead on straights, lose ground in turns, and get overtaken by a Methodical pilot who is steady everywhere. The overtake locations emerge from the interaction between pilot profiles and course geometry.

   For DNF drones, segment_pace is computed normally up to the crash segment (they fly at their natural pace until the crash).

   **Normalization pass:** After computing all per-segment paces for a drone, dry-run the full spline traversal (same cheap loop as step 1 but now using `base_speed * segment_pace[i]` per segment) to get the actual `simulated_finish_time`. Apply a uniform correction factor to all segments: `correction = target_finish_time / simulated_finish_time; for each i: segment_pace[i] *= correction`. This preserves the relative inter-segment variation (which produces overtakes) while ensuring total time matches the target. Without this, the straight-line and cornering modifiers can shift total time by 5-15% because they redistribute time between segments non-uniformly. For DNF drones, normalize only the segments before the crash point.

7. **Drama pass — photo finishes**: After computing all segment paces, simulate the full race (cheap — accumulate spline_t per drone per segment, ~12 × gate_count iterations):

   - Track position order at each gate to detect **overtakes** (any position swap between consecutive gates). Record each overtake as a `ScriptedOvertake { gate_index, overtaker_idx, overtaken_idx }` and store in `RaceScript.overtakes`. These are used by `fire_scripted_events` (Phase 3) to emit overtake events at the correct moment.
   - Check top-2 finish time gap:
     - If < 2.0s: natural close finish — leave as-is.
     - If 2.0–5.0s AND both pilots have `skill.level > 0.5`: tighten the gap by nudging the slower drone's final 2–3 segment paces up by up to 5%. Re-simulate to verify the order didn't flip (the slower drone should close the gap, not win).
     - If > 5.0s: don't force it — not every race needs a photo finish.
   - Cap top-2 drama adjustments: no segment pace nudge > 5%, no more than 3 segments adjusted per drone.
   - **Mid-pack clustering**: After handling top-2, identify isolated drones — any drone with >3s gap to both the drone ahead and behind at any gate during the simulation. Group nearby drones (gap <3s) into "battle packs" of 2-3. For each isolated drone, nudge 1-2 of its segment paces to close the gap toward the nearest pack (up to 3% per segment — less aggressive than top-2 adjustments). Re-simulate after each nudge to verify: (a) top-3 finish order unchanged, (b) the nudged drone joins a pack rather than overshooting into a different position band. Cap: no more than 4 drones nudged total across mid-pack clustering.
   - This keeps outcomes feeling natural rather than rubberbanded — top-2 nudges are ≤5%, mid-pack nudges are ≤3%, and the majority of position drama comes from the natural pace profile interactions.

#### File modifications

- **`src/race/mod.rs`** — add `pub mod script`, register generation system
- **`src/race/lifecycle.rs`** — call `generate_race_script()` during countdown start (when `tick_countdown` transitions to Racing, same place RaceProgress is created). Insert `RaceScript` resource.

#### Pre-compute per-gate spline_t offsets

During script generation, compute `gate_pass_t: Vec<f32>` **per drone** — for each drone's spline and each gate, find the `spline_t` value whose world-space position is closest to the gate's actual position. Search within `[gate_index * POINTS_PER_GATE, (gate_index + 1) * POINTS_PER_GATE]` using linear scan (a few samples per segment, cheap). Store in each `DroneScript.gate_pass_t`. Per-drone because each drone's racing line takes a different path through the gate, so the crossing t-value differs. This replaces the fixed `GATE_CENTER_OFFSET` approach, which would fire early/late for gates where the spline shape doesn't match a constant offset.

#### Reused code

- `safe_speed_for_curvature(curvature, tuning)` from `src/drone/ai/mod.rs` — used for base segment time estimation (step 1, same function applied to each drone's spline)
- `cyclic_curvature` from `src/drone/ai/mod.rs` — already a pure function
- `DroneConfig.cornering_aggression`, `.braking_distance` — read from existing configs
- Gate positions/forwards from `AIController` on spawned drones

---

## Phase 2: Core Spline Playback

### New file: `src/drone/choreography.rs`

This is the pivotal phase. Drones follow their splines visually instead of through physics.

#### Drone lifecycle during Race state

Drones wander freely (physics-based) whenever they're not actively racing. The choreography only takes over during the Racing phase. No VictoryLap — drones return to wandering after finishing.

```
OnEnter(Race)
  │
  ├── spawn_drones (existing) → DronePhase::Idle
  ├── Drones wander near start area (physics chain, collision suppressed)
  │
  ▼ Player presses START RACE
  │
  ├── Convergence window (~3-5s): wander targets set to spline start positions
  │   Drones naturally fly toward their start marks
  │
  ▼ 3-2-1 countdown
  │
  ▼ GO → DronePhase::Racing
  │   Snap to exact spline start position (clean handoff)
  │   Choreography takes over — physics chain skips Racing drones
  │
  ▼ Drone crosses finish → DronePhase::Wandering
  │   reset_physics_for_wandering() per-entity
  │   Physics chain picks it up immediately — wanders near course
  │
  ▼ All drones finished/crashed → Results
```

#### System: `advance_choreography` (FixedUpdate, chained, run_if Race state)

Only processes drones with `DronePhase::Racing` (or `Crashed` with `BallisticState` — Phase 4). All other phases (Idle, Wandering) are handled by the existing physics/wander chain.

**Racing**:
```
current_segment = (spline_t / POINTS_PER_GATE).floor() as usize
    .clamp(0, drone_script.segment_pace.len() - 1)
curvature = cyclic_curvature(spline, spline_t, cycle_t)
base_speed = safe_speed_for_curvature(curvature, tuning)
speed = base_speed * drone_script.segment_pace[current_segment]
tangent = spline.velocity(spline_t % cycle_t)
spline_t += speed * dt / tangent.length().max(0.01)
position = spline.position(spline_t % cycle_t)
```

Write to entity:
- `Transform.translation = position`
- `DroneDynamics.velocity = tangent.normalize() * speed` (for cameras)
- `ChoreographyState.previous_spline_t = old spline_t` (save BEFORE writing new value — needed by `fire_scripted_events` for crossing detection)
- `AIController.spline_t = spline_t` (for FPV look-ahead, leaderboard)

**Crashed**: Handled in Phase 4. For now, skip crashed drones.

#### System: `compute_choreographed_rotation` (FixedUpdate, after advance_choreography)

Only processes `DronePhase::Racing` drones. Derive rotation from spline curvature:
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

> **Implementation note:** The `-tangent` sign depends on the drone model's local facing axis. If the model faces -Z (Bevy convention), this should be `tangent` instead. Verify against the existing rotation code in `physics.rs` before implementing.

Write `Transform.rotation = rotation`. (Acrobatic override added in Phase 5.)

#### System scheduling changes

**`src/drone/mod.rs`:**

The current FixedUpdate chain is:
```
AI systems → physics chain (11 systems)
```

Change to: **both chains run during Race state**, separated by per-entity phase filtering.

```
ALWAYS during Race state (both chains run):
    choreography systems (.chain()): advance_choreography → compute_choreographed_rotation → (Phase 3: fire_scripted_events)
        → only processes DronePhase::Racing entities
    wander + physics chain (existing)
        → each system skips DronePhase::Racing entities (add early-continue on phase check)
        → collision suppressed (see below)

During Results state:
    wander + physics chain (all drones, collision suppressed)
```

**Per-entity filtering**: The choreography systems query all `Drone` entities but skip non-Racing drones (`if *phase != DronePhase::Racing { continue; }`). The existing physics chain systems add the same guard to skip Racing drones. This is minimal code churn — one `continue` at the top of each system's loop body.

**Collision and gate detection — Phase 2 transitional behavior**: The existing collision/gate detection systems (`gate_trigger_check`, `obstacle_collision_check`, `miss_detection`) continue running during Phase 2, but only for `DronePhase::Racing` drones — add a phase guard to skip non-Racing entities (Idle, Wandering). This keeps gate passes registering on the leaderboard and race progress tracking functional while Phase 3's scripted events don't exist yet. Wandering drones never trigger crashes.

**Phase 3+ final behavior**: Phase 3 replaces these live detection systems entirely with `fire_scripted_events`. At that point, the collision/gate systems are removed from the Race state chain (not just phase-guarded). See Phase 3 file modifications for details.

**Convergence window**: When the player presses START RACE, a new system `set_convergence_targets` sets each drone's `DesiredPosition` to its spline start position. The existing wander/physics chain naturally flies them there. The convergence window ends when **either** all drones are within 2m of their start positions, **or** a hard time cap of 5 seconds elapses — whichever comes first. The 3-2-1 countdown then begins (3 seconds of additional convergence time).

**Snap behavior**: On GO, choreography sets `Transform.translation` to the exact spline start position. Most drones will be within ~0.5m at this point (5s convergence + 3s countdown = 8s total approach time). For any drone still >1m away (e.g., stuck behind geometry), the snap is masked by the camera focusing on the race start — a brief lerp over 2-3 frames further softens it. The `PreviousTranslation` interpolation pipeline naturally smooths small snaps across the FixedUpdate→render boundary.

The interpolation pipeline (`PreviousTranslation`/`PreviousRotation`/`PhysicsTranslation`/`PhysicsRotation`) runs for both Race and Results states — unchanged. It operates generically on all `Drone` entities: `save_previous_transforms` (FixedPreUpdate) snapshots before the tick, then either choreography or physics writes `Transform` in FixedUpdate, `save_physics_transforms` (FixedPostUpdate) captures the result, and `interpolate_visual_transforms` (PostUpdate) blends for smooth rendering. Since both choreography and physics write `Transform` at the same FixedUpdate insertion point, the pipeline works identically for both. Cameras that read `PreviousTranslation` (Chase, FPV) get correct data from this pipeline automatically.

**Key component writes for camera compatibility:**

| Component | Written by choreography (Racing) | Written by physics (Wandering) | Read by |
|-----------|----------------------------------|-------------------------------|---------|
| `Transform.translation` | spline.position(t) | physics integration | Chase, FPV, leaderboard |
| `Transform.rotation` | curvature-based bank | physics attitude | Visual rendering |
| `DroneDynamics.velocity` | tangent * speed | physics integration | Chase (heading, FOV), FPV |
| `AIController.spline_t` | advanced each tick | (not written) | FPV look-ahead, sync_spline_progress |
| `AIController.target_gate_index` | advanced on gate pass | (not written) | Leaderboard display |
| `DronePhase` | transitions on events | (not written) | Camera selection, all systems |

#### Physics state reset on Racing → Wandering transition

When a drone finishes (or in future, when entering Race state if needed), `reset_physics_for_wandering` resets stale physics state per-entity:

- `DroneDynamics.velocity` — already set by choreography (tangent * speed), **keep as-is** for smooth handoff
- `DroneDynamics.angular_velocity` — stale; **zero it**
- `DroneDynamics.thrust` / `commanded_thrust` — stale; **set to `mass * GRAVITY`** (hover thrust) so the first physics tick doesn't drop the drone
- `PositionPid.integral` — stale; **zero it** to prevent integral windup
- `DesiredPosition.position` — stale; **set to current `Transform.translation`** so the position PID doesn't chase a stale target
- `DesiredAcceleration`, `DesiredAttitude` — stale; **zero / reset to hover-level defaults**

This runs **per-entity at the moment of phase transition** (not on state change), so it happens immediately when a drone finishes mid-race. The wander system picks up the drone on the next tick with clean state.

#### Results state transition

Simplified: by the time the race ends, all surviving drones are already wandering (they transitioned on finish). Crashed drones are already hidden/exploded. `OnEnter(Results)` just needs to update stats and UI — no physics reset needed because it was handled per-entity during the race.

---

## Phase 3: Scripted Events

Extend `src/drone/choreography.rs`.

#### New resource: `RaceEventLog`

```rust
/// Timestamped race event for post-race highlight reel.
pub enum RaceEventKind {
    GatePass { drone_idx: u8, gate_index: u32 },
    Overtake { overtaker_idx: u8, overtaken_idx: u8, gate_index: u32 },
    Acrobatic { drone_idx: u8, gate_index: u32 },
    Crash { drone_idx: u8, crash_type: ScriptedCrashType },
    Finish { drone_idx: u8, time: f32 },
}

pub struct TimestampedEvent {
    pub race_time: f32,
    pub kind: RaceEventKind,
}

#[derive(Resource, Default)]
pub struct RaceEventLog {
    pub events: Vec<TimestampedEvent>,
}
```

Inserted alongside `RaceScript` in `lifecycle.rs`. Every event emitted by `fire_scripted_events` is also pushed to this log. The Hype Phase (future) reads this log to present highlight candidates to the player.

#### System: `fire_scripted_events` (FixedUpdate, after advance_choreography)

Replace the current gate_trigger_check + obstacle_collision_check + miss_detection chain.

For each Racing drone, check spline_t against thresholds:

**Gate pass**: When `spline_t` crosses `drone_script.gate_pass_t[gate_index]` (pre-computed per-drone per-gate offsets from Phase 1, not a fixed constant). Specifically, `choreography_state.previous_spline_t < threshold AND AIController.spline_t >= threshold`.
- Call `progress.record_gate_pass(drone_idx, gate_idx)`
- Advance `AIController.target_gate_index`
- Push `RaceEventKind::GatePass` to `RaceEventLog`

**Overtake**: Check `RaceScript.overtakes` — when the **overtaker** drone's `spline_t` crosses `drone_script.gate_pass_t[overtake.gate_index]`, emit the event (this is the moment the overtaker reaches the gate where the position swap occurs):
- Push `RaceEventKind::Overtake` to `RaceEventLog`
- (Future: trigger camera/audio cue)

**Finish**: When `spline_t >= gate_count * POINTS_PER_GATE + FINISH_EXTENSION + FINISH_EPSILON` (where `FINISH_EPSILON = 0.01` — matches existing AI finish detection to avoid floating-point edge cases):
- Call `progress.record_finish(drone_idx, race_clock.elapsed)`
- Call `reset_physics_for_wandering()` on this entity (clean physics state for handoff)
- Set `DronePhase::Wandering` (physics/wander chain picks up immediately — no VictoryLap)
- Push `RaceEventKind::Finish` to `RaceEventLog`

**Race complete**: When all drones finished or crashed → existing `check_race_finished` handles this (reads RaceProgress, unchanged).

#### File modifications

- **`src/race/mod.rs`** — remove `gate_trigger_check`, `obstacle_collision_check`, `miss_detection` from the Update chain during Race state. These systems no longer needed (events come from script). Keep `sync_spline_progress` and `check_race_finished` in Update (they read RaceProgress which the scripted events update).
- **`src/race/lifecycle.rs`** — insert `RaceEventLog::default()` alongside `RaceScript`. Clean up on race exit.
- Note: `build_gate_planes` and `build_obstacle_collision_cache` can be skipped (not needed without live detection). But keeping them is harmless and they're gated by `not(resource_exists)`.

---

## Phase 4: Scripted Crashes

Both obstacle/gate crashes and drone-on-drone collisions are implemented from the start.

#### Script generation (in `generate_race_script()`)

Crash assignment extends the script generation pipeline. Obstacle crashes are assigned in step 4 (existing). Drone-on-drone collisions are a **new step 8**, running **after** the drama pass (step 7) so that pacing is finalized before proximity detection. This avoids drama-pass pace nudges invalidating collision positions.

**Obstacle crashes** (step 4): Assigned to drones with high crash probability at tight-turn gates. The crash point is `drone_script.gate_pass_t[gate_index] + progress_past_gate * POINTS_PER_GATE` — partway through the turn, relative to the drone's actual gate crossing position.

**Drone-on-drone crashes** (step 8, after drama pass): Using the final simulated race from step 7 (with all pace adjustments applied), identify proximity windows — pairs of drones that occupy the same gate segment at the same simulated real-time. Check world-space proximity by sampling both drones' splines at their respective spline_t values for that time step. If positions are within ~3m, this is a collision candidate. Assign `DroneCollision` to one or both drones at that spline_t. The `other_drone_idx` field links paired crashes so both trigger at the same tick.

**Minimum finishers re-check** (end of step 8): After assigning all drone-on-drone crashes, re-verify the minimum finishers constraint (≥4 finishers). A `DroneCollision` that takes out 2 drones simultaneously could push the total below the cap. If the constraint is violated, remove drone-on-drone crashes starting from the least-dramatic pair (lowest combined turn tightness at crash point) until ≥4 finishers are restored. The step 4 obstacle crash cap (≤3 DNFs) counts only obstacle crashes; this re-check counts all DNFs including drone collisions.

#### New components (in `src/drone/components.rs`)

```rust
/// Per-entity choreography tracking, inserted on DronePhase::Racing transition.
#[derive(Component)]
pub struct ChoreographyState {
    /// spline_t from the previous tick — needed by fire_scripted_events
    /// for gate crossing detection (previous_t < threshold <= current_t).
    pub previous_spline_t: f32,
}

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
5. Push `RaceEventKind::Crash` to `RaceEventLog`

**Drone-on-drone crash**: When a drone's `spline_t` reaches a `DroneCollision` crash point:
1. Find the paired drone (from `other_drone_idx`)
2. If the paired drone already has `BallisticState` (processed earlier this tick or a prior tick), skip — the collision was already handled from the other side
3. Compute collision normal between the two drones' positions
4. Apply diverging velocities: each drone gets a velocity kick away from the other (±3-5 m/s lateral + upward component)
5. Insert `BallisticState` on both drones (if both are scripted to crash) or just the one that loses the encounter — check the paired drone's `CrashScript` to decide (asymmetric case: only the weaker drone DNFs from a clip, the other continues racing)
6. Fire crash events for each affected drone
7. Push `RaceEventKind::Crash` to `RaceEventLog` for each crashed drone

**Ballistic update** (in `advance_choreography` for drones with `BallisticState`):
```
velocity.y -= GRAVITY * dt
position += velocity * dt
if position.y <= GROUND_HEIGHT:
    call crash_drone()  // explosion, hide, zero velocity, update RaceProgress
    remove BallisticState
```

This produces a visible arc + explosion rather than an instant disappearance. For drone-on-drone crashes, both drones tumble away from each other before hitting the ground.

#### DnfReason extension

`DnfReason` in `src/race/progress.rs` currently has `MissedGate(u32)` and `ObstacleCollision`. Add a `DroneCollision` variant for drone-on-drone crashes. Update any downstream code that matches on `DnfReason` (results UI, stats display).

#### crash_drone() reuse

The existing `crash_drone()` in `src/race/collision.rs` handles everything: set phase, zero velocity, hide, spawn explosion, play sound, update RaceProgress. Call it exactly as the current collision system does.

---

## Phase 5: Acrobatic Maneuvers (Rotation + Position Offset)

Extend `compute_choreographed_rotation` and `advance_choreography` in `src/drone/choreography.rs`.

Acrobatics affect both **rotation** (the flip) and **position** (altitude dip/climb). In real drone racing, a Split-S causes a visible altitude dip because thrust points away from the ground while inverted, and gravity pulls the drone down. A power loop causes a visible climb. Without the position offset, flips look like a rotating sprite on rails.

#### Acrobatic window

For each gate marked in `DroneScript.acrobatic_gates`:
- Entry: `drone_script.gate_pass_t[gate_index] - ACRO_ENTRY_OFFSET` (e.g., 0.4 * POINTS_PER_GATE = 1.2 parameter units before gate)
- Exit: `drone_script.gate_pass_t[gate_index] + ACRO_EXIT_OFFSET` (e.g., 0.4 * POINTS_PER_GATE after gate)

Within this window, apply both rotation keyframes and position offset. On entry (when `spline_t` first crosses `entry_t`), push `RaceEventKind::Acrobatic` to `RaceEventLog`.

#### Position offset (in `advance_choreography`)

Applied on top of the spline position during the acrobatic window:

**Split-S** (tight turn, next gate at same or lower altitude):
```
t_local = (spline_t - entry_t) / (exit_t - entry_t)
dip = -dip_amount * sin(t_local * PI)
position = spline.position(spline_t % cycle_t) + Vec3::Y * dip
```

**Power loop** (tight turn, next gate is higher):
```
climb = climb_amount * sin(t_local * PI)
position = spline.position(spline_t % cycle_t) + Vec3::Y * climb
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

The velocity written to `DroneDynamics.velocity` should include the offset derivative so cameras track smoothly. `dt_local_per_sec` is the rate at which `t_local` changes per real second — i.e., how fast the drone traverses the acrobatic window:
```
// How fast spline_t advances per second at current speed:
spline_t_per_sec = speed / tangent.length().max(0.01)
// How fast t_local (0..1) changes per second:
dt_local_per_sec = spline_t_per_sec / (exit_t - entry_t)

velocity = tangent.normalize() * speed
    + Vec3::Y * (-dip_amount * PI * cos(t_local * PI) * dt_local_per_sec)
    + lateral_dir * (drift_amount * PI * cos(t_local * PI) * dt_local_per_sec * roll_sign)  // if lateral offset is applied
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
- Medium curvature turn (per-drone tightness from step 2) with high skill → **Aggressive bank** (no flip, just bank past 90° with a small dip). Uses simpler keyframes: just deeper bank angle, ~1m altitude offset.

#### Variation per drone

- **Roll direction**: from `DroneConfig.racing_line_bias` sign (positive = roll right, negative = roll left)
- **Smoothness**: high skill = clean slerp. Low skill = add small random rotation noise to intermediate keyframes (±5-10° wobble)
- **Aggressiveness**: high `cornering_aggression` = deeper bank on entry/exit, larger dip/climb amount
- **Dip magnitude**: scales with speed (fast drones dip more — they carry more momentum through the maneuver)

#### Blend zones

~0.05-0.10 in `t_local` space at entry and exit (~5-10% of the acrobatic window): slerp between normal bank rotation and acrobatic rotation using smoothstep. Position offset uses `sin(t_local * PI)` which naturally starts and ends at zero, so no position blending needed.

---

## Phase 6: Visual Polish

Extend `src/drone/choreography.rs`.

#### System: `apply_visual_noise` (FixedUpdate, after compute_choreographed_rotation)

**Attitude jitter**: Small-amplitude rotation noise simulating flight controller hunting. Uses elapsed game time (seconds), not `spline_t`, so jitter is continuous and doesn't speed up/slow down with the drone.
```
t = race_clock.elapsed_secs  // wall-clock time, not spline position
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
| **NEW `src/race/script.rs`** | RaceScript, DroneScript, ScriptedOvertake, RaceEventLog, generate_race_script(), estimate_lap_time() |
| **NEW `src/drone/choreography.rs`** | advance_choreography, compute_choreographed_rotation, fire_scripted_events, apply_visual_noise, reset_physics_for_wandering, set_convergence_targets |
| `src/race/mod.rs` | Add `pub mod script`. Remove gate/collision/miss detection from Race chain. Keep sync_spline_progress + check_race_finished. Register script generation system. |
| `src/drone/mod.rs` | Add `pub mod choreography`. Register choreography systems for Race state. Existing AI+physics chain runs in **both** Race and Results states but with per-entity phase filtering (skip `DronePhase::Racing`). Keep interpolation for both states. |
| `src/race/lifecycle.rs` | Insert `RaceScript` and `RaceEventLog` resources alongside `RaceProgress` when race starts. Clean up both on race exit. |
| `src/drone/components.rs` | Add `ChoreographyState` component (previous_spline_t for crossing detection). Add `BallisticState` component. |
| `src/race/progress.rs` | Add `DnfReason::DroneCollision` variant. |
| `src/drone/physics.rs` (+ other physics chain systems) | Add `DronePhase::Racing` early-continue guard to skip choreography-controlled drones. |
| `src/race/collision.rs` | Add phase guard — collision detection only for `DronePhase::Racing` drones (wandering drones don't trigger crashes). |

## Files explicitly NOT changed

- `src/camera/chase.rs` — reads Transform, DroneDynamics.velocity, DronePhase, RaceProgress (all provided by choreography)
- `src/camera/fpv.rs` — reads Transform, DroneDynamics.velocity, AIController.spline/spline_t (all provided)
- `src/race/leaderboard.rs` — reads RaceProgress (updated by fire_scripted_events)
- `src/race/progress.rs` — RaceProgress resource unchanged
- `src/drone/explosion.rs` — spawn_explosion() called by crash_drone() unchanged
- `src/drone/fireworks.rs` — detects finish from RaceProgress unchanged
- `src/drone/interpolation.rs` — interpolation pipeline unchanged
- `src/drone/wander.rs` — wandering unchanged (physics chain active for non-Racing drones during Race state and all drones during Results)
- `src/results/` — reads RaceResults unchanged
- `src/pilot/` — skill/personality system unchanged
- `src/drone/paths/` — spline generation unchanged

---

## Verification

### Per-phase

**Phase 1**: Unit tests for script generation — finish order respects skill ranking (statistically), crash count within expected range (0-3, minimum 4 finishers), acrobatic gates only on tight turns. Verify segment pace profiles produce at least 1-2 overtakes on courses with mixed turn tightness. Verify mid-pack clustering creates battle packs (2-3 drones within 3s of each other) without disrupting top-3 finish order. Verify `RaceScript.overtakes` is populated with correct gate indices. Verify drama pass tightens close finishes without flipping finish order. Verify base segment times use the same speed function across drones (per-drone variation comes from spline geometry, not from skill-based speed adjustments). Verify per-drone `gate_pass_t` values correspond to actual gate world-space positions for each drone's racing line.

**Phase 2**: Drones wander near start area pre-race (no crashes). On START RACE, drones converge to start positions. On GO, drones visibly follow their splines, banking into turns. Finished drones transition to wandering immediately (no VictoryLap). Cameras track correctly. No physics artifacts. Leaderboard shows progress via sync_spline_progress. No jerk/glitch on Racing → Wandering transition.

**Phase 3**: Gates register at correct visual positions (per-gate offsets, not fixed offset). Finishers appear on leaderboard with times. Fireworks trigger on first finish. Race ends and transitions to Results. `RaceEventLog` contains all gate passes, overtakes, and finishes with correct timestamps.

**Phase 4**: Some drones crash during tight turns — visible ballistic arc then explosion. RaceProgress shows DNFs. Results screen shows correct crash count.

**Phase 5**: Skilled drones flip at tight turns. Unskilled drones bank normally. Entry/exit blending is smooth (no rotation pop).

**Phase 6**: Drones have visible attitude jitter. Drones in close proximity wobble more. Micro-drift prevents robotic look.

### End-to-end

- `cargo build && cargo clippy && cargo test`
- Play 5+ races on courses with varying turn tightness
- Verify: finish order roughly correlates with pilot skill
- Verify: overtakes occur mid-race (position swaps visible on leaderboard and in camera views)
- Verify: close finishes happen occasionally (top-2 gap < 2s in some races)
- Verify: cameras (Chase, FPV, Spectator, CourseCamera) all work
- Verify: crashes look dramatic, not instant-disappear
- Verify: acrobatic flips look smooth at tight turns
- Verify: Results screen shows correct standings
- Verify: wandering drones fly normally in Results state
- Verify: RACE AGAIN button works (fresh script + race)
- Verify: drones wander naturally before and after racing (no crashes during wandering)
- Verify: convergence window brings drones to start positions before countdown
- Verify: RaceEventLog contains events usable for future highlight reel (gate passes, overtakes, acrobatics, crashes, finishes)
