# Plan: Gate Validation Overhaul

## Problem Statement

Drones sometimes fly near a gate without triggering a pass, and then don't DNF. Root causes identified through code audit:

1. **Finish-line deadlock** (Critical): When `spline_t` reaches `finish_t`, `update_ai_targets` stops advancing it, but `miss_detection` uses strict `>` with the same threshold — the drone is stuck forever, never finishing and never crashing.
2. **Tunneling through thin trigger volumes**: Point-in-volume detection misses fast drones that pass through the 1.0m-deep gates (`gate_ground`, `gate_best`) between frames.
3. **Physics deviation exceeds gate bounds**: PID lag + proximity avoidance + per-drone offsets can push drones 3–5m off the spline, outside the trigger volume even though the spline passes through.
4. **No directional validation**: `GateForward` is stored but never checked — backwards passes count.

## Phase 1: Fix Finish-Line Deadlock (P0 — one-line fix)

**File:** `src/drone/ai.rs`

**Change:** In `update_ai_targets`, add a small epsilon to the guard so that `spline_t` can advance slightly past `finish_t`, allowing `miss_detection`'s strict `>` to fire.

```rust
// BEFORE (line 98):
if ai.spline_t >= finish_t {

// AFTER:
const FINISH_EPSILON: f32 = 0.01;
if ai.spline_t >= finish_t + FINISH_EPSILON {
```

This ensures that when a drone misses the finish gate, `spline_t` continues to `finish_t + FINISH_EPSILON` (which is > `miss_threshold`), so `miss_detection` fires and crashes the drone.

The `compute_racing_line` early-exit guard at line 234 should use the same epsilon:
```rust
// BEFORE (line 234):
if ai.spline_t >= finish_t {

// AFTER:
if ai.spline_t >= finish_t + FINISH_EPSILON {
```

**Test:** Run a race, use dev dashboard (F4) to observe. If any drone previously got stuck hovering near the finish, it should now DNF with an explosion.

## Phase 2: Plane-Crossing Gate Detection (P1 — architectural fix)

Replace the point-in-volume check with a plane-crossing test that uses the drone's previous position. This eliminates tunneling at any speed, validates approach direction, and tolerates larger path deviations.

### 2a: Add `PreviousPosition` tracking to gate check

**File:** `src/drone/components.rs`

The `PreviousTranslation` component already exists in `src/drone/interpolation.rs` (used for camera smoothing) and is updated every `FixedPreUpdate`. Reuse it directly — no new component needed.

### 2b: Build a `GatePlanes` resource at race start

**File:** `src/race/gate.rs` (new resource + builder system)

Create a resource that caches each gate's plane data at race start, avoiding per-frame ECS queries:

```rust
pub struct GatePlane {
    pub gate_index: u32,
    /// Gate center in world space (trigger volume center, not obstacle origin).
    pub center: Vec3,
    /// Gate forward (plane normal) — approach direction.
    pub normal: Vec3,
    /// Gate's local right axis (for bounded plane test).
    pub right: Vec3,
    /// Gate's local up axis.
    pub up: Vec3,
    /// Half-width and half-height of the gate opening.
    pub half_width: f32,
    pub half_height: f32,
}

#[derive(Resource)]
pub struct GatePlanes(pub Vec<GatePlane>);
```

Build this in a system that runs after course obstacles are spawned (add to `OnEnter(Race)` or poll until gates exist, similar to `spawn_drones`). Query `(GateIndex, GateForward, GlobalTransform)` on gate parent entities and `(TriggerVolume, GlobalTransform, ChildOf)` on trigger children to extract center, normal, extents, and local axes.

Clean up `OnExit(Race)` (remove the resource).

### 2c: Rewrite `gate_trigger_check` to use plane-crossing

**File:** `src/race/gate.rs`

New algorithm for each racing drone:

