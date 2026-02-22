pub mod ui;

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};

use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::states::EditorMode;

#[derive(Resource)]
pub struct WorkshopState {
    pub obstacle_name: String,
    pub node_name: String,
    pub is_gate: bool,
    pub has_trigger: bool,
    pub trigger_offset: Vec3,
    pub trigger_half_extents: Vec3,
    pub preview_entity: Option<Entity>,
    pub available_nodes: Vec<String>,
    pub nodes_loaded: bool,
    pub editing_name: bool,
    pub edit_target: EditTarget,
    pub model_offset: Vec3,
}

impl Default for WorkshopState {
    fn default() -> Self {
        Self {
            obstacle_name: String::new(),
            node_name: String::new(),
            is_gate: true,
            has_trigger: true,
            trigger_offset: Vec3::new(0.0, 1.0, 0.0),
            trigger_half_extents: Vec3::new(2.0, 2.0, 0.5),
            preview_entity: None,
            available_nodes: Vec::new(),
            nodes_loaded: false,
            editing_name: false,
            edit_target: EditTarget::default(),
            model_offset: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum EditTarget {
    #[default]
    Model,
    Trigger,
}

#[derive(Component)]
pub struct PreviewObstacle;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn direction(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    fn color(self, hovered: bool, active: bool) -> Color {
        let brightness = if active {
            1.0
        } else if hovered {
            0.8
        } else {
            0.5
        };
        match self {
            Axis::X => Color::srgb(brightness, 0.0, 0.0),
            Axis::Y => Color::srgb(0.0, brightness, 0.0),
            Axis::Z => Color::srgb(0.0, 0.0, brightness),
        }
    }
}

#[derive(Resource)]
struct MoveWidgetState {
    active_axis: Option<Axis>,
    hovered_axis: Option<Axis>,
    drag_offset: f32,
}

impl Default for MoveWidgetState {
    fn default() -> Self {
        Self {
            active_axis: None,
            hovered_axis: None,
            drag_offset: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sign {
    Positive,
    Negative,
}

#[derive(Resource, Default)]
struct ResizeWidgetState {
    active_handle: Option<(Axis, Sign)>,
    hovered_handle: Option<(Axis, Sign)>,
}

const ARROW_LENGTH: f32 = 2.5;
const ARROW_HIT_THRESHOLD: f32 = 25.0; // pixels
const HANDLE_SIZE: f32 = 0.2;
const HANDLE_HIT_THRESHOLD: f32 = 20.0; // pixels

pub struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(EditorMode::ObstacleWorkshop),
            (load_gltf_for_workshop, setup_workshop),
        )
        .add_systems(
            Update,
            (
                populate_node_list,
                ui::handle_node_selection,
                ui::handle_library_selection,
                ui::handle_trigger_toggle,
                ui::handle_save_button,
                ui::handle_new_button,
                ui::handle_delete_button,
                ui::handle_back_button,
                ui::handle_switch_to_course_editor,
            )
                .run_if(in_state(EditorMode::ObstacleWorkshop)),
        )
        .add_systems(
            Update,
            (
                ui::handle_name_text_input,
                ui::handle_name_field_focus,
                ui::handle_edit_target_toggle,
                ui::update_display_values,
                ui::handle_button_hover,
                spawn_placeholder_preview,
                draw_trigger_gizmo,
            )
                .run_if(in_state(EditorMode::ObstacleWorkshop)),
        )
        .add_systems(
            Update,
            (
                workshop_camera,
                draw_ground_gizmo,
                draw_move_arrows,
                handle_move_widget,
                draw_resize_handles,
                handle_resize_widget,
            )
                .run_if(in_state(EditorMode::ObstacleWorkshop)),
        )
        .add_systems(OnExit(EditorMode::ObstacleWorkshop), cleanup_workshop);
    }
}

fn load_gltf_for_workshop(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Option<Res<ObstaclesGltfHandle>>,
) {
    if existing.is_none() {
        let handle = asset_server.load("models/obstacles.glb");
        commands.insert_resource(ObstaclesGltfHandle(handle));
    }
}

fn setup_workshop(mut commands: Commands, library: Res<ObstacleLibrary>) {
    let state = WorkshopState::default();
    ui::build_workshop_ui(&mut commands, &library);
    commands.insert_resource(state);
    commands.insert_resource(MoveWidgetState::default());
    commands.insert_resource(ResizeWidgetState::default());
}

fn cleanup_workshop(
    mut commands: Commands,
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    commands.remove_resource::<WorkshopState>();
    commands.remove_resource::<MoveWidgetState>();
    commands.remove_resource::<ResizeWidgetState>();
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }
}

fn populate_node_list(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    container_query: Query<Entity, With<ui::NodeListContainer>>,
) {
    if state.nodes_loaded {
        return;
    }
    let Some(handle) = gltf_handle else { return };
    let Some(gltf) = gltf_assets.get(&handle.0) else {
        return;
    };

    let Ok(container) = container_query.single() else {
        return;
    };

    let mut nodes: Vec<String> = gltf.named_nodes.keys().map(|k| k.to_string()).collect();
    nodes.sort();
    state.available_nodes = nodes.clone();
    state.nodes_loaded = true;

    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        if nodes.is_empty() {
            parent.spawn((
                Text::new("No objects found in glb"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        } else {
            for node in &nodes {
                ui::spawn_node_button(parent, node);
            }
        }
    });

    info!("Found {} named nodes in obstacles.glb", nodes.len());
}

/// Spawn a preview from a named node in the glTF, positioned at `model_offset`.
pub fn spawn_preview(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    materials: &mut Assets<StandardMaterial>,
    gltf_handle: &ObstaclesGltfHandle,
    node_name: &str,
    model_offset: Vec3,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    let mut transform = node.transform;
    transform.translation = model_offset;

    let parent = commands
        .spawn((
            transform,
            Visibility::default(),
            PreviewObstacle,
        ))
        .id();

    for primitive in &gltf_mesh.primitives {
        let material = match primitive.material {
            Some(ref mat) => MeshMaterial3d(mat.clone()),
            None => MeshMaterial3d(materials.add(StandardMaterial::default())),
        };

        commands
            .spawn((
                Mesh3d(primitive.mesh.clone()),
                material,
                Transform::default(),
            ))
            .set_parent_in_place(parent);
    }

    Some(parent)
}

fn spawn_placeholder_preview(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
) {
    if state.node_name.is_empty() || state.preview_entity.is_some() {
        return;
    }

    let offset = state.model_offset;

    // Try to spawn from glTF first
    if let Some(handle) = &gltf_handle {
        if let Some(entity) = spawn_preview(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut materials,
            handle,
            &state.node_name,
            offset,
        ) {
            state.preview_entity = Some(entity);
            return;
        }
    }

    // No matching glTF node — spawn a placeholder cube
    let entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.6),
                ..default()
            })),
            Transform::from_translation(offset),
            PreviewObstacle,
        ))
        .id();
    state.preview_entity = Some(entity);
}

