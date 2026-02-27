# Editor Gizmo Rework — Detailed Implementation Plan

## Context

The course editor's transform gizmos (move, rotate, scale) currently use world-aligned axes, Blender-style hotkeys (G/R/S), and default Bevy line thickness. This rework:
1. Gizmos rotate with entity's Y rotation
2. Rotation snaps in 5° increments
3. Hotkeys: 1=Move, 2=Rotate, 3=Scale
4. Move default=XZ plane, Shift=Y axis
5. Scale default=uniform, Shift=non-uniform per-axis
6. Rotation default=Y, Shift=Z, Ctrl=X
7. Gizmos 1.5x larger and thicker lines

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/editor/gizmos.rs` | Add 3 new helper functions |
| `src/editor/course_editor/transform_gizmos.rs` | Rework widget states, constants, all 6 draw/handle fns |
| `src/editor/course_editor/mod.rs` | Hotkeys, line width, placement guard |
| `src/editor/course_editor/ui/build.rs` | Button labels, hint text |

---

## Step 1: Math Helpers — `src/editor/gizmos.rs`

### 1a. Add `yaw_quat_from_transform` (after line 35, below Axis impl)

```rust
/// Extract only the Y-rotation (yaw) from a transform's rotation.
/// Used so gizmo axes follow the entity's horizontal orientation
/// without being affected by pitch/roll.
pub(crate) fn yaw_quat_from_transform(transform: &Transform) -> Quat {
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
    Quat::from_rotation_y(yaw)
}
```

### 1b. Add `rotated_direction` method to `Axis` impl (after `color()`, line ~34)

```rust
/// Axis direction rotated by the entity's yaw quaternion.
/// Gizmos use this so they follow the entity's Y rotation.
pub(crate) fn rotated_direction(self, yaw_quat: Quat) -> Vec3 {
    yaw_quat * self.direction()
}
```

### 1c. Add `rotated_perpendicular_basis` (after `perpendicular_basis()`, line ~107)

```rust
/// Perpendicular basis vectors rotated by yaw, for sampling rotation
/// rings that follow the entity's Y orientation.
pub(crate) fn rotated_perpendicular_basis(axis: Axis, yaw_quat: Quat) -> (Vec3, Vec3) {
    let (a, b) = perpendicular_basis(axis);
    (yaw_quat * a, yaw_quat * b)
}
```

### 1d. Add unit tests (inside existing `mod tests`)

```rust
#[test]
fn rotated_direction_90_y() {
    let yaw = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    // 90° Y rotation: X maps to -Z, Z maps to X, Y stays Y
    let rx = Axis::X.rotated_direction(yaw);
    assert!((rx - Vec3::NEG_Z).length() < 1e-5);
    let rz = Axis::Z.rotated_direction(yaw);
    assert!((rz - Vec3::X).length() < 1e-5);
    let ry = Axis::Y.rotated_direction(yaw);
    assert!((ry - Vec3::Y).length() < 1e-5);
}

#[test]
fn rotated_perpendicular_basis_identity() {
    let (a, b) = rotated_perpendicular_basis(Axis::Y, Quat::IDENTITY);
    assert!((a - Vec3::X).length() < 1e-5);
    assert!((b - Vec3::Z).length() < 1e-5);
}

#[test]
fn yaw_quat_extracts_y_only() {
    // Transform with mixed rotation: 45° yaw + 30° pitch
    let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)
        * Quat::from_rotation_x(0.5);
    let tf = Transform::from_rotation(rotation);
    let yaw_q = yaw_quat_from_transform(&tf);
    // The extracted yaw quat should only rotate around Y
    let up = yaw_q * Vec3::Y;
    assert!((up - Vec3::Y).length() < 1e-5, "Yaw quat should not tilt Y axis");
}
```

**No existing functions are changed.** All additions are purely additive.

---

## Step 2: Widget State Restructuring — `src/editor/course_editor/transform_gizmos.rs`

### 2a. Replace `MoveWidgetState` (lines 15-20)

**Before:**
```rust
#[derive(Resource, Default)]
pub(super) struct MoveWidgetState {
    pub(super) active_axis: Option<Axis>,
    pub(super) hovered_axis: Option<Axis>,
    drag_offset: f32,
}
```

**After:**
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveDragMode {
    XzPlane,
    YAxis,
}

#[derive(Resource, Default)]
pub(super) struct MoveWidgetState {
    pub(super) active_drag: Option<MoveDragMode>,
    pub(super) hovered: bool,
    drag_anchor: Vec3,
    entity_start_pos: Vec3,
}
```

