pub mod ui;

use std::f32::consts::TAU;

use bevy::{
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
    prelude::*,
};

use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{ObstaclesGltfHandle, TriggerVolume};
use crate::states::EditorMode;

use super::gizmos::{
    closest_point_on_axis, perpendicular_basis, point_to_segment_distance, ray_intersect_plane,
    Axis,
};

// --- Transform mode ---

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformMode {
    #[default]
    Move,
    Rotate,
    Scale,
}

// --- Resources ---

#[derive(Resource)]
pub struct PlacementState {
    /// Obstacle selected in the palette for placing.
    pub selected_palette_id: Option<ObstacleId>,
    /// The placed entity currently selected for editing.
    pub selected_entity: Option<Entity>,
    /// Which transform gizmo is active.
    pub transform_mode: TransformMode,
    /// When true, LMB on any placed obstacle assigns the next gate order.
    pub gate_order_mode: bool,
    /// Auto-incrementing counter for gate order assignment.
    pub next_gate_order: u32,
    /// Course name for save path.
    pub course_name: String,
    /// Whether the course name text field has keyboard focus.
    pub editing_name: bool,
}

impl Default for PlacementState {
    fn default() -> Self {
        Self {
            selected_palette_id: None,
            selected_entity: None,
            transform_mode: TransformMode::default(),
            gate_order_mode: false,
            next_gate_order: 0,
            course_name: "new_course".to_string(),
            editing_name: false,
        }
    }
}

// --- Widget resources ---

#[derive(Resource, Default)]
struct MoveWidgetState {
    active_axis: Option<Axis>,
    hovered_axis: Option<Axis>,
    drag_offset: f32,
}

#[derive(Resource, Default)]
struct RotateWidgetState {
    active_axis: Option<Axis>,
    hovered_axis: Option<Axis>,
    drag_start_angle: f32,
    entity_start_rotation: Quat,
}

#[derive(Resource, Default)]
struct ScaleWidgetState {
    active_axis: Option<Axis>,
    hovered_axis: Option<Axis>,
    drag_start_t: f32,
    entity_start_scale: Vec3,
}

// --- Components ---

/// Marker on every obstacle entity spawned in the course editor.
#[derive(Component, Clone)]
pub struct PlacedObstacle {
    pub obstacle_id: ObstacleId,
    pub gate_order: Option<u32>,
}

// --- Gizmo group ---

#[derive(Default, Reflect, GizmoConfigGroup)]
struct CourseGizmoGroup;

// --- Constants ---

const ARROW_LENGTH: f32 = 2.5;
const ARROW_HIT_THRESHOLD: f32 = 25.0;

const RING_RADIUS: f32 = 2.0;
const RING_SAMPLES: usize = 32;
const RING_HIT_THRESHOLD: f32 = 15.0;

const SCALE_HANDLE_LENGTH: f32 = 2.5;
const SCALE_CUBE_SIZE: f32 = 0.3;
const SCALE_HIT_THRESHOLD: f32 = 25.0;
const SCALE_SENSITIVITY: f32 = 1.0;

// --- Plugin ---

pub struct CourseEditorPlugin;

impl Plugin for CourseEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<CourseGizmoGroup>()
            .add_systems(
                OnEnter(EditorMode::CourseEditor),
                (ensure_gltf_loaded, setup_course_editor),
            )
            .add_systems(
                Update,
                (
                    ui::handle_palette_selection,
                    ui::handle_back_to_workshop,
                    ui::handle_back_to_menu,
                    ui::handle_save_button,
                    ui::handle_load_button,
                    ui::handle_gate_order_toggle,
                    ui::handle_clear_gate_orders_button,
                    ui::handle_name_field_focus,
                    ui::handle_name_text_input,
                    ui::update_display_values,
                    ui::handle_button_hover,
                    ui::handle_transform_mode_buttons,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    ui::update_transform_mode_ui,
                    ui::update_gate_count_display,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    handle_placement_and_selection,
                    handle_delete_key,
                    handle_transform_mode_keys,
                    draw_trigger_gizmos,
                    draw_gate_sequence_lines,
                    draw_selection_highlight,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    draw_move_gizmo,
                    handle_move_gizmo,
                    draw_rotate_gizmo,
                    handle_rotate_gizmo,
                    draw_scale_gizmo,
                    handle_scale_gizmo,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(OnExit(EditorMode::CourseEditor), cleanup_course_editor);
    }
}

// --- Setup / Cleanup ---