fn draw_trigger_gizmo(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if !state.has_trigger || state.node_name.is_empty() {
        return;
    }

    let preview_pos = state
        .preview_entity
        .and_then(|e| preview_query.get(e).ok())
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let color = if state.is_gate {
        Color::srgb(0.2, 1.0, 0.2)
    } else {
        Color::srgb(1.0, 0.8, 0.2)
    };

    let center = preview_pos + state.trigger_offset;
    let size = state.trigger_half_extents * 2.0;
    let transform = Transform::from_translation(center).with_scale(size);

    gizmos.cube(transform, color);
}

// --- Workshop Camera ---

fn workshop_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    state: Res<WorkshopState>,
    mut query: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Right-click drag for rotation
    if mouse_buttons.pressed(MouseButton::Right) {
        let delta = mouse_motion.delta;
        if delta != Vec2::ZERO {
            let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            let new_yaw = yaw - delta.x * 0.003;
            let new_pitch = (pitch - delta.y * 0.003).clamp(-1.5, 1.5);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);
        }
    }

    // Scroll wheel for zoom (dolly along forward vector)
    let scroll_y = mouse_scroll.delta.y;
    if scroll_y != 0.0 {
        let forward = transform.forward().as_vec3();
        let zoom_speed = 3.0;
        let new_pos = transform.translation + forward * scroll_y * zoom_speed;
        // Clamp Y to stay above ground
        if new_pos.y > 0.5 {
            transform.translation = new_pos;
        }
    }

    // WASD for ground-plane movement (skip when editing text fields)
    if state.editing_name {
        return;
    }

    let speed = 20.0;
    let dt = time.delta_secs();

    let cam_forward = transform.forward().as_vec3();
    let cam_right = transform.right().as_vec3();

    // Project onto XZ plane
    let ground_forward = Vec3::new(cam_forward.x, 0.0, cam_forward.z).normalize_or_zero();
    let ground_right = Vec3::new(cam_right.x, 0.0, cam_right.z).normalize_or_zero();

    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        movement += ground_forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement -= ground_forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement -= ground_right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement += ground_right;
    }

    if movement != Vec3::ZERO {
        transform.translation += movement.normalize() * speed * dt;
    }
}

