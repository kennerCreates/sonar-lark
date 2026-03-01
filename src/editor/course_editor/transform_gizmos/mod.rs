mod move_gizmo;
mod rotate_gizmo;
mod scale_gizmo;

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::editor::gizmos::Axis;

pub(super) use move_gizmo::{draw_move_gizmo, handle_move_gizmo};
pub(super) use rotate_gizmo::{draw_rotate_gizmo, handle_rotate_gizmo};
pub(super) use scale_gizmo::{draw_scale_gizmo, handle_scale_gizmo};

// --- Widget resources ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveDragMode {
    XzPlane,
    YAxis,
}

#[derive(Resource, Default)]
pub(super) struct MoveWidgetState {
    pub(super) active_drag: Option<MoveDragMode>,
    pub(super) hovered: bool,
    pub(crate) drag_anchor: Vec3,
    pub(crate) entity_start_pos: Vec3,
}

#[derive(Resource, Default)]
pub(super) struct RotateWidgetState {
    pub(super) active: bool,
    pub(super) hovered: bool,
    pub(crate) active_axis: Axis,
    pub(crate) drag_start_angle: f32,
    pub(crate) entity_start_rotation: Quat,
}

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
    pub(crate) drag_start_t: f32,
    pub(crate) entity_start_scale: Vec3,
}

// --- Constants ---

pub(crate) const ARROW_LENGTH: f32 = 3.75;
pub(crate) const ARROW_HIT_THRESHOLD: f32 = 25.0;

pub(crate) const RING_RADIUS: f32 = 3.0;
pub(crate) const RING_SAMPLES: usize = 32;
pub(crate) const RING_HIT_THRESHOLD: f32 = 15.0;

pub(crate) const SCALE_HANDLE_LENGTH: f32 = 3.75;
pub(crate) const SCALE_CUBE_SIZE: f32 = 0.45;
pub(crate) const SCALE_HIT_THRESHOLD: f32 = 25.0;
pub(crate) const SCALE_SENSITIVITY: f32 = 1.0;

pub(crate) const ROTATION_STEP_DEG: f32 = 5.0;

pub(crate) const PLANE_INDICATOR_FRAC: f32 = 0.3;

// --- Helpers ---

pub(crate) fn sample_ring_screen_dist(
    camera: &Camera,
    camera_gt: &GlobalTransform,
    cursor_pos: Vec2,
    center: Vec3,
    ref1: Vec3,
    ref2: Vec3,
    radius: f32,
    n: usize,
) -> f32 {
    let mut min_dist = f32::MAX;
    for i in 0..n {
        let angle = i as f32 * TAU / n as f32;
        let world_pt = center + (ref1 * angle.cos() + ref2 * angle.sin()) * radius;
        if let Ok(screen_pt) = camera.world_to_viewport(camera_gt, world_pt) {
            let d = (cursor_pos - screen_pt).length();
            if d < min_dist {
                min_dist = d;
            }
        }
    }
    min_dist
}