### 2b. Replace `RotateWidgetState` (lines 22-28)

**Before:**
```rust
#[derive(Resource, Default)]
pub(super) struct RotateWidgetState {
    pub(super) active_axis: Option<Axis>,
    pub(super) hovered_axis: Option<Axis>,
    drag_start_angle: f32,
    entity_start_rotation: Quat,
}
```

**After:**
```rust
#[derive(Resource, Default)]
pub(super) struct RotateWidgetState {
    pub(super) active: bool,
    pub(super) hovered: bool,
    active_axis: Axis,
    drag_start_angle: f32,
    entity_start_rotation: Quat,
}
```

Note: `Axis` needs `Default` impl. Add `#[derive(Default)]` to `Axis` in `gizmos.rs` (Y is a sensible default — it's the primary rotation axis).

### 2c. Replace `ScaleWidgetState` (lines 30-36)

**Before:**
```rust
#[derive(Resource, Default)]
pub(super) struct ScaleWidgetState {
    pub(super) active_axis: Option<Axis>,
    pub(super) hovered_axis: Option<Axis>,
    drag_start_t: f32,
    entity_start_scale: Vec3,
}
```

**After:**
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScaleDragMode {
    Uniform,
    PerAxis(Axis),
}

#[derive(Resource, Default)]
pub(super) struct ScaleWidgetState {
    pub(super) active_drag: Option<ScaleDragMode>,
    pub(super) hovered_axis: Option<Axis>,
    pub(super) hovered_center: bool,
    drag_start_t: f32,
    entity_start_scale: Vec3,
}
```

---

## Step 3: Update Constants — `src/editor/course_editor/transform_gizmos.rs`

**Before (lines 38-50):**
```rust
const ARROW_LENGTH: f32 = 2.5;
const ARROW_HIT_THRESHOLD: f32 = 25.0;

const RING_RADIUS: f32 = 2.0;
const RING_SAMPLES: usize = 32;
const RING_HIT_THRESHOLD: f32 = 15.0;

const SCALE_HANDLE_LENGTH: f32 = 2.5;
const SCALE_CUBE_SIZE: f32 = 0.3;
const SCALE_HIT_THRESHOLD: f32 = 25.0;
const SCALE_SENSITIVITY: f32 = 1.0;
```

**After:**
```rust
const ARROW_LENGTH: f32 = 3.75;          // 2.5 * 1.5
const ARROW_HIT_THRESHOLD: f32 = 25.0;

const RING_RADIUS: f32 = 3.0;            // 2.0 * 1.5
const RING_SAMPLES: usize = 32;
const RING_HIT_THRESHOLD: f32 = 15.0;

const SCALE_HANDLE_LENGTH: f32 = 3.75;   // 2.5 * 1.5
const SCALE_CUBE_SIZE: f32 = 0.45;       // 0.3 * 1.5
const SCALE_HIT_THRESHOLD: f32 = 25.0;
const SCALE_SENSITIVITY: f32 = 1.0;

const ROTATION_STEP_DEG: f32 = 5.0;

/// Size of the XZ-plane indicator square (fraction of ARROW_LENGTH).
const PLANE_INDICATOR_FRAC: f32 = 0.3;
```

---

## Step 4: Move Gizmo Rework — `src/editor/course_editor/transform_gizmos.rs`

### 4a. Replace `draw_move_gizmo` (lines 54-78)

```rust
pub(super) fn draw_move_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<MoveWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Move {
        return;
    }
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let yaw_quat = yaw_quat_from_transform(transform);
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
        || keyboard.pressed(KeyCode::ShiftRight);
    let origin = transform.translation;

    let rot_x = Axis::X.rotated_direction(yaw_quat);
    let rot_z = Axis::Z.rotated_direction(yaw_quat);

    // XZ-plane arrows + square indicator
    let xz_active = matches!(widget.active_drag, Some(MoveDragMode::XzPlane));
    let xz_brightness = if xz_active {
        1.0
    } else if !shift_held && widget.hovered {
        0.8
    } else {
        0.5
    };
    let xz_color = Color::srgb(xz_brightness, xz_brightness, 0.0);
    gizmos.arrow(origin, origin + rot_x * ARROW_LENGTH, xz_color);
    gizmos.arrow(origin, origin + rot_z * ARROW_LENGTH, xz_color);
    // Small square between X and Z to indicate plane movement
    let sq = ARROW_LENGTH * PLANE_INDICATOR_FRAC;
    gizmos.line(origin + rot_x * sq, origin + rot_x * sq + rot_z * sq, xz_color);
    gizmos.line(origin + rot_z * sq, origin + rot_x * sq + rot_z * sq, xz_color);

    // Y-axis arrow (Shift mode)
    let y_active = matches!(widget.active_drag, Some(MoveDragMode::YAxis));
    let y_brightness = if y_active {
        1.0
    } else if shift_held && widget.hovered {
        0.8
    } else {
        0.5
    };
    let y_color = Color::srgb(0.0, y_brightness, 0.0);
    gizmos.arrow(origin, origin + Vec3::Y * ARROW_LENGTH, y_color);
}
```

### 4b. Replace `handle_move_gizmo` (lines 80-147)

```rust
pub(super) fn handle_move_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    state: Res<PlacementState>,
    mut widget: ResMut<MoveWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Move {
        widget.active_drag = None;
        widget.hovered = false;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_drag = None;
        widget.hovered = false;
        return;
    };
    let Ok(mut transform) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let origin = transform.translation;
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(drag_mode) = widget.active_drag {
        if mouse_buttons.pressed(MouseButton::Left) {
            match drag_mode {
                MoveDragMode::XzPlane => {
                    if let Some(hit) = ray_intersect_plane(
                        ray,
                        Vec3::new(0.0, widget.entity_start_pos.y, 0.0),
                        Vec3::Y,
                    ) {
                        let delta = hit - widget.drag_anchor;
                        transform.translation = widget.entity_start_pos
                            + Vec3::new(delta.x, 0.0, delta.z);
                    }
                }
                MoveDragMode::YAxis => {
                    let t = closest_point_on_axis(ray, widget.entity_start_pos, Vec3::Y);
                    let delta = t - widget.drag_anchor.y;
                    transform.translation = widget.entity_start_pos + Vec3::Y * delta;
                }
            }
        } else {
            widget.active_drag = None;
        }
    } else {
        // Hover detection: check screen distance to all three arrows
        let yaw_quat = yaw_quat_from_transform(&transform);
        let arrows_ends = [
            origin + Axis::X.rotated_direction(yaw_quat) * ARROW_LENGTH,
            origin + Vec3::Y * ARROW_LENGTH,
            origin + Axis::Z.rotated_direction(yaw_quat) * ARROW_LENGTH,
        ];
        let mut near = false;
        for end in arrows_ends {
            let Ok(ss) = camera.world_to_viewport(camera_gt, origin) else {
                continue;
            };
            let Ok(se) = camera.world_to_viewport(camera_gt, end) else {
                continue;
            };
            if point_to_segment_distance(cursor_pos, ss, se) < ARROW_HIT_THRESHOLD {
                near = true;
                break;
            }
        }
        widget.hovered = near;

        if !mouse_over_ui && mouse_buttons.just_pressed(MouseButton::Left) && near {
            let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
                || keyboard.pressed(KeyCode::ShiftRight);
            let mode = if shift_held {
                MoveDragMode::YAxis
            } else {
                MoveDragMode::XzPlane
            };
            widget.entity_start_pos = origin;
            match mode {
                MoveDragMode::XzPlane => {
                    if let Some(hit) = ray_intersect_plane(ray, origin, Vec3::Y) {
                        widget.drag_anchor = hit;
                        widget.active_drag = Some(mode);
                    }
                }
                MoveDragMode::YAxis => {
                    let t = closest_point_on_axis(ray, origin, Vec3::Y);
                    widget.drag_anchor = Vec3::new(0.0, t, 0.0);
                    widget.active_drag = Some(mode);
                }
            }
        }
    }
}
```

---

## Step 5: Rotate Gizmo Rework — `src/editor/course_editor/transform_gizmos.rs`

### 5a. Add `angle_in_plane` helper (near existing `angle_in_ring_plane`, line ~404)

```rust
/// Compute the angle of `point` around `center` in an arbitrary plane
/// defined by two orthonormal reference vectors.
fn angle_in_plane(point: Vec3, center: Vec3, ref1: Vec3, ref2: Vec3) -> f32 {
    let local = point - center;
    local.dot(ref2).atan2(local.dot(ref1))
}
```

Keep existing `angle_in_ring_plane` but refactor it to delegate:
```rust
pub(crate) fn angle_in_ring_plane(point: Vec3, center: Vec3, axis: Axis) -> f32 {
    let (ref1, ref2) = perpendicular_basis(axis);
    angle_in_plane(point, center, ref1, ref2)
}
```

### 5b. Helper to determine axis from modifiers

```rust
fn rotation_axis_from_modifiers(keyboard: &ButtonInput<KeyCode>) -> Axis {
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        Axis::X
    } else if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        Axis::Z
    } else {
        Axis::Y
    }
}
```

### 5c. Replace `draw_rotate_gizmo` (lines 151-182)

```rust
pub(super) fn draw_rotate_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<RotateWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Rotate {
        return;
    }
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let pos = transform.translation;
    let yaw_quat = yaw_quat_from_transform(transform);

    // Determine which axis to show: locked during drag, else from modifiers
    let display_axis = if widget.active {
        widget.active_axis
    } else {
        rotation_axis_from_modifiers(&keyboard)
    };

    let is_hovered = !widget.active && widget.hovered;
    let color = display_axis.color(is_hovered, widget.active);

    let face_quat = yaw_quat
        * match display_axis {
            Axis::X => Quat::from_rotation_arc(Vec3::Z, Vec3::X),
            Axis::Y => Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            Axis::Z => Quat::IDENTITY,
        };
    let iso = Isometry3d::new(pos, face_quat);
    gizmos.circle(iso, RING_RADIUS, color);
}
```

### 5d. Replace `handle_rotate_gizmo` (lines 184-263)

```rust
pub(super) fn handle_rotate_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    state: Res<PlacementState>,
    mut widget: ResMut<RotateWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Rotate {
        widget.active = false;
        widget.hovered = false;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active = false;
        widget.hovered = false;
        return;
    };
    let Ok(mut transform) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let pos = transform.translation;
    let yaw_quat = yaw_quat_from_transform(&transform);
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if widget.active {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = widget.active_axis.rotated_direction(yaw_quat);
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                let (ref1, ref2) =
                    rotated_perpendicular_basis(widget.active_axis, yaw_quat);
                let current_angle = angle_in_plane(hit, pos, ref1, ref2);
                let raw_delta = current_angle - widget.drag_start_angle;

                // Snap to 5° increments
                let step = ROTATION_STEP_DEG.to_radians();
                let snapped_delta = (raw_delta / step).round() * step;

                transform.rotation = Quat::from_axis_angle(axis_dir, snapped_delta)
                    * widget.entity_start_rotation;
            }
        } else {
            widget.active = false;
        }
    } else {
        // Determine which ring is currently displayed
        let current_axis = rotation_axis_from_modifiers(&keyboard);
        let (ref1, ref2) = rotated_perpendicular_basis(current_axis, yaw_quat);
        let min_dist = sample_ring_screen_dist(
            camera,
            camera_gt,
            cursor_pos,
            pos,
            ref1,
            ref2,
            RING_RADIUS,
            RING_SAMPLES,
        );
        widget.hovered = min_dist < RING_HIT_THRESHOLD;

        if !mouse_over_ui && mouse_buttons.just_pressed(MouseButton::Left) && widget.hovered
        {
            let axis_dir = current_axis.rotated_direction(yaw_quat);
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                widget.active = true;
                widget.active_axis = current_axis;
                widget.drag_start_angle = angle_in_plane(hit, pos, ref1, ref2);
                widget.entity_start_rotation = transform.rotation;
            }
        }
    }
}
```

---

## Step 6: Scale Gizmo Rework — `src/editor/course_editor/transform_gizmos.rs`

### 6a. Replace `draw_scale_gizmo` (lines 267-294)

```rust
pub(super) fn draw_scale_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<ScaleWidgetState>,
    placed_query: Query<&Transform, PlacedFilter>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.transform_mode != TransformMode::Scale {
        return;
    }
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let yaw_quat = yaw_quat_from_transform(transform);
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
        || keyboard.pressed(KeyCode::ShiftRight);
    let origin = transform.translation;

    // Per-axis lines and cubes (always drawn)
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_per_axis_active = matches!(
            widget.active_drag,
            Some(ScaleDragMode::PerAxis(a)) if a == axis
        );
        let is_hovered = shift_held && widget.hovered_axis == Some(axis);
        let color = axis.color(is_hovered, is_per_axis_active);
        let dir = axis.rotated_direction(yaw_quat);
        let tip = origin + dir * SCALE_HANDLE_LENGTH;
        gizmos.line(origin, tip, color);
        let cube_tf = Transform::from_translation(tip)
            .with_rotation(yaw_quat)
            .with_scale(Vec3::splat(SCALE_CUBE_SIZE));
        gizmos.cube(cube_tf, color);
    }

    // Center cube for uniform scale
    let uniform_active = matches!(widget.active_drag, Some(ScaleDragMode::Uniform));
    let uniform_hovered = !shift_held && widget.hovered_center;
    let center_brightness = if uniform_active {
        1.0
    } else if uniform_hovered {
        0.8
    } else {
        0.4
    };
    let center_color = Color::srgb(center_brightness, center_brightness, center_brightness);
    let center_tf =
        Transform::from_translation(origin).with_scale(Vec3::splat(SCALE_CUBE_SIZE * 1.5));
    gizmos.cube(center_tf, center_color);
}
```

### 6b. Replace `handle_scale_gizmo` (lines 296-376)

```rust
pub(super) fn handle_scale_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    state: Res<PlacementState>,
    mut widget: ResMut<ScaleWidgetState>,
    mut placed_query: Query<&mut Transform, PlacedFilter>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Scale {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_drag = None;
        widget.hovered_axis = None;
        widget.hovered_center = false;
        return;
    };
    let Ok(mut transform) = placed_query.get_mut(entity) else {
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let origin = transform.translation;
    let yaw_quat = yaw_quat_from_transform(&transform);
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(drag_mode) = widget.active_drag {
        if mouse_buttons.pressed(MouseButton::Left) {
            match drag_mode {
                ScaleDragMode::Uniform => {
                    // Use camera-right vector as drag axis for uniform scale
                    let cam_right = *camera_gt.right();
                    let current_t = closest_point_on_axis(ray, origin, cam_right);
                    let delta = (current_t - widget.drag_start_t) * SCALE_SENSITIVITY;
                    transform.scale =
                        (widget.entity_start_scale + Vec3::splat(delta)).max(Vec3::splat(0.01));
                }
                ScaleDragMode::PerAxis(axis) => {
                    let axis_dir = axis.rotated_direction(yaw_quat);
                    let current_t = closest_point_on_axis(ray, origin, axis_dir);
                    let delta = current_t - widget.drag_start_t;
                    match axis {
                        Axis::X => {
                            transform.scale.x = (widget.entity_start_scale.x
                                + delta * SCALE_SENSITIVITY)
                                .max(0.01);
                        }
                        Axis::Y => {
                            transform.scale.y = (widget.entity_start_scale.y
                                + delta * SCALE_SENSITIVITY)
                                .max(0.01);
                        }
                        Axis::Z => {
                            transform.scale.z = (widget.entity_start_scale.z
                                + delta * SCALE_SENSITIVITY)
                                .max(0.01);
                        }
                    }
                }
            }
        } else {
            widget.active_drag = None;
        }
    } else {
        let shift_held = keyboard.pressed(KeyCode::ShiftLeft)
            || keyboard.pressed(KeyCode::ShiftRight);

        if shift_held {
            // Per-axis hover: find closest axis handle
            widget.hovered_center = false;
            let mut best_axis: Option<Axis> = None;
            let mut best_dist = SCALE_HIT_THRESHOLD;
            for axis in [Axis::X, Axis::Y, Axis::Z] {
                let tip = origin + axis.rotated_direction(yaw_quat) * SCALE_HANDLE_LENGTH;
                let Ok(ss) = camera.world_to_viewport(camera_gt, origin) else {
                    continue;
                };
                let Ok(se) = camera.world_to_viewport(camera_gt, tip) else {
                    continue;
                };
                let dist = point_to_segment_distance(cursor_pos, ss, se);
                if dist < best_dist {
                    best_dist = dist;
                    best_axis = Some(axis);
                }
            }
            widget.hovered_axis = best_axis;

            if !mouse_over_ui
                && mouse_buttons.just_pressed(MouseButton::Left)
                && let Some(axis) = best_axis
            {
                let axis_dir = axis.rotated_direction(yaw_quat);
                widget.active_drag = Some(ScaleDragMode::PerAxis(axis));
                widget.drag_start_t = closest_point_on_axis(ray, origin, axis_dir);
                widget.entity_start_scale = transform.scale;
            }
        } else {
            // Uniform hover: check distance to center
            widget.hovered_axis = None;
            let Ok(screen_origin) = camera.world_to_viewport(camera_gt, origin) else {
                return;
            };
            let dist = (cursor_pos - screen_origin).length();
            widget.hovered_center = dist < SCALE_HIT_THRESHOLD;

            if !mouse_over_ui
                && mouse_buttons.just_pressed(MouseButton::Left)
                && widget.hovered_center
            {
                let cam_right = *camera_gt.right();
                widget.active_drag = Some(ScaleDragMode::Uniform);
                widget.drag_start_t = closest_point_on_axis(ray, origin, cam_right);
                widget.entity_start_scale = transform.scale;
            }
        }
    }
}
```

---

## Step 7: Hotkey Change — `src/editor/course_editor/mod.rs`

### Replace lines 383-391 in `handle_transform_mode_keys`

**Before:**
```rust
let new_mode = if keyboard.just_pressed(KeyCode::KeyG) {
    Some(TransformMode::Move)
} else if keyboard.just_pressed(KeyCode::KeyR) {
    Some(TransformMode::Rotate)
} else if keyboard.just_pressed(KeyCode::KeyS) {
    Some(TransformMode::Scale)
} else {
    None
};
```

**After:**
```rust
let new_mode = if keyboard.just_pressed(KeyCode::Digit1) {
    Some(TransformMode::Move)
} else if keyboard.just_pressed(KeyCode::Digit2) {
    Some(TransformMode::Rotate)
} else if keyboard.just_pressed(KeyCode::Digit3) {
    Some(TransformMode::Scale)
} else {
    None
};
```

No conflicts — camera switching (1-9) is guarded by `in_state(AppState::Race)`.

---

## Step 8: Line Thickness — `src/editor/course_editor/mod.rs`

### In `setup_course_editor` (after line 257)

**Before:**
```rust
let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
config.depth_bias = -1.0;
```

**After:**
```rust
let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
config.depth_bias = -1.0;
config.line.width = 3.0; // 1.5x default (2.0)
```

### In `cleanup_course_editor` (after line 278)

**Before:**
```rust
let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
config.depth_bias = 0.0;
```

**After:**
```rust
let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
config.depth_bias = 0.0;
config.line.width = 2.0; // restore default
```

---

## Step 9: UI Label Updates — `src/editor/course_editor/ui/build.rs`

### Update button labels (lines 605-607)

**Before:**
```rust
spawn_transform_mode_button(row, "Move (G)", TransformMode::Move);
spawn_transform_mode_button(row, "Rotate (R)", TransformMode::Rotate);
spawn_transform_mode_button(row, "Scale (S)", TransformMode::Scale);
```

**After:**
```rust
spawn_transform_mode_button(row, "Move (1)", TransformMode::Move);
spawn_transform_mode_button(row, "Rotate (2)", TransformMode::Rotate);
spawn_transform_mode_button(row, "Scale (3)", TransformMode::Scale);
```

### Add modifier hint text (after "F → flip gate direction" text, around line 655)

Insert after the existing `"F  →  flip gate direction"` spawn:

```rust
panel.spawn((
    Text::new("Shift  →  Y-move / axis-scale / Z-rotate"),
    TextFont {
        font_size: 12.0,
        ..default()
    },
    TextColor(palette::CHAINMAIL),
));