```rust
pub fn gate_trigger_check(
    mut progress: Option<ResMut<RaceProgress>>,
    clock: Option<Res<RaceClock>>,
    gate_planes: Option<Res<GatePlanes>>,
    drone_query: Query<(&Drone, &Transform, &PreviousTranslation, &DronePhase)>,
) {
    // ... existing guards ...

    for (drone, transform, prev_pos, phase) in &drone_query {
        // ... existing phase/active guards ...

        let expected_gate = progress.drone_states[drone_idx].next_gate;
        let target_gate_index = if is_finish_pass { 0 } else { expected_gate };

        // Find the gate plane for target_gate_index
        let Some(plane) = gate_planes.0.iter().find(|p| p.gate_index == target_gate_index) else {
            continue;
        };

        // Signed distances to the gate plane
        let d_prev = (prev_pos.0 - plane.center).dot(plane.normal);
        let d_curr = (transform.translation - plane.center).dot(plane.normal);

        // Must cross from front (positive) to back (negative or zero)
        if d_prev <= 0.0 || d_curr > 0.0 {
            continue;
        }

        // Interpolate to find crossing point
        let t = d_prev / (d_prev - d_curr);
        let crossing = prev_pos.0 + t * (transform.translation - prev_pos.0);

        // Check crossing point is within gate bounds
        let offset = crossing - plane.center;
        let x = offset.dot(plane.right).abs();
        let y = offset.dot(plane.up).abs();

        if x < plane.half_width && y < plane.half_height {
            if is_finish_pass {
                progress.record_finish(drone_idx, clock.elapsed);
            } else {
                progress.record_gate_pass(drone_idx, expected_gate);
            }
        }
    }
}
```

**Key properties:**
- **No tunneling**: Tests the line segment between two positions, not a point. Works at any speed.
- **Directional**: Only counts front-to-back crossings (`d_prev > 0, d_curr <= 0`). Backwards passes are rejected.
- **Cheaper**: Two dot products for plane test + two for bounds check vs. full affine inverse + 3-axis AABB.
- **More tolerant of deviation**: The gate's full 2D opening is the detection zone, not a thin volume. A drone 2m off the spline center still passes as long as it crosses the plane within bounds.

### 2d: Keep `point_in_trigger_volume` and existing tests

Don't delete the old function — it's well-tested and may be useful elsewhere (e.g., editor trigger volume visualization). Just stop using it in `gate_trigger_check`.

### 2e: Keep `miss_detection` as safety net

The spline-parameter-based miss detection remains unchanged. It catches edge cases where a drone flies completely around a gate (never crosses the plane at all). The plane-crossing approach handles the common case; miss_detection handles the degenerate case.

### 2f: Update the drone query in `gate_trigger_check`

Add `PreviousTranslation` to the query. Since `PreviousTranslation` is already spawned on all drones (by `spawning.rs`) and updated in `FixedPreUpdate`, no spawning changes are needed.

Verify that `PreviousTranslation` is populated before the first `gate_trigger_check` runs. Since drones start in `DronePhase::Idle` and the race clock isn't running yet, the gate check returns early until the countdown finishes — by which time multiple FixedUpdate ticks have run and `PreviousTranslation` is valid.

## Phase 3: Reduce Path Deviation at Gates (P1 — tuning fix)

The plane-crossing detection is more tolerant than point-in-volume, but drones still need to physically cross through the gate opening. Combined deviation sources can push drones 3–5m off the spline, which exceeds the half-width of small gates like `gate_best` (4.2m half-width, minus a 2.5m per-drone offset = only 1.7m margin).

### 3a: Cap `gate_pass_offset` range

**File:** `src/drone/spawning.rs`

Find where `gate_pass_offset` is assigned per drone (in the DroneConfig generation). Reduce the maximum from 0.6 to 0.4:

```rust
// BEFORE: gate_pass_offset in range 0.2–0.6
// AFTER:  gate_pass_offset in range 0.15–0.4
```

With 0.4 × 4.2m (`gate_best` half-width) = 1.68m max offset, leaving 2.5m margin for PID lag and avoidance. This is enough for normal racing conditions.

### 3b: Suppress proximity avoidance near gates

**File:** `src/drone/ai.rs` — `proximity_avoidance` system

When a drone is close to its next gate, reduce the avoidance offset so drones don't get pushed out of the gate opening at the critical moment. Add a gate-proximity attenuation:

