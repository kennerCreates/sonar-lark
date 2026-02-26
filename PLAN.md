# Drone-Obstacle Collision Detection

## Context

Drones currently pass through the solid geometry of gate frames without consequence. The only crash mechanism is `miss_detection` (spline-based: DNFs drones that advance past a gate without triggering a plane-crossing event). Physical collision with obstacle geometry does not exist. This plan adds swept collision detection against obstacle bounding volumes, with gate openings exempted via the existing trigger volume data.

## Approach: Swept Segment vs OBB + Trigger Volume Exemption

Each obstacle type defines a `collision_volume` (local-space AABB) in the RON library, editable in the Obstacle Workshop. At race time, this becomes a world-space OBB (oriented bounding box) after applying instance rotation/scale. Each frame, the drone's swept segment (`PreviousTranslation` → current `Transform`) is tested against all obstacle OBBs. For gates, the trigger volume defines the flyable opening — if the hit point is within the opening's width/height (treated as an infinite-depth tube), the collision is exempted. Collisions cause instant crash (DNF), identical to `miss_detection`.

---

## Step 1 — Data Structures

**`src/obstacle/definition.rs`**:
- Add `CollisionVolumeConfig { offset: Vec3, half_extents: Vec3 }` struct (Serialize/Deserialize)
- Add `#[serde(default)] pub collision_volume: Option<CollisionVolumeConfig>` to `ObstacleDef`

**`src/obstacle/spawning.rs`**:
- Add `ObstacleCollisionVolume` component: `{ offset: Vec3, half_extents: Vec3, is_gate: bool }`
- Add `collision_config: Option<&CollisionVolumeConfig>` param to `spawn_obstacle()`
- Insert `ObstacleCollisionVolume` on the parent entity when present

**`src/course/loader.rs`** + **`src/editor/course_editor/ui.rs`** (2 call sites):
- Pass `def.collision_volume.as_ref()` to `spawn_obstacle()`

**`src/race/progress.rs`**:
- Add `ObstacleCollision` variant to `DnfReason`

---

## Step 2 — Collision Cache (`src/race/collision.rs`, new file)

Built once at race start (polling system, like `build_gate_planes`):

```
ObstacleObb {
    center: Vec3,           // world-space center
    axes: [Vec3; 3],        // world-space axes (from rotation)
    half_extents: Vec3,     // scaled by instance scale (NOT expanded by drone radius)
    gate_opening: Option<GateOpening>,
}

GateOpening {
    center: Vec3,           // trigger volume world-space center
    right: Vec3, up: Vec3,  // gate axes
    half_width: f32,        // trigger half-extents.x * scale
    half_height: f32,       // trigger half-extents.y * scale
}

ObstacleCollisionCache(Vec<ObstacleObb>)   // Resource
```

Queries: `ObstacleCollisionVolume` + `GlobalTransform` on obstacle entities. For gates, also finds child `TriggerVolume` + `GateForward` (same pattern as `build_gate_planes` at `src/race/gate.rs:75-124`).

---

## Step 3 — Pure Collision Functions (`src/race/collision.rs`)

**`segment_obb_intersection(p0, p1, obb, expansion) -> Option<Vec3>`**
- Slab method: project segment onto each OBB axis, find overlap interval
- `expansion` param (drone radius) added to half-extents at test time (not baked into cache)
- Returns first hit point on OBB surface, or `None`

**`point_in_gate_opening(point, opening) -> bool`**
- Projects point onto opening's right/up axes
- Returns true if `|x| < half_width && |y| < half_height`
- Ignores depth (infinite tube) — handles all approach angles correctly

Both are pure functions with unit tests.

---

## Step 4 — Collision Detection System

**`obstacle_collision_check`** — runs in `Update`, inserted into the race logic chain between `gate_trigger_check` and `miss_detection`:

```
tick_countdown → tick_race_clock → gate_trigger_check →
  obstacle_collision_check → miss_detection → sync_spline_progress → check_race_finished
```

Logic:
- For each `Racing | VictoryLap` drone, test swept segment vs each OBB
- If hit on a gate: check `point_in_gate_opening` — if inside opening, skip (safe pass)
- On collision: crash drone (shared helper)
- Break after first collision per drone

**Drone collision radius**: `const DRONE_COLLISION_RADIUS: f32 = 0.35` passed as `expansion` to the slab test. The OBB is effectively expanded by this radius, while the gate opening is NOT — so drones must clear the frame by at least their radius.

---

## Step 5 — Crash Logic Extraction

Extract shared `crash_drone()` helper from `miss_detection` (`src/race/gate.rs:204-248`):

```rust
pub fn crash_drone(
    commands, phase, dynamics, visibility,
    drone_index, position, crash_velocity,
    progress: Option<&mut RaceProgress>,
    explosion_meshes, materials, explosion_sounds,
    reason: DnfReason,
)
```

Sets `DronePhase::Crashed`, zeros velocity/angular_velocity, `Visibility::Hidden`, calls `record_crash`, spawns explosion. Both `miss_detection` and `obstacle_collision_check` call this helper.

