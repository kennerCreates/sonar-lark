mod move_gizmo;
mod rotate_gizmo;
mod scale_gizmo;

use bevy::prelude::*;

use crate::editor::gizmos::Axis;

pub(super) use move_gizmo::{draw_move_gizmo, handle_move_gizmo};
pub(super) use rotate_gizmo::{draw_rotate_gizmo, handle_rotate_gizmo};
pub(super) use scale_gizmo::{draw_scale_gizmo, handle_scale_gizmo};

// --- Widget resources ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveDragMode {
    XzPlane,
    SingleAxis(Axis),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveHoverPart {
    Arrow(Axis),
    PlaneSquare,
}

#[derive(Resource, Default)]
pub(super) struct MoveWidgetState {
    pub(super) active_drag: Option<MoveDragMode>,
    pub(super) hovered_part: Option<MoveHoverPart>,
    pub(crate) drag_anchor: Vec3,
    pub(crate) entity_start_pos: Vec3,
    /// Full transform at drag start (for undo).
    pub(crate) entity_start_transform: Transform,
}

#[derive(Resource, Default)]
pub(super) struct RotateWidgetState {
    pub(super) active: bool,
    pub(super) hovered: bool,
    pub(crate) active_axis: Axis,
    pub(crate) drag_start_angle: f32,
    pub(crate) entity_start_rotation: Quat,
    pub(crate) start_yaw_quat: Quat,
    /// Full transform at drag start (for undo).
    pub(crate) entity_start_transform: Transform,
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
    /// Full transform at drag start (for undo).
    pub(crate) entity_start_transform: Transform,
}

// --- Constants ---

const ARROW_LENGTH: f32 = 3.75;
const ARROW_HIT_THRESHOLD: f32 = 25.0;

const SCALE_HANDLE_LENGTH: f32 = 3.75;
const SCALE_CUBE_SIZE: f32 = 0.45;
const SCALE_HIT_THRESHOLD: f32 = 25.0;
const SCALE_SENSITIVITY: f32 = 1.0;

const PLANE_INDICATOR_FRAC: f32 = 0.3;
