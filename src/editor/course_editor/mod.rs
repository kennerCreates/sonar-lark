pub mod ui;

mod overlays;
mod preview;
mod transform_gizmos;
mod undo_redo;

use bevy::{
    asset::AssetEvent,
    camera::visibility::RenderLayers,
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
    prelude::*,
};

use crate::camera::orbit::MainCamera;

use crate::course::data::{CourseData, PropKind};
use crate::course::location::GateInventory;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{ObstaclesGltfHandle, obstacles_gltf_ready};
use crate::editor::types::EditorTab;
use crate::states::{EditorMode, PendingEditorCourse};
use crate::ui_theme::UiFont;

use crate::editor::undo::{CourseEditorAction, UndoStack};
use transform_gizmos::{MoveWidgetState, RotateWidgetState, ScaleWidgetState};

// --- Transform mode ---

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformMode {
    #[default]
    Move,
    Rotate,
    Scale,
}

pub const DEFAULT_COURSE_NAME: &str = "new_course";

// --- Resources ---

#[derive(Resource, Default)]
pub struct EditorSelection {
    pub palette_id: Option<ObstacleId>,
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct EditorTransform {
    pub mode: TransformMode,
    pub gate_order_mode: bool,
    pub next_gate_order: u32,
}

#[derive(Resource)]
pub struct EditorCourse {
    pub name: String,
    pub location_index: usize,
    pub inventory: GateInventory,
}

impl Default for EditorCourse {
    fn default() -> Self {
        Self {
            name: DEFAULT_COURSE_NAME.to_string(),
            location_index: 0,
            inventory: GateInventory::default(),
        }
    }
}

#[derive(Resource, Default)]
pub struct EditorUI {
    pub active_tab: EditorTab,
}

/// Inserted when the obstacles glTF is hot-reloaded. Holds a snapshot of placed
/// obstacles so they can be respawned once the new asset data is available.
#[derive(Resource)]
struct PendingGlbReload {
    course_data: CourseData,
}

// --- Components ---

/// Marker on every obstacle entity spawned in the course editor.
#[derive(Component, Clone)]
pub struct PlacedObstacle {
    pub obstacle_id: ObstacleId,
    pub gate_order: Option<u32>,
    pub gate_forward_flipped: bool,
    pub color_override: Option<[f32; 4]>,
    /// True if this gate was placed from inventory (free), false if purchased with money.
    pub from_inventory: bool,
}

/// Marker on every prop entity spawned in the course editor.
#[derive(Component, Clone)]
pub struct PlacedProp {
    pub kind: PropKind,
    pub color_override: Option<[f32; 4]>,
}

/// Marker on every camera entity spawned in the course editor.
#[derive(Component, Clone)]
pub struct PlacedCamera {
    pub is_primary: bool,
    pub label: Option<String>,
}

/// Query filter matching any editor-placed entity (obstacle, prop, or camera).
pub(crate) type PlacedFilter = Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>;

/// Resets editor state to defaults: clears selection, resets course name,
/// and disables gate-order mode.
pub fn reset_editor_to_default(
    selection: &mut EditorSelection,
    course_state: &mut EditorCourse,
    transform_state: &mut EditorTransform,
) {
    selection.entity = None;
    selection.palette_id = None;
    course_state.name = DEFAULT_COURSE_NAME.to_string();
    transform_state.gate_order_mode = false;
    transform_state.next_gate_order = 0;
}

// --- Plugin ---

pub struct CourseEditorPlugin;

impl Plugin for CourseEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<overlays::CourseGizmoGroup>()
            .add_systems(
                OnEnter(EditorMode::CourseEditor),
                (
                    ensure_gltf_loaded,
                    setup_course_editor,
                    ui::setup_prop_editor_meshes,
                    ui::setup_camera_editor_meshes,
                    preview::setup_camera_preview,
                ),
            )
            .add_systems(
                Update,
                (
                    ui::handle_palette_selection,
                    ui::handle_prop_palette_selection,
                    ui::handle_tab_switch,
                    ui::handle_back_to_menu,
                    ui::handle_save_button,
                    ui::handle_start_race,
                    ui::handle_gate_order_toggle,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    ui::handle_clear_gate_orders_button,
                    ui::update_display_values,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    ui::handle_button_hover,
                    ui::handle_transform_mode_buttons,
                    ui::handle_prop_color_cycle,
                    ui::update_prop_color_label,
                    ui::handle_gate_color_click,
                    ui::handle_gate_color_default,
                    ui::update_gate_color_label,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                ui::auto_load_pending_course
                    .run_if(in_state(EditorMode::CourseEditor))
                    .run_if(resource_exists::<PendingEditorCourse>)
                    .run_if(obstacles_gltf_ready),
            )
            .add_systems(
                Update,
                (
                    ui::update_transform_mode_ui,
                    ui::update_gate_count_display,
                    ui::update_money_display,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    transform_gizmos::draw_move_gizmo,
                    transform_gizmos::handle_move_gizmo,
                    transform_gizmos::draw_rotate_gizmo,
                    transform_gizmos::handle_rotate_gizmo,
                    transform_gizmos::draw_scale_gizmo,
                    transform_gizmos::handle_scale_gizmo,
                )
                    .before(handle_placement_and_selection)
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                undo_redo::handle_course_undo_input
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    handle_placement_and_selection,
                    handle_delete_key,
                    handle_transform_mode_keys,
                    handle_flip_gate_key,
                    overlays::draw_gate_sequence_lines,
                    overlays::draw_gate_forward_arrows,
                    overlays::draw_flight_spline_preview,
                    overlays::draw_prop_gizmos,
                    overlays::draw_camera_gizmos,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                preview::sync_preview_camera.run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                preview::save_thumbnail_when_ready
                    .run_if(in_state(EditorMode::CourseEditor))
                    .run_if(resource_exists::<ui::PendingThumbnailSave>),
            )
            .add_systems(
                Update,
                ui::check_pending_race_transition
                    .run_if(in_state(EditorMode::CourseEditor))
                    .run_if(resource_exists::<ui::PendingRaceTransition>)
                    .run_if(not(resource_exists::<ui::PendingThumbnailSave>)),
            )
            .add_systems(
                Update,
                snapshot_and_despawn_on_glb_reload
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                respawn_after_glb_reload
                    .run_if(in_state(EditorMode::CourseEditor))
                    .run_if(resource_exists::<PendingGlbReload>)
                    .run_if(obstacles_gltf_ready),
            )
            .add_systems(
                OnExit(EditorMode::CourseEditor),
                (cleanup_course_editor, preview::cleanup_camera_preview),
            );
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
    font: Res<UiFont>,
    mut config_store: ResMut<GizmoConfigStore>,
    selected_location: Option<Res<crate::course::location::SelectedLocation>>,
    location_registry: Res<crate::course::location::LocationRegistry>,
) {
    ui::build_course_editor_ui(&mut commands, &library, &font.0);
    commands.insert_resource(EditorSelection::default());
    commands.insert_resource(EditorTransform::default());
    let location_index = selected_location.as_ref().map_or(1, |l| l.0); // default to Warehouse (index 1)
    let location_name = location_registry
        .locations
        .get(location_index)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "Abandoned Warehouse".to_string());
    commands.insert_resource(EditorCourse {
        name: location_name,
        location_index,
        inventory: GateInventory::default(),
    });
    commands.insert_resource(EditorUI::default());
    commands.insert_resource(MoveWidgetState::default());
    commands.insert_resource(RotateWidgetState::default());
    commands.insert_resource(ScaleWidgetState::default());
    commands.insert_resource(UndoStack::<CourseEditorAction>::default());