panel.spawn((
    Text::new("Ctrl  →  X-rotate"),
    TextFont {
        font_size: 12.0,
        ..default()
    },
    TextColor(palette::CHAINMAIL),
));
```

---

## Step 10: Placement Guard Update — `src/editor/course_editor/mod.rs`

### Update `handle_placement_and_selection` (lines 303-306)

**Before:**
```rust
if move_widget.active_axis.is_some()
    || rotate_widget.active_axis.is_some()
    || scale_widget.active_axis.is_some()
```

**After:**
```rust
if move_widget.active_drag.is_some()
    || rotate_widget.active
    || scale_widget.active_drag.is_some()
```

---

## Step 11: Import Updates

### `transform_gizmos.rs` — update imports (line 6-9)

Add `rotated_perpendicular_basis` and `yaw_quat_from_transform`:

```rust
use crate::editor::gizmos::{
    closest_point_on_axis, perpendicular_basis, point_to_segment_distance,
    ray_intersect_plane, rotated_perpendicular_basis, yaw_quat_from_transform,
    Axis,
};
```

`perpendicular_basis` is still used by `angle_in_ring_plane` in this file — keep the import.

### `gizmos.rs` — no import changes needed

`EulerRot` is in the Bevy prelude, covered by `use bevy::prelude::*`.

---

## Step 12: Add `Default` to `Axis` — `src/editor/gizmos.rs`

`RotateWidgetState` derives `Default`, and its `active_axis: Axis` field needs `Axis: Default`.

**Before (line 5-10):**
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
```