// --- Ground Center Gizmo ---

fn draw_ground_gizmo(mut gizmos: Gizmos, state: Res<WorkshopState>) {
    if state.preview_entity.is_none() {
        return;
    }

    // Fixed at the world origin — this is the obstacle's ground center
    // that will be used as the placement anchor in the course editor.
    let ground_pos = Vec3::ZERO;
    let magenta = Color::srgb(1.0, 0.0, 1.0);

    let iso = Isometry3d::new(ground_pos, Quat::IDENTITY);
    gizmos.circle(iso, 0.5, magenta);

    let cross_size = 0.4;
    gizmos.line(
        ground_pos + Vec3::new(-cross_size, 0.0, 0.0),
        ground_pos + Vec3::new(cross_size, 0.0, 0.0),
        magenta,
    );
    gizmos.line(
        ground_pos + Vec3::new(0.0, 0.0, -cross_size),
        ground_pos + Vec3::new(0.0, 0.0, cross_size),
        magenta,
    );
}

// --- Move Widget (3D Axis Arrows) ---

fn move_arrow_origin(state: &WorkshopState, preview_query: &Query<&Transform, With<PreviewObstacle>>) -> Option<Vec3> {
    let entity = state.preview_entity?;
    let transform = preview_query.get(entity).ok()?;
    match state.edit_target {
        EditTarget::Model => Some(transform.translation),
        EditTarget::Trigger => {
            if state.has_trigger {
                Some(transform.translation + state.trigger_offset)
            } else {
                None
            }
        }
    }
}

fn draw_move_arrows(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    widget: Res<MoveWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    let Some(origin) = move_arrow_origin(&state, &preview_query) else {
        return;
    };

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let is_hovered = widget.hovered_axis == Some(axis);
        let is_active = widget.active_axis == Some(axis);
        let color = axis.color(is_hovered, is_active);

        let end = origin + axis.direction() * ARROW_LENGTH;
        gizmos.arrow(origin, end, color);
    }
}

fn handle_move_widget(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut preview_query: Query<&mut Transform, With<PreviewObstacle>>,
    mut state: ResMut<WorkshopState>,
    mut widget: ResMut<MoveWidgetState>,
    interaction_query: Query<&Interaction>,
) {
    let Some(preview_entity) = state.preview_entity else {
        widget.hovered_axis = None;
        widget.active_axis = None;
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };
    let Ok(mut preview_transform) = preview_query.get_mut(preview_entity) else {
        return;
    };

    // Determine arrow origin based on edit target
    let origin = match state.edit_target {
        EditTarget::Model => preview_transform.translation,
        EditTarget::Trigger => {
            if !state.has_trigger {
                widget.hovered_axis = None;
                widget.active_axis = None;
                return;
            }
            preview_transform.translation + state.trigger_offset
        }
    };

    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    if let Some(active_axis) = widget.active_axis {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let t = closest_point_on_axis(ray, origin, axis_dir);
            let delta = t - widget.drag_offset;

            match state.edit_target {
                EditTarget::Model => {
                    let new_pos = origin + axis_dir * delta;
                    preview_transform.translation = new_pos;
                    state.model_offset = new_pos;
                }
                EditTarget::Trigger => {
                    state.trigger_offset += axis_dir * delta;
                }
            }
        } else {
            widget.active_axis = None;
        }
    } else {
        let mut best_axis: Option<Axis> = None;
        let mut best_dist = ARROW_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            let end = origin + axis.direction() * ARROW_LENGTH;

            let Ok(screen_start) = camera.world_to_viewport(camera_gt, origin) else {
                continue;
            };
            let Ok(screen_end) = camera.world_to_viewport(camera_gt, end) else {
                continue;
            };

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

// --- Resize Handles for Trigger Volume ---

fn trigger_box_center(state: &WorkshopState, preview_query: &Query<&Transform, With<PreviewObstacle>>) -> Option<Vec3> {
    let entity = state.preview_entity?;
    let transform = preview_query.get(entity).ok()?;
    Some(transform.translation + state.trigger_offset)
}

fn draw_resize_handles(
    mut gizmos: Gizmos,
    state: Res<WorkshopState>,
    resize: Res<ResizeWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
) {
    if state.edit_target != EditTarget::Trigger || !state.has_trigger {
        return;
    }
    let Some(center) = trigger_box_center(&state, &preview_query) else {
        return;
    };

    let he = state.trigger_half_extents;

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        for sign in [Sign::Positive, Sign::Negative] {
            let dir = axis.direction() * match sign {
                Sign::Positive => 1.0,
                Sign::Negative => -1.0,
            };
            let extent = match axis {
                Axis::X => he.x,
                Axis::Y => he.y,
                Axis::Z => he.z,
            };
            let pos = center + dir * extent;

            let is_hovered = resize.hovered_handle == Some((axis, sign));
            let is_active = resize.active_handle == Some((axis, sign));
            let base = axis.color(is_hovered, is_active);
            // Lighten the color for resize handles to distinguish from move arrows
            let color = if is_active || is_hovered {
                base
            } else {
                match axis {
                    Axis::X => Color::srgb(0.7, 0.3, 0.3),
                    Axis::Y => Color::srgb(0.3, 0.7, 0.3),
                    Axis::Z => Color::srgb(0.3, 0.3, 0.7),
                }
            };

            let transform = Transform::from_translation(pos).with_scale(Vec3::splat(HANDLE_SIZE * 2.0));
            gizmos.cube(transform, color);
        }
    }
}

