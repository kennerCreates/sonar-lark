# Plan: Course Cameras + Post-Race Wandering Drones

## Context

Currently the race has three camera modes (Chase/FPV/Spectator) on keys 1/2/3. After a race ends, drones continue doing victory laps on the course spline. This plan adds **editor-placed course cameras** as the primary viewing experience during races, and replaces victory laps with **ambient wandering** in the results screen.

---

## Phase 1: Data Model — CameraInstance in CourseData

**Files:** `src/course/data.rs`

Add `CameraInstance` struct and `cameras` field to `CourseData`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraInstance {
    pub translation: Vec3,
    pub rotation: Quat,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub label: Option<String>,
}
```

```rust
pub struct CourseData {
    pub name: String,
    pub instances: Vec<ObstacleInstance>,
    #[serde(default)]
    pub props: Vec<PropInstance>,
    #[serde(default)]
    pub cameras: Vec<CameraInstance>,  // NEW
}
```

`#[serde(default)]` ensures all existing `.course.ron` files load without changes.

**Tests:** Add roundtrip serialization and backward-compat tests in `src/course/loader.rs`.

---

## Phase 2: Editor — Camera Placement

Follow the existing **prop placement pattern** exactly.

### 2a. Components and types (`src/editor/course_editor/mod.rs`)

- Add `PlacedCamera` component:
  ```rust
  #[derive(Component, Clone)]
  pub struct PlacedCamera {
      pub is_primary: bool,
      pub label: Option<String>,
  }
  ```
- Add `EditorTab::Cameras` variant
- Update `PlacedFilter` to include `With<PlacedCamera>`
- Update `find_placed_ancestor_from_ray` to check `PlacedCamera`

### 2b. UI markers (`src/editor/course_editor/ui/types.rs`)

New components: `CamerasTabButton`, `CameraPaletteContent`, `PlaceCameraButton`, `CameraPrimaryToggle`, `CameraPrimaryLabel`

New resource: `CameraEditorMeshes` (small cuboid mesh + CelMaterial in SKY/SUNSHINE)

### 2c. Tab UI (`src/editor/course_editor/ui/build.rs`)

Add third tab button "Cameras" and `CameraPaletteContent` panel (hidden by default) containing:
- "Place Camera" button
- "Primary: No" label + toggle button
- Help text about using Move/Rotate to aim

### 2d. Interaction systems (`src/editor/course_editor/ui/systems.rs`)

- Extend `handle_tab_switch` for the third tab
- Add `handle_camera_placement` — spawn PlacedCamera entity with cuboid mesh at camera position
- Add `handle_camera_primary_toggle` — toggle `is_primary`, enforce single-primary (clear others)
- Add `update_camera_primary_label` — reflect selected camera's primary status
- Add `setup_camera_editor_meshes` — create mesh + materials on `OnEnter(CourseEditor)`

### 2e. Gizmos (`src/editor/course_editor/overlays.rs`)

Add `draw_camera_gizmos` — wireframe frustum per PlacedCamera (yellow for primary, sky blue for normal), with up-arrow for orientation.

### 2f. Save/load integration (`src/editor/course_editor/ui/file_ops.rs`)

- Update `build_course_data` to gather `PlacedCamera` entities -> `CameraInstance` vec
- Update `load_course_into_editor` to spawn `PlacedCamera` entities from `course.cameras`
- Warn if >9 cameras at save time (soft cap)

### 2g. Plugin registration (`src/editor/course_editor/mod.rs`)

- Register new systems in existing `Update` groups
- Add `setup_camera_editor_meshes` to `OnEnter(CourseEditor)`
- Add `CameraEditorMeshes` to `cleanup_course_editor` resource removal

---

## Phase 3: Race Camera Mode Refactor

### 3a. CameraMode enum (`src/camera/switching.rs`)

```rust
pub enum CameraMode {
    Chase,
    Fpv,
    Spectator,
    CourseCamera(usize),  // NEW — index into CourseCameras
}
```

### 3b. CourseCameras resource (`src/camera/switching.rs`)

```rust
#[derive(Resource, Default)]
pub struct CourseCameras {
    pub cameras: Vec<CourseCameraEntry>,
}

pub struct CourseCameraEntry {
    pub transform: Transform,
    pub label: Option<String>,
}
```

Built from `CourseData.cameras` on `OnEnter(AppState::Race)`. Primary camera goes at index 0, non-primary cameras follow in order. Cleaned up `OnExit(AppState::Results)`.

### 3c. Key bindings (`src/camera/switching.rs` — rewrite `handle_camera_keys`)

**Number keys (unmodified):**
| Key | Action |
|-----|--------|
| 1 | `CourseCamera(0)` (primary) — falls back to Chase if no cameras |
| 2 | Chase (always) |
| 3-9 | `CourseCamera(1..7)` if that camera exists, else ignored |
| 0 | `CourseCamera(8)` if exists, else ignored |

**Shift combos:**
| Combo | Action |
|-------|--------|
| Shift+F | FPV (cycle drone on repeat) |
| Shift+S | Spectator |

### 3d. Course camera update system (`src/camera/mod.rs`)

New `course_camera_update` system — snaps `MainCamera` transform to the stored `CourseCameraEntry.transform`. Gated by `camera_mode_is_course_camera()` run condition (matches any `CourseCamera(_)`).

Static cameras = no spring smoothing needed. Just copy the transform and reset FOV to base.

### 3e. Race start camera (`src/camera/switching.rs`)

`reset_camera_for_race`: If `CourseCameras` has entries, default to `CourseCamera(0)`. Otherwise default to `Chase`.