    // Gizmos on layer 1 so only the main camera (layers 0+1) sees them,
    // not the camera preview render-to-texture camera (layer 0 only).
    let gizmo_layers = RenderLayers::layer(1);

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
    config.line.width = 3.0;
    config.render_layers = gizmo_layers.clone();

    let (course_config, _) = config_store.config_mut::<overlays::CourseGizmoGroup>();
    course_config.render_layers = gizmo_layers;
}

fn cleanup_course_editor(
    mut commands: Commands,
    placed_query: Query<Entity, PlacedFilter>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.remove_resource::<EditorSelection>();
    commands.remove_resource::<EditorTransform>();
    commands.remove_resource::<EditorCourse>();
    commands.remove_resource::<EditorUI>();
    commands.remove_resource::<MoveWidgetState>();
    commands.remove_resource::<RotateWidgetState>();
    commands.remove_resource::<ScaleWidgetState>();
    commands.remove_resource::<UndoStack<CourseEditorAction>>();
    commands.remove_resource::<PendingEditorCourse>();
    commands.remove_resource::<PendingGlbReload>();
    commands.remove_resource::<ui::PropEditorMeshes>();
    commands.remove_resource::<ui::CameraEditorMeshes>();
    commands.remove_resource::<ui::PendingRaceTransition>();
    for entity in &placed_query {
        // Use try_despawn because PlacedCamera entities are children of
        // PlacedObstacle entities — recursive despawn of the parent already
        // removes the child, so it may be gone by the time we iterate to it.
        commands.entity(entity).try_despawn();
    }

    let default_layers = RenderLayers::default();

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = 0.0;
    config.line.width = 2.0;
    config.render_layers = default_layers.clone();

    let (course_config, _) = config_store.config_mut::<overlays::CourseGizmoGroup>();
    course_config.render_layers = default_layers;
}