```rust
// After computing total_offset, before applying:
// Check distance to next gate — suppress avoidance when close
let gate_dist = /* distance to ai.gate_positions[target_gate_index] */;
const GATE_SUPPRESS_RADIUS: f32 = 10.0;
const GATE_SUPPRESS_MIN: f32 = 0.2; // reduce avoidance to 20% at gate
if gate_dist < GATE_SUPPRESS_RADIUS {
    let suppress = GATE_SUPPRESS_MIN + (1.0 - GATE_SUPPRESS_MIN) * (gate_dist / GATE_SUPPRESS_RADIUS);
    total_offset *= suppress;
}
```

This fades avoidance from 100% at 10m away to 20% at the gate itself. Drones still avoid each other in the midleg segments but don't push each other out of gates.

**Note:** This requires passing `AIController` into the `proximity_avoidance` query. Since the query already has `&Transform, &Drone, &DroneDynamics, &DronePhase, &mut DesiredPosition`, adding `&AIController` is straightforward.

### 3c: Add bounds margin to plane-crossing check

**File:** `src/race/gate.rs` — in the rewritten `gate_trigger_check`

Add a small margin multiplier to the bounds check so drones that graze the edge still count:

```rust
// Instead of exact bounds:
// if x < plane.half_width && y < plane.half_height {

// Use a 10% margin:
const GATE_PASS_MARGIN: f32 = 1.1;
if x < plane.half_width * GATE_PASS_MARGIN && y < plane.half_height * GATE_PASS_MARGIN {
```

This is a "player-friendly" tolerance that forgives edge-grazing passes. 10% on `gate_best` (4.2m half-width) adds 0.42m of forgiveness — invisible to the viewer but catches borderline cases.

## Phase 4: Directional Gate Validation (P2 — included in plane-crossing but needs explicit testing)

The plane-crossing algorithm in Phase 2c already enforces direction via the signed-distance check:

```rust
// d_prev > 0  → drone was in front of gate (approach side)
// d_curr <= 0 → drone is now behind gate (departure side)
// Together: front-to-back crossing only
if d_prev <= 0.0 || d_curr > 0.0 {
    continue; // reject: not a valid front-to-back crossing
}
```

`plane.normal` is set from `GateForward`, which already accounts for `gate_forward_flipped`. No additional code is needed, but this behavior must be explicitly tested.

### 4a: Add directional unit tests

**File:** `src/race/gate.rs` — `#[cfg(test)] mod tests`

Add a pure-function helper for testability (extract the crossing logic from the system):

```rust
pub fn plane_crossing_check(
    prev_pos: Vec3,
    curr_pos: Vec3,
    plane: &GatePlane,
    margin: f32,
) -> bool {
    let d_prev = (prev_pos - plane.center).dot(plane.normal);
    let d_curr = (curr_pos - plane.center).dot(plane.normal);
    if d_prev <= 0.0 || d_curr > 0.0 {
        return false;
    }
    let t = d_prev / (d_prev - d_curr);
    let crossing = prev_pos + t * (curr_pos - prev_pos);
    let offset = crossing - plane.center;
    let x = offset.dot(plane.right).abs();
    let y = offset.dot(plane.up).abs();
    x < plane.half_width * margin && y < plane.half_height * margin
}
```

Tests:
- Front-to-back, within bounds → `true`
- Back-to-front (reversed direction) → `false`
- Front-to-back, outside bounds horizontally → `false`
- Front-to-back, outside bounds vertically → `false`
- Both positions on same side (no crossing) → `false`
- Crossing at edge with margin=1.0 → `false`, with margin=1.1 → `true`
- Rotated gate (90° around Y) — verify axes are correct → `true`
- Gate with `gate_forward_flipped` — approach from opposite side → `true`

### 4b: Remove `#[allow(dead_code)]` from `GateForward`

**File:** `src/race/gate.rs`

`GateForward` is now used (to build `GatePlanes`), so the dead_code allow can be removed.

## Phase 5: Increase Trigger Volume Depth (P2 — data fix, defense-in-depth)

**File:** `assets/library/default.obstacles.ron`

Increase the Z half-extent of the two thin gates as a defense-in-depth measure. Even with plane-crossing detection, thicker volumes improve the `miss_detection` safety net (a thicker volume means `point_in_trigger_volume` can still serve as a secondary check if ever needed):

```
gate_ground: half_extents z: 0.5 → 1.5
gate_best:   half_extents z: 0.5 → 1.5
```

