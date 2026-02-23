pub mod ui;

use bevy::{
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    prelude::*,
};

use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{spawn_obstacle, ObstaclesGltfHandle, TriggerVolume};
use crate::states::EditorMode;

// --- Resources ---

#[derive(Resource)]
pub struct PlacementState {
    /// Obstacle selected in the palette for placing.
    pub selected_palette_id: Option<ObstacleId>,
    /// The placed entity currently selected for dragging.
    pub selected_entity: Option<Entity>,
    /// Whether a drag is active this frame.
    pub drag_active: bool,
    /// XZ offset from entity origin to the initial ray hit, preserves grab point.
    pub drag_xz_offset: Vec2,
    /// Y level used when placing new obstacles.
    pub drag_height: f32,
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
            drag_active: false,
            drag_xz_offset: Vec2::ZERO,
            drag_height: 0.0,
            gate_order_mode: false,
            next_gate_order: 0,
            course_name: "new_course".to_string(),
            editing_name: false,
        }
    }
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
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    handle_placement_and_drag,
                    handle_height_key,
                    handle_delete_key,
                    draw_trigger_gizmos,
                    draw_gate_sequence_lines,
                    draw_selection_highlight,
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

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
}

fn cleanup_course_editor(
    mut commands: Commands,
    placed_query: Query<Entity, With<PlacedObstacle>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.remove_resource::<PlacementState>();
    for entity in &placed_query {
        commands.entity(entity).despawn();
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = 0.0;
}

// --- Placement, Selection, and Dragging ---

fn handle_placement_and_drag(
    mut commands: Commands,
    mut state: ResMut<PlacementState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut placed_query: Query<(Entity, &mut Transform, &mut PlacedObstacle)>,
    interaction_query: Query<&Interaction>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if state.editing_name {
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

    // Continue an active drag while LMB is held
    if state.drag_active {
        if mouse_buttons.pressed(MouseButton::Left) {
            let selected = state.selected_entity;
            let xz_offset = state.drag_xz_offset;
            if let Some(entity) = selected {
                if let Ok((_, mut transform, _)) = placed_query.get_mut(entity) {
                    let drag_y = transform.translation.y;
                    if let Some(hit) = ray_intersect_y_plane(ray, drag_y) {
                        transform.translation.x = hit.x + xz_offset.x;
                        transform.translation.z = hit.z + xz_offset.y;
                    }
                }
            }
        } else {
            state.drag_active = false;
        }
        return;
    }

    // Only process new clicks below
    let over_ui = interaction_query.iter().any(|i| *i != Interaction::None);
    if over_ui || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    // Find the nearest placed entity to the cursor in screen space
    let nearest = find_nearest_to_cursor(&placed_query, camera, camera_gt, cursor_pos);

    if state.gate_order_mode {
        // Assign the next gate order to the clicked obstacle
        if let Some(entity) = nearest {
            if let Ok((_, _, mut placed)) = placed_query.get_mut(entity) {
                let order = state.next_gate_order;
                placed.gate_order = Some(order);
                state.next_gate_order += 1;
                info!("Assigned gate order {order} to {:?}", entity);
            }
        }
        return;
    }

    if let Some(entity) = nearest {
        // Select and begin dragging the existing obstacle
        if let Ok((_, transform, _)) = placed_query.get(entity) {
            let entity_pos = transform.translation;
            let drag_y = entity_pos.y;
            let hit = ray_intersect_y_plane(ray, drag_y)
                .unwrap_or(Vec3::new(entity_pos.x, drag_y, entity_pos.z));
            state.drag_xz_offset = Vec2::new(entity_pos.x - hit.x, entity_pos.z - hit.z);
        }
        state.selected_entity = Some(entity);
        state.drag_active = true;
        state.selected_palette_id = None;
    } else if let Some(id) = state.selected_palette_id.clone() {
        // Place a new obstacle at the cursor's ground hit
        let Some(def) = library.get(&id) else {
            return;
        };
        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot place obstacle");
            return;
        };

        let height = state.drag_height;
        let Some(hit_pos) = ray_intersect_y_plane(ray, height) else {
            return;
        };

        let transform = Transform::from_translation(hit_pos);
        let spawned = spawn_obstacle(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut materials,
            handle,
            &def.id,
            &def.glb_node_name,
            transform,
            def.trigger_volume.as_ref(),
            None,
        );

        if let Some(entity) = spawned {
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: id.clone(),
                gate_order: None,
            });
        } else {
            warn!(
                "Failed to spawn obstacle '{}' (node '{}')",
                def.id.0, def.glb_node_name
            );
        }
    }
}