// --- Placement and Selection ---

fn handle_placement_and_selection(
    mut selection: ResMut<EditorSelection>,
    mut transform_state: ResMut<EditorTransform>,
    move_widget: Res<MoveWidgetState>,
    rotate_widget: Res<RotateWidgetState>,
    scale_widget: Res<ScaleWidgetState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut obstacle_query: Query<(Entity, &mut PlacedObstacle)>,
    prop_query: Query<Entity, With<PlacedProp>>,
    camera_entity_query: Query<Entity, With<PlacedCamera>>,
    parent_query: Query<&ChildOf>,
    interaction_query: Query<&Interaction>,
    mut ray_cast: MeshRayCast,
) {
    // Don't process clicks when a gizmo is hovered or being dragged
    if move_widget.active_drag.is_some()
        || move_widget.hovered_part.is_some()
        || rotate_widget.active
        || rotate_widget.hovered
        || scale_widget.active_drag.is_some()
        || scale_widget.hovered_axis.is_some()
        || scale_widget.hovered_center
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
        &obstacle_query,
        &prop_query,
        &camera_entity_query,
        &parent_query,
    );

    if transform_state.gate_order_mode {
        if let Some(entity) = nearest
            && let Ok((_, mut placed)) = obstacle_query.get_mut(entity)
        {
            let order = transform_state.next_gate_order;
            placed.gate_order = Some(order);
            transform_state.next_gate_order += 1;
            info!("Assigned gate order {order} to {:?}", entity);
        }
        return;
    }

    if let Some(entity) = nearest {
        selection.entity = Some(entity);
    } else {
        selection.entity = None;
    }
}

fn handle_delete_key(
    mut commands: Commands,
    mut selection: ResMut<EditorSelection>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
    obstacle_query: Query<&PlacedObstacle>,
    prop_query: Query<&PlacedProp, Without<PlacedObstacle>>,
    camera_query: Query<&PlacedCamera, (Without<PlacedObstacle>, Without<PlacedProp>)>,
    transform_query: Query<&Transform, PlacedFilter>,
    camera_child_query: Query<(Entity, &ChildOf, &PlacedCamera, &Transform)>,
    library: Res<ObstacleLibrary>,
    mut course_state: ResMut<EditorCourse>,
) {
    if keyboard.just_pressed(KeyCode::Delete)
        && let Some(entity) = selection.entity.take()
    {
        // Store gate in inventory (instead of refunding money)
        if let Ok(placed) = obstacle_query.get(entity) {
            let cost = crate::course::data::gate_cost(&placed.obstacle_id.0, &library);
            if cost > 0 {
                course_state.inventory.add(&placed.obstacle_id);
            }
        }

        if let Some(action) = undo_redo::snapshot_for_delete(
            entity,
            &obstacle_query,
            &prop_query,
            &camera_query,
            &transform_query,
            &camera_child_query,
        ) {
            undo_stack.push(action);
        }
        commands.entity(entity).despawn();
    }
}

fn handle_transform_mode_keys(
    mut transform_state: ResMut<EditorTransform>,
    mut move_widget: ResMut<MoveWidgetState>,
    mut rotate_widget: ResMut<RotateWidgetState>,
    mut scale_widget: ResMut<ScaleWidgetState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let new_mode = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(TransformMode::Move)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(TransformMode::Rotate)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(TransformMode::Scale)
    } else {
        None
    };

    if let Some(mode) = new_mode {
        transform_state.mode = mode;
        move_widget.active_drag = None;
        move_widget.hovered_part = None;
        rotate_widget.active = false;
        rotate_widget.hovered = false;
        scale_widget.active_drag = None;
        scale_widget.hovered_axis = None;
        scale_widget.hovered_center = false;
    }
}