This makes all gates at least 3.0m deep (matching `gate_loop`'s ~1.9m). At 55 m/s max speed and 64Hz FixedUpdate, per-tick displacement is 0.86m — well within a 3.0m volume.

**Note:** This is purely a data change. It does not affect the visual model — trigger volumes are invisible at runtime. However, verify in the editor that the trigger volume gizmo (if drawn) still looks reasonable.

## Phase 6: Post-Implementation Checklist

### Build & Test
- [ ] `cargo build` — fix any new warnings
- [ ] `cargo test` — all existing tests pass
- [ ] Add plane-crossing unit tests (see Phase 4a for full list — 8 cases including direction, bounds, margin, rotation, flip)

### Manual Testing

| # | Test | Expected | Pass/Fail/Notes |
|---|------|----------|-----------------|
| 1 | Run a race with `gate_ground` gates, watch all 12 drones finish | All drones either finish or DNF cleanly (no stuck hovering) | |
| 2 | Run a race with `gate_best` gates at high speed (increase max_speed via F4 dashboard) | No drones pass through gates without detection | |
| 3 | Check that DNF explosions still trigger on genuine gate misses | Drone explodes at the missed gate location | |
| 4 | Run 5+ races back-to-back, check no drone ever gets stuck | Race always completes within ~90s | |
| 5 | Check the finish line specifically — drones that miss gate 0 on the final pass should DNF | Explosion + "CRASHED — missed gate" log message | |
| 6 | Toggle debug draw (F3) and verify spline paths still look correct | Colored spline lines pass through gates | |
| 7 | Check the editor flight spline preview still works | Green/red curvature visualization unchanged | |
| 8 | Load an existing course (backward compat) | Course loads without errors | |
| 9 | Performance: verify stable 60fps with 12 drones | No frame drops from the new detection logic | |
| 10 | Directional validation: in editor, flip a gate (F key), run race — drones should approach from the correct side | Drones fly through in the gate's forward direction | |
| 11 | Directional validation: manually confirm no backwards passes register in logs | No "passed gate" log for wrong-direction crossings | |
| 12 | Path deviation: enable avoidance (F4 → Avoid Strength 12.0), run race on tight course with `gate_best` gates | Drones still register gate passes despite avoidance pushing them around | |
| 13 | Gate suppression: watch pack of drones approaching a gate — they should tighten up near the gate and spread out between gates | Visible tightening of formation near gates | |

### Documentation Updates
- [ ] Update `CLAUDE.md` — replace the race validation pattern section to reflect plane-crossing detection, `GatePlanes` resource, and the `PreviousTranslation` dependency
- [ ] Update `ARCHITECTURE.md` if it describes gate validation

## File Change Summary

| File | Phase | Changes |
|------|-------|---------|
| `src/drone/ai.rs` | 1 | Add `FINISH_EPSILON` to the `spline_t >= finish_t` guards (2 locations) |
| `src/race/gate.rs` | 2, 3c, 4 | Add `GatePlane`, `GatePlanes` resource, builder system, `plane_crossing_check` helper; rewrite `gate_trigger_check` with plane-crossing + margin; remove `#[allow(dead_code)]` from `GateForward`; add 8 unit tests. Keep `point_in_trigger_volume` and `miss_detection` unchanged |
| `src/race/mod.rs` | 2 | Register `GatePlanes` builder system and cleanup |
| `src/drone/spawning.rs` | 3a | Reduce `gate_pass_offset` range from 0.2–0.6 to 0.15–0.4 |
| `src/drone/ai.rs` | 3b | Add gate-proximity attenuation to `proximity_avoidance`; add `&AIController` to query |
| `assets/library/default.obstacles.ron` | 5 | Increase `gate_ground` and `gate_best` Z half-extents from 0.5 to 1.5 |

## Performance Notes

- `GatePlanes` resource is built once at race start — zero per-frame cost for construction
- Plane-crossing test: 4 dot products + 1 lerp per drone per frame = ~20 FLOPs x 12 drones = 240 FLOPs total. Cheaper than the current inverse-affine approach
- Gate-proximity attenuation in `proximity_avoidance`: 1 extra distance calculation per drone per FixedUpdate tick. Negligible
- `miss_detection` unchanged — still O(12) per frame
- No new per-frame allocations