fn handle_height_key(
    mut state: ResMut<PlacementState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut placed_query: Query<&mut Transform, With<PlacedObstacle>>,
) {
    if state.editing_name {
        return;
    }

    const STEP: f32 = 0.5;

    if keyboard.just_pressed(KeyCode::KeyE) {
        state.drag_height += STEP;
        if let Some(entity) = state.selected_entity {
            if let Ok(mut transform) = placed_query.get_mut(entity) {
                transform.translation.y += STEP;
            }
        }
    }

    if keyboard.just_pressed(KeyCode::KeyQ) {
        state.drag_height = (state.drag_height - STEP).max(0.0);
        if let Some(entity) = state.selected_entity {
            if let Ok(mut transform) = placed_query.get_mut(entity) {
                transform.translation.y = (transform.translation.y - STEP).max(0.0);
            }
        }
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
            state.drag_active = false;
        }
    }
}

// --- Gizmos ---

fn draw_trigger_gizmos(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    trigger_query: Query<(&TriggerVolume, &GlobalTransform)>,
) {
    for (trigger, gt) in &trigger_query {
        let center = gt.translation();
        let size = trigger.half_extents * 2.0;
        let transform = Transform::from_translation(center).with_scale(size);
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

    // Draw lines between consecutive gates
    for pair in gates.windows(2) {
        let (_, from) = pair[0];
        let (_, to) = pair[1];
        gizmos.line(from, to, line_color);
    }

    // Draw a sphere at each gate to mark position and show order via color gradient
    let count = gates.len();
    for (i, (_, pos)) in gates.iter().enumerate() {
        let t = if count > 1 {
            i as f32 / (count - 1) as f32
        } else {
            0.0
        };
        // Gradient: green (first) → red (last)
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

    // Wireframe box centered slightly above entity origin
    let center = transform.translation + Vec3::Y * 1.5;
    let hl_transform = Transform::from_translation(center).with_scale(Vec3::splat(3.5));
    gizmos.cube(hl_transform, Color::srgb(1.0, 1.0, 0.0));
}

// --- Helpers ---

/// Intersect a ray with a horizontal plane at height `y`.
/// Returns `None` if the ray is nearly parallel to the plane or misses it.
fn ray_intersect_y_plane(ray: Ray3d, y: f32) -> Option<Vec3> {
    let dir_y = ray.direction.y;
    if dir_y.abs() < 1e-6 {
        return None;
    }
    let t = (y - ray.origin.y) / dir_y;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + *ray.direction * t)
}

/// Find the placed entity whose world-space center is closest to the cursor
/// in screen space. Returns `None` if nothing is within the pick threshold.
fn find_nearest_to_cursor(
    placed_query: &Query<(Entity, &mut Transform, &mut PlacedObstacle)>,
    camera: &Camera,
    camera_gt: &GlobalTransform,
    cursor_pos: Vec2,
) -> Option<Entity> {
    const THRESHOLD_PX: f32 = 50.0;

    let mut best_entity = None;
    let mut best_dist = THRESHOLD_PX;

    for (entity, transform, _) in placed_query.iter() {
        let world_pos = transform.translation;
        let Ok(screen_pos) = camera.world_to_viewport(camera_gt, world_pos) else {
            continue;
        };
        let dist = (cursor_pos - screen_pos).length();
        if dist < best_dist {
            best_dist = dist;
            best_entity = Some(entity);
        }
    }

    best_entity
}