**After:**
```rust
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Axis {
    #[default]
    X,
```

(X as default is fine — it's only used as the initial value before being overwritten at drag-start.)

---

## Verification

1. `cargo build` — no errors
2. `cargo clippy -- -D warnings` — no lints
3. `cargo test` — existing + new gizmo math tests pass
4. Manual testing:

| # | Test | Expected |
|---|------|----------|
| 1 | Select obstacle, press 1 | Move gizmo with XZ arrows following entity yaw |
| 2 | Drag XZ gizmo | Entity moves on ground plane, no Y change |
| 3 | Hold Shift + drag | Entity moves on Y axis only |
| 4 | Rotate entity 90°, re-select, check gizmos | X/Z arrows rotated to match entity |
| 5 | Press 2 | Single rotation ring (Y-axis) appears |
| 6 | Drag ring | Rotation snaps in 5° increments |
| 7 | Hold Shift, ring changes to Z | Z-axis ring displayed |
| 8 | Hold Ctrl, ring changes to X | X-axis ring displayed |
| 9 | Press 3 | Scale gizmo with center cube + 3 axis lines |
| 10 | Drag center cube | Uniform scale (all axes) |
| 11 | Hold Shift + drag axis handle | Single-axis scale |
| 12 | G/R/S keys | No longer switch modes |
| 13 | 1/2/3 keys | Correct mode activated |
| 14 | Visual: gizmos larger | ~1.5x previous size |
| 15 | Visual: lines thicker | Noticeably thicker than before |
| 16 | UI buttons | Labels show (1), (2), (3) |
| 17 | Modifier hints visible | Shift/Ctrl hints in panel |
| 18 | Click empty space during drag | Drag continues (no deselect) |
| 19 | Release mouse | Drag ends cleanly |
| 20 | Existing: placement, deletion, gate ordering | Still work unchanged |