fn handle_resize_widget(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut state: ResMut<WorkshopState>,
    mut resize: ResMut<ResizeWidgetState>,
    move_widget: Res<MoveWidgetState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
    interaction_query: Query<&Interaction>,
) {
    if state.edit_target != EditTarget::Trigger || !state.has_trigger {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    }

    let Some(center) = trigger_box_center(&state, &preview_query) else {
        resize.hovered_handle = None;
        resize.active_handle = None;
        return;
    };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_gt)) = camera_query.single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else { return };

    let mouse_over_ui = interaction_query.iter().any(|i| *i != Interaction::None);
    let he = state.trigger_half_extents;

    if let Some((active_axis, active_sign)) = resize.active_handle {
        if mouse_buttons.pressed(MouseButton::Left) {
            let axis_dir = active_axis.direction();
            let sign_f = match active_sign {
                Sign::Positive => 1.0,
                Sign::Negative => -1.0,
            };
            // Project cursor ray onto the axis to get new extent
            let t = closest_point_on_axis(ray, center, axis_dir);
            let new_extent = (t * sign_f).max(0.1);
            match active_axis {
                Axis::X => state.trigger_half_extents.x = new_extent,
                Axis::Y => state.trigger_half_extents.y = new_extent,
                Axis::Z => state.trigger_half_extents.z = new_extent,
            }
        } else {
            resize.active_handle = None;
        }
    } else {
        // Don't process resize hover/click if move widget is active
        if move_widget.active_axis.is_some() {
            resize.hovered_handle = None;
            return;
        }

        let mut best: Option<(Axis, Sign)> = None;
        let mut best_dist = HANDLE_HIT_THRESHOLD;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for sign in [Sign::Positive, Sign::Negative] {
                let dir = axis.direction() * match sign {
                    Sign::Positive => 1.0,
                    Sign::Negative => -1.0,
                };
                let extent = match axis {
                    Axis::X => he.x,
                    Axis::Y => he.y,
                    Axis::Z => he.z,
                };
                let handle_pos = center + dir * extent;
                let Ok(screen_pos) = camera.world_to_viewport(camera_gt, handle_pos) else {
                    continue;
                };
                let dist = (cursor_pos - screen_pos).length();
                if dist < best_dist {
                    best_dist = dist;
                    best = Some((axis, sign));
                }
            }
        }

        resize.hovered_handle = best;

        if !mouse_over_ui
            && mouse_buttons.just_pressed(MouseButton::Left)
            && let Some((axis, sign)) = best
        {
            resize.active_handle = Some((axis, sign));
        }
    }
}

/// Project the camera ray onto a world-space axis through `origin` and return the parameter `t`
/// such that `origin + axis_dir * t` is the closest point on the axis to the ray.
fn closest_point_on_axis(ray: Ray3d, origin: Vec3, axis_dir: Vec3) -> f32 {
    let ray_origin = ray.origin;
    let ray_dir = *ray.direction;
    let w = ray_origin - origin;

    let a = ray_dir.dot(ray_dir);
    let b = ray_dir.dot(axis_dir);
    let c = axis_dir.dot(axis_dir);
    let d = ray_dir.dot(w);
    let e = axis_dir.dot(w);

    let denom = a * c - b * b;
    if denom.abs() < 1e-6 {
        return 0.0;
    }

    (a * e - b * d) / denom
}

/// Distance from a point to a line segment in 2D screen space.
fn point_to_segment_distance(point: Vec2, seg_start: Vec2, seg_end: Vec2) -> f32 {
    let ab = seg_end - seg_start;
    let ap = point - seg_start;
    let len_sq = ab.length_squared();
    if len_sq < 1e-6 {
        return ap.length();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = seg_start + ab * t;
    (point - proj).length()
}