fn ensure_gltf_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Option<Res<ObstaclesGltfHandle>>,
) {
    if existing.is_none() {
        let handle = asset_server.load("models/obstacles.glb");
        commands.insert_resource(ObstaclesGltfHandle(handle));
    }
}

fn setup_course_editor(
    mut commands: Commands,
    library: Res<ObstacleLibrary>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let existing_courses = ui::discover_existing_courses();
    ui::build_course_editor_ui(&mut commands, &library, &existing_courses);
    commands.insert_resource(PlacementState::default());
    commands.insert_resource(MoveWidgetState::default());
    commands.insert_resource(RotateWidgetState::default());
    commands.insert_resource(ScaleWidgetState::default());

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
}

fn cleanup_course_editor(
    mut commands: Commands,
    placed_query: Query<Entity, With<PlacedObstacle>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.remove_resource::<PlacementState>();
    commands.remove_resource::<MoveWidgetState>();
    commands.remove_resource::<RotateWidgetState>();
    commands.remove_resource::<ScaleWidgetState>();
    for entity in &placed_query {
        commands.entity(entity).despawn();
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = 0.0;
}

// --- Placement and Selection ---

fn handle_placement_and_selection(
    mut state: ResMut<PlacementState>,
    move_widget: Res<MoveWidgetState>,
    rotate_widget: Res<RotateWidgetState>,
    scale_widget: Res<ScaleWidgetState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut placed_query: Query<(Entity, &mut PlacedObstacle)>,
    parent_query: Query<&ChildOf>,
    interaction_query: Query<&Interaction>,
    mut ray_cast: MeshRayCast,
) {
    if state.editing_name {
        return;
    }

    // Don't process clicks when a gizmo drag is active
    if move_widget.active_axis.is_some()
        || rotate_widget.active_axis.is_some()
        || scale_widget.active_axis.is_some()
    {
        return;
    }

    let over_ui = interaction_query.iter().any(|i| *i != Interaction::None);
    if over_ui || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    let nearest = find_placed_ancestor_from_ray(
        &mut ray_cast,
        ray,
        &placed_query,
        &parent_query,
    );

    if state.gate_order_mode {
        if let Some(entity) = nearest {
            if let Ok((_, mut placed)) = placed_query.get_mut(entity) {
                let order = state.next_gate_order;
                placed.gate_order = Some(order);
                state.next_gate_order += 1;
                info!("Assigned gate order {order} to {:?}", entity);
            }
        }
        return;
    }

    if let Some(entity) = nearest {
        state.selected_entity = Some(entity);
    } else {
        state.selected_entity = None;
    }
}

fn handle_delete_key(
    mut commands: Commands,
    mut state: ResMut<PlacementState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.editing_name {
        return;
    }

    if keyboard.just_pressed(KeyCode::Delete) {
        if let Some(entity) = state.selected_entity.take() {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_transform_mode_keys(
    mut state: ResMut<PlacementState>,
    mut move_widget: ResMut<MoveWidgetState>,
    mut rotate_widget: ResMut<RotateWidgetState>,
    mut scale_widget: ResMut<ScaleWidgetState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if state.editing_name {
        return;
    }

    let new_mode = if keyboard.just_pressed(KeyCode::KeyG) {
        Some(TransformMode::Move)
    } else if keyboard.just_pressed(KeyCode::KeyR) {
        Some(TransformMode::Rotate)
    } else if keyboard.just_pressed(KeyCode::KeyS) {
        Some(TransformMode::Scale)
    } else {
        None
    };

    if let Some(mode) = new_mode {
        state.transform_mode = mode;
        move_widget.active_axis = None;
        move_widget.hovered_axis = None;
        rotate_widget.active_axis = None;
        rotate_widget.hovered_axis = None;
        scale_widget.active_axis = None;
        scale_widget.hovered_axis = None;
    }
}

// --- Move Gizmo ---

fn draw_move_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<MoveWidgetState>,
    placed_query: Query<&Transform, With<PlacedObstacle>>,
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

    let origin = transform.translation;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);
        let end = origin + axis.direction() * ARROW_LENGTH;
        gizmos.arrow(origin, end, color);
    }
}

fn handle_move_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    state: Res<PlacementState>,
    mut widget: ResMut<MoveWidgetState>,
    mut placed_query: Query<&mut Transform, With<PlacedObstacle>>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Move {
        widget.active_axis = None;
        widget.hovered_axis = None;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_axis = None;
        widget.hovered_axis = None;
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

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let t = closest_point_on_axis(ray, origin, axis_dir);
            let delta = t - widget.drag_offset;
            transform.translation = origin + axis_dir * delta;
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = ARROW_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let end = origin + axis.direction() * ARROW_LENGTH;
            let Ok(screen_start) = camera.world_to_viewport(camera_gt, origin) else { continue };
            let Ok(screen_end) = camera.world_to_viewport(camera_gt, end) else { continue };
            let dist = point_to_segment_distance(cursor_pos, screen_start, screen_end);
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
            let axis_dir = axis.direction();
            let t = closest_point_on_axis(ray, origin, axis_dir);
            widget.active_axis = Some(axis);
            widget.drag_offset = t;
        }
    }
}