For `VictoryLap` drones: `record_crash` no-ops (drone already finished), but visual crash still fires.

---

## Step 6 — Workshop UI for Collision Volumes

**`src/editor/workshop/mod.rs`** — Add to `WorkshopState`:
- `has_collision: bool`
- `collision_offset: Vec3` (model-relative, like `trigger_offset`)
- `collision_half_extents: Vec3`

Add `EditTarget::Collision` variant (existing: `Model | Trigger`).

**`src/editor/workshop/ui.rs`**:
- Add "Collision Volume" toggle button (like the trigger volume ON/OFF toggle)
- Reuse existing move widget (axis arrows) and resize widget (cube handles) — they already operate on the active `EditTarget`
- Draw collision gizmo as a **red/orange wireframe cube** (distinct from green/yellow trigger gizmo)
- Save logic: convert model-relative → anchor-relative (`stored_offset = model_offset + collision_offset`)
- Load logic: convert anchor-relative → model-relative (`collision_offset = stored_offset - model_offset`)

The move/resize widgets in `mod.rs` already dispatch by `EditTarget`. Adding a `Collision` variant extends them naturally: same arrow/handle interactions, writing to `collision_offset`/`collision_half_extents` instead of `trigger_offset`/`trigger_half_extents`.

---

## Step 7 — RON Data

Update `assets/library/default.obstacles.ron` with `collision_volume` for each gate type. Initial values are estimates; tune in the Workshop.

Example for `gate_loop` (opening centered at y≈11.7, ~12.6m wide, ~7.8m tall):
```ron
collision_volume: Some((
    offset: (0.4, 8.0, 0.05),
    half_extents: (7.5, 8.0, 1.5),
)),
```

---

## Step 8 — System Registration

**`src/race/mod.rs`**:
- Add `pub mod collision;`
- Register `build_obstacle_collision_cache` (polling, `run_if(not(resource_exists::<ObstacleCollisionCache>))`)
- Insert `obstacle_collision_check` into the race logic `.chain()` between gate_trigger_check and miss_detection
- Add `commands.remove_resource::<collision::ObstacleCollisionCache>()` to `cleanup_race`

---

## Performance

12 drones × ~15 obstacles = ~180 slab tests/frame. Each test: 3 axis projections (~6 dot products, a few comparisons). Total: ~2000 flops — negligible. No spatial acceleration needed.

---

## Files Modified

| File | Change |
|------|--------|
| `src/obstacle/definition.rs` | Add `CollisionVolumeConfig`, field on `ObstacleDef` |
| `src/obstacle/spawning.rs` | Add `ObstacleCollisionVolume` component, update `spawn_obstacle()` |
| `src/course/loader.rs` | Pass collision config to `spawn_obstacle()` |
| `src/editor/course_editor/ui.rs` | Pass collision config to `spawn_obstacle()` (2 call sites) |
| `src/race/collision.rs` | **NEW** — cache, pure functions, collision system, crash helper, tests |
| `src/race/mod.rs` | Register module + systems, cleanup |
| `src/race/progress.rs` | Add `DnfReason::ObstacleCollision` |
| `src/race/gate.rs` | Refactor `miss_detection` to use shared `crash_drone()` |
| `src/editor/workshop/mod.rs` | Add collision volume fields to `WorkshopState`, `EditTarget::Collision` |
| `src/editor/workshop/ui.rs` | Add collision toggle, gizmo drawing, save/load |
| `assets/library/default.obstacles.ron` | Add `collision_volume` for each gate |
| `CLAUDE.md` | Document new pattern |

---

## Unit Tests (`src/race/collision.rs`)

**`segment_obb_intersection`**: through center, miss, parallel inside, parallel outside, starts inside, too short, rotated OBB hit, rotated miss, expansion widens hit, hit point on surface.

**`point_in_gate_opening`**: center, inside bounds, outside width, outside height, different depth (should still pass), rotated axes.

**Integration**: segment through gate opening (OBB hit but exempted), segment through frame (OBB hit, not exempted), segment misses entirely.

---

## Manual Testing Checklist

| # | Test | Expected |
|---|------|----------|
| 1 | Drones fly through gate center | No collision, gate pass recorded |
| 2 | Drone clips gate frame | Crash with explosion |
| 3 | Drone barely through opening edge | Passes (within trigger bounds) |
| 4 | VictoryLap drone hits obstacle | Visual crash, finish time preserved |
| 5 | Race restart | Cache rebuilt, collisions work |
| 6 | Workshop: toggle collision volume | Red gizmo appears/disappears |
| 7 | Workshop: move/resize collision volume | Arrows and handles work correctly |
| 8 | Workshop: save and reload | Collision volume persists in RON |
| 9 | 12-drone race | Steady 60fps, no false positives |
| 10 | Remove collision_volume from RON | That gate has no collision (backward compat) |
| 11 | Leaderboard shows DNF | Correct for collision crashes |
| 12 | All drones finish normally | No false collisions on clean passes |