**Ordering:** `build_course_cameras` must run before `reset_camera_for_race` — use `.chain()` on `OnEnter(Race)`.

### 3f. End-of-race camera switch (`src/race/lifecycle.rs`)

In `check_race_finished`, when transitioning to `Finished`: if CourseCameras exist, force `CameraState.mode = CourseCamera(0)`.

### 3g. Camera HUD update (`src/race/ui.rs`)

Update `update_camera_hud` to:
- Show "CAM 1: {label}" / "CAM 1" for course cameras
- Show available keys in hint line based on camera count
- Add `CourseCameras` as a system parameter

### 3h. Plugin registration (`src/camera/mod.rs`)

- Register `course_camera_update` in Update with `in_race_or_results` + `camera_mode_is_course_camera`
- Register `build_course_cameras` on `OnEnter(Race)` chained before `reset_camera_for_race`
- Clean up `CourseCameras` on `OnExit(Results)`
- Update `camera_mode_is` — the existing closure uses `==` which won't match `CourseCamera(n)` against other modes, so existing gating still works. Add a new `camera_mode_is_course_camera` function for the variant match.

---

## Phase 4: Post-Race Wandering Drones

### 4a. New DronePhase variant (`src/drone/components.rs`)

```rust
pub enum DronePhase {
    Idle,
    Racing,
    VictoryLap,
    Wandering,  // NEW
    Crashed,
}
```

New component:
```rust
#[derive(Component)]
pub struct WanderState {
    pub target: Vec3,
    pub dwell_timer: f32,
    pub step: u32,  // increments for deterministic waypoint variety
}
```

### 4b. Wandering AI system (`src/drone/ai.rs`)

Constants: `WANDER_SPEED = 8.0`, `WANDER_HEIGHT_MIN = 3.0`, `WANDER_HEIGHT_MAX = 12.0`, `WANDER_DWELL = 2.0..5.0`

New `update_wander_targets` system:
- Only processes drones with `DronePhase::Wandering`
- When close to target or dwell timer expires -> pick next waypoint via deterministic hash (Fibonacci hashing from drone index + step counter)
- Waypoints stay within a bounding box computed from course obstacle positions (with padding)
- Sets `DesiredPosition { position: target, velocity_hint: dir * WANDER_SPEED, max_speed: WANDER_SPEED }`
- The existing PID physics chain handles actual flight

### 4c. VictoryLap -> Wandering transition

New system `transition_to_wandering` on `OnEnter(AppState::Results)`:
- All `VictoryLap` drones -> `Wandering` + insert `WanderState`
- Crashed drones stay crashed (hidden, no change)

### 4d. Update run conditions and AI skips

- `drones_are_active` (`src/race/lifecycle.rs`): Include `DronePhase::Wandering` in the active check
- `update_ai_targets` / `compute_racing_line` (`src/drone/ai.rs`): Add `Wandering` to the skip list alongside `Idle | Crashed`
- `proximity_avoidance`: Already iterates all active drones — wandering drones benefit naturally

### 4e. System registration (`src/drone/mod.rs`)

Add `update_wander_targets` to the FixedUpdate chain, gated by `drones_are_active`, positioned after the existing AI systems but before physics:

```
update_ai_targets -> compute_racing_line -> proximity_avoidance -> update_wander_targets -> position_pid -> ...
```

Register `transition_to_wandering` on `OnEnter(AppState::Results)`.

---

## Phase 5: Integration and Polish

1. **Bounding box for wander area**: Compute from `CourseData.instances` translations at race start, store as a resource. Wandering waypoints stay within this box + padding.
2. **Documentation**: Update `ARCHITECTURE.md`, `CLAUDE.md`, `TODO.md` with new types and patterns.
3. **Cargo build + fix warnings**.
4. **Cargo test** — new tests for serialization roundtrip, waypoint bounds, determinism.
5. **Manual testing checklist** (presented after implementation).

---

## Files Modified (Summary)

| File | Changes |
|------|---------|
| `src/course/data.rs` | Add `CameraInstance`, add `cameras` to `CourseData` |
| `src/course/loader.rs` | Tests for camera serialization |
| `src/editor/course_editor/mod.rs` | `PlacedCamera`, `EditorTab::Cameras`, update `PlacedFilter`, update ray-cast |
| `src/editor/course_editor/ui/types.rs` | Camera UI markers, `CameraEditorMeshes` |
| `src/editor/course_editor/ui/build.rs` | Cameras tab UI |
| `src/editor/course_editor/ui/systems.rs` | Camera placement, primary toggle, tab switch extension |
| `src/editor/course_editor/ui/file_ops.rs` | Save/load camera entities |
| `src/editor/course_editor/overlays.rs` | Camera frustum gizmos |
| `src/camera/switching.rs` | `CameraMode::CourseCamera`, `CourseCameras`, rewritten key handler |
| `src/camera/mod.rs` | `course_camera_update` system, new registrations |
| `src/race/lifecycle.rs` | End-of-race camera switch, update `drones_are_active` |
| `src/race/ui.rs` | Camera HUD for course cameras |
| `src/drone/components.rs` | `DronePhase::Wandering`, `WanderState` |
| `src/drone/ai.rs` | `update_wander_targets`, skip wandering in spline systems |
| `src/drone/mod.rs` | Register wander system, transition system |

## Performance

No risk to 60fps:
- Course cameras: single Transform copy per frame when active
- Wandering: O(12) distance checks per FixedUpdate tick, trivial
- No new per-frame allocations, no new heavy queries