// --- Rotate Gizmo ---

fn draw_rotate_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<RotateWidgetState>,
    placed_query: Query<&Transform, With<PlacedObstacle>>,
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

    let ring_orientations = [
        (Axis::X, Quat::from_rotation_arc(Vec3::Z, Vec3::X)),
        (Axis::Y, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
        (Axis::Z, Quat::IDENTITY),
    ];

    for (axis, face_quat) in ring_orientations {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);
        let iso = Isometry3d::new(pos, face_quat);
        gizmos.circle(iso, RING_RADIUS, color);
    }
}

fn handle_rotate_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    state: Res<PlacementState>,
    mut widget: ResMut<RotateWidgetState>,
    mut placed_query: Query<&mut Transform, With<PlacedObstacle>>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Rotate {
        widget.active_axis = None;
        widget.hovered_axis = None;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_axis = None;
        widget.hovered_axis = None;
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
    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                let current_angle = angle_in_ring_plane(hit, pos, active_axis);
                let delta = current_angle - widget.drag_start_angle;
                transform.rotation =
                    widget.entity_start_rotation * Quat::from_axis_angle(axis_dir, delta);
            }
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = RING_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let (ref1, ref2) = perpendicular_basis(axis);
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
            if min_dist < best_dist {
                best_dist = min_dist;
                best_axis = Some(axis);
            }
        }

        widget.hovered_axis = best_axis;

        if !mouse_over_ui
            && mouse_buttons.just_pressed(MouseButton::Left)
            && let Some(axis) = best_axis
        {
            let axis_dir = axis.direction();
            if let Some(hit) = ray_intersect_plane(ray, pos, axis_dir) {
                widget.active_axis = Some(axis);
                widget.drag_start_angle = angle_in_ring_plane(hit, pos, axis);
                widget.entity_start_rotation = transform.rotation;
            }
        }
    }
}

// --- Scale Gizmo ---

fn draw_scale_gizmo(
    mut gizmos: Gizmos,
    state: Res<PlacementState>,
    widget: Res<ScaleWidgetState>,
    placed_query: Query<&Transform, With<PlacedObstacle>>,
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

    let origin = transform.translation;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);
        let tip = origin + axis.direction() * SCALE_HANDLE_LENGTH;
        gizmos.line(origin, tip, color);
        let cube_transform =
            Transform::from_translation(tip).with_scale(Vec3::splat(SCALE_CUBE_SIZE));
        gizmos.cube(cube_transform, color);
    }
}

fn handle_scale_gizmo(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    state: Res<PlacementState>,
    mut widget: ResMut<ScaleWidgetState>,
    mut placed_query: Query<&mut Transform, With<PlacedObstacle>>,
    interaction_query: Query<&Interaction>,
) {
    if state.transform_mode != TransformMode::Scale {
        widget.active_axis = None;
        widget.hovered_axis = None;
        return;
    }
    let Some(entity) = state.selected_entity else {
        widget.active_axis = None;
        widget.hovered_axis = None;
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

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let current_t = closest_point_on_axis(ray, origin, axis_dir);
            let delta = current_t - widget.drag_start_t;
            match active_axis {
                Axis::X => {
                    transform.scale.x =
                        (widget.entity_start_scale.x + delta * SCALE_SENSITIVITY).max(0.01)
                }
                Axis::Y => {
                    transform.scale.y =
                        (widget.entity_start_scale.y + delta * SCALE_SENSITIVITY).max(0.01)
                }
                Axis::Z => {
                    transform.scale.z =
                        (widget.entity_start_scale.z + delta * SCALE_SENSITIVITY).max(0.01)
                }
            }
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = SCALE_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let tip = origin + axis.direction() * SCALE_HANDLE_LENGTH;
            let Ok(screen_start) = camera.world_to_viewport(camera_gt, origin) else { continue };
            let Ok(screen_end) = camera.world_to_viewport(camera_gt, tip) else { continue };
            let dist = point_to_segment_distance(cursor_pos, screen_start, screen_end);
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
            let axis_dir = axis.direction();
            widget.active_axis = Some(axis);
            widget.drag_start_t = closest_point_on_axis(ray, origin, axis_dir);
            widget.entity_start_scale = transform.scale;
        }
    }
}