fn handle_flip_gate_key(
    selection: Res<EditorSelection>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut placed_query: Query<&mut PlacedObstacle>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    let Some(entity) = selection.entity else {
        return;
    };
    let Ok(mut placed) = placed_query.get_mut(entity) else {
        return;
    };
    if placed.gate_order.is_none() {
        return;
    }
    undo_stack.push(CourseEditorAction::FlipGate { entity });
    placed.gate_forward_flipped = !placed.gate_forward_flipped;
    info!(
        "Gate direction flipped (now {})",
        if placed.gate_forward_flipped { "flipped" } else { "default" }
    );
}

// --- Helpers ---

/// Cast a ray into the scene and return the placed entity ancestor of the
/// nearest hit mesh, if any. Walks up the entity hierarchy from the hit child
/// to find the parent that carries `PlacedObstacle` or `PlacedProp`.
fn find_placed_ancestor_from_ray(
    ray_cast: &mut MeshRayCast,
    ray: Ray3d,
    obstacle_query: &Query<(Entity, &mut PlacedObstacle)>,
    prop_query: &Query<Entity, With<PlacedProp>>,
    camera_query: &Query<Entity, With<PlacedCamera>>,
    parent_query: &Query<&ChildOf>,
) -> Option<Entity> {
    let hits = ray_cast.cast_ray(ray, &MeshRayCastSettings::default());
    for (hit_entity, _) in hits {
        let mut current = *hit_entity;
        loop {
            if obstacle_query.get(current).is_ok()
                || prop_query.get(current).is_ok()
                || camera_query.get(current).is_ok()
            {
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

// --- GLB hot-reload ---

/// Detects when the obstacles glTF is modified, snapshots all placed entities,
/// despawns them, and stores the snapshot for respawning once the new asset is ready.
#[allow(clippy::too_many_arguments)]
fn snapshot_and_despawn_on_glb_reload(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<bevy::gltf::Gltf>>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    course_state: Option<Res<EditorCourse>>,
    placed_query: Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), (Without<PlacedObstacle>, Without<PlacedCamera>)>,
    camera_child_query: Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
    child_of_query: Query<(Entity, &ChildOf), With<PlacedCamera>>,
    placed_all: Query<Entity, PlacedFilter>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
) {
    let Some(handle) = gltf_handle else { return };
    let Some(course_state) = course_state else { return };
    if !events.read().any(|e| matches!(e, AssetEvent::Modified { id } if *id == handle.0.id())) {
        return;
    }

    let obstacles_with_cameras = placed_query.iter().map(|(entity, placed, transform)| {
        let camera = child_of_query
            .iter()
            .find(|(_, child_of)| child_of.parent() == entity)
            .and_then(|(cam_entity, _)| camera_child_query.get(cam_entity).ok());
        (placed, transform, camera)
    });

    let course_data = ui::build_course_data(
        course_state.name.clone(),
        String::new(), // location not needed for glb reload snapshot
        obstacles_with_cameras,
        prop_query.iter(),
    );

    for entity in &placed_all {
        commands.entity(entity).despawn();
    }

    undo_stack.clear();
    commands.insert_resource(PendingGlbReload { course_data });
    info!("obstacles.glb reloaded — refreshing course editor");
}

/// Respawns all placed entities from the snapshot once the new glTF data is ready.
#[allow(clippy::too_many_arguments)]
fn respawn_after_glb_reload(
    mut commands: Commands,
    pending: Res<PendingGlbReload>,
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
    mut transform_state: ResMut<EditorTransform>,
    placed_query: Query<Entity, PlacedFilter>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Res<ObstaclesGltfHandle>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<crate::rendering::CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<crate::rendering::CelLightDir>,
    prop_meshes: Option<Res<ui::PropEditorMeshes>>,
    camera_meshes: Option<Res<ui::CameraEditorMeshes>>,
) {
    let mut ctx = crate::obstacle::spawning::SpawnObstacleContext::from_res(
        &gltf_assets,
        &node_assets,
        &mesh_assets,
        &mut cel_materials,
        &std_materials,
        &light_dir,
        &gltf_handle,
    );

    ui::load_course_into_editor(
        &mut commands,
        &mut selection,
        &mut course_state,
        &mut transform_state,
        &placed_query,
        &library,
        &mut ctx,
        &pending.course_data,
        prop_meshes.as_deref(),
        camera_meshes.as_deref(),
    );
    commands.remove_resource::<PendingGlbReload>();
}