// --- Existing Gizmos ---

fn draw_trigger_gizmos(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    trigger_query: Query<(&TriggerVolume, &GlobalTransform)>,
) {
    for (trigger, gt) in &trigger_query {
        let (parent_scale, parent_rotation, center) = gt.to_scale_rotation_translation();
        let size = trigger.half_extents * 2.0 * parent_scale;
        let transform = Transform {
            translation: center,
            rotation: parent_rotation,
            scale: size,
        };
        gizmos.cube(transform, Color::srgb(0.2, 1.0, 0.2));
    }
}

fn draw_gate_sequence_lines(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    placed_query: Query<(&PlacedObstacle, &GlobalTransform)>,
) {
    let mut gates: Vec<(u32, Vec3)> = placed_query
        .iter()
        .filter_map(|(placed, gt)| placed.gate_order.map(|order| (order, gt.translation())))
        .collect();

    gates.sort_by_key(|(order, _)| *order);

    let line_color = Color::srgb(1.0, 0.8, 0.0);

    for pair in gates.windows(2) {
        let (_, from) = pair[0];
        let (_, to) = pair[1];
        gizmos.line(from, to, line_color);
    }

    // Draw loop-closing line from last gate back to first gate
    if gates.len() >= 2 {
        let (_, first) = gates[0];
        let (_, last) = gates[gates.len() - 1];
        let loop_color = Color::srgb(0.4, 0.8, 1.0);
        gizmos.line(last, first, loop_color);
    }

    let count = gates.len();
    for (i, (_, pos)) in gates.iter().enumerate() {
        let t = if count > 1 {
            i as f32 / (count - 1) as f32
        } else {
            0.0
        };
        let color = Color::srgb(t, 1.0 - t * 0.7, 0.0);
        let iso = Isometry3d::new(*pos, Quat::IDENTITY);
        gizmos.sphere(iso, 0.5, color);
    }
}

fn draw_selection_highlight(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    state: Res<PlacementState>,
    placed_query: Query<&Transform, With<PlacedObstacle>>,
) {
    let Some(entity) = state.selected_entity else {
        return;
    };
    let Ok(transform) = placed_query.get(entity) else {
        return;
    };

    let center = transform.translation + Vec3::Y * 1.5;
    let hl_transform = Transform::from_translation(center).with_scale(Vec3::splat(3.5));
    gizmos.cube(hl_transform, Color::srgb(1.0, 1.0, 0.0));
}

// --- Helpers ---

/// Cast a ray into the scene and return the `PlacedObstacle` ancestor of the
/// nearest hit mesh, if any. Walks up the entity hierarchy from the hit child
/// to find the parent that carries the `PlacedObstacle` component.
fn find_placed_ancestor_from_ray(
    ray_cast: &mut MeshRayCast,
    ray: Ray3d,
    placed_query: &Query<(Entity, &mut PlacedObstacle)>,
    parent_query: &Query<&ChildOf>,
) -> Option<Entity> {
    let hits = ray_cast.cast_ray(ray, &MeshRayCastSettings::default());
    for (hit_entity, _) in hits {
        // Walk up the hierarchy from the hit mesh child to find the PlacedObstacle parent
        let mut current = *hit_entity;
        loop {
            if placed_query.get(current).is_ok() {
                return Some(current);
            }
            if let Ok(child_of) = parent_query.get(current) {
                current = child_of.parent();
            } else {
                break;
            }
        }
    }
    None
}

/// Sample a ring of `n` evenly-spaced world points and return the minimum
/// screen-space distance from `cursor_pos` to any of those projected points.
fn sample_ring_screen_dist(
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

/// Compute the signed angle (in radians) of a world-space `point` projected
/// onto the plane perpendicular to `axis` through `center`.
fn angle_in_ring_plane(point: Vec3, center: Vec3, axis: Axis) -> f32 {
    let local = point - center;
    let (ref1, ref2) = perpendicular_basis(axis);
    local.dot(ref2).atan2(local.dot(ref1))
}
