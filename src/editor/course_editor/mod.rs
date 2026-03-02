pub mod ui;

mod overlays;
mod preview;
mod transform_gizmos;

use bevy::{
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
    prelude::*,
};

use crate::camera::orbit::MainCamera;

use crate::course::data::PropKind;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{ObstaclesGltfHandle, obstacles_gltf_ready};
use crate::editor::types::EditorTab;
use crate::states::{EditorMode, PendingEditorCourse};

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
    pub editing_name: bool,
}

impl Default for EditorCourse {
    fn default() -> Self {
        Self {
            name: DEFAULT_COURSE_NAME.to_string(),
            editing_name: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct EditorUI {
    pub active_tab: EditorTab,
}

// --- Components ---

/// Marker on every obstacle entity spawned in the course editor.
#[derive(Component, Clone)]
pub struct PlacedObstacle {
    pub obstacle_id: ObstacleId,
    pub gate_order: Option<u32>,
    pub gate_forward_flipped: bool,
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
type PlacedFilter = Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>;

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
    course_state.editing_name = false;
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
                    ui::handle_back_to_workshop,
                    ui::handle_back_to_menu,
                    ui::handle_save_button,
                    ui::handle_load_button,
                    ui::handle_new_course_button,
                    ui::handle_gate_order_toggle,
                    ui::handle_clear_gate_orders_button,
                    ui::handle_name_field_focus,
                    ui::handle_name_text_input,
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
                    ui::handle_camera_placement,
                    ui::handle_remove_camera,
                    ui::handle_camera_primary_toggle,
                    ui::update_camera_primary_label,
                )
                    .run_if(in_state(EditorMode::CourseEditor)),
            )
            .add_systems(
                Update,
                (
                    ui::handle_delete_button,
                    ui::handle_confirm_delete,
                    ui::handle_cancel_delete,
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
                (
                    handle_placement_and_selection,
                    handle_delete_key,
                    handle_transform_mode_keys,
                    handle_flip_gate_key,
                    overlays::draw_trigger_gizmos,
                    overlays::draw_gate_sequence_lines,
                    overlays::draw_gate_forward_arrows,
                    overlays::draw_selection_highlight,
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
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let existing_courses = ui::discover_existing_courses();
    ui::build_course_editor_ui(&mut commands, &library, &existing_courses);
    commands.insert_resource(EditorSelection::default());
    commands.insert_resource(EditorTransform::default());
    commands.insert_resource(EditorCourse::default());
    commands.insert_resource(EditorUI::default());
    commands.insert_resource(MoveWidgetState::default());
    commands.insert_resource(RotateWidgetState::default());
    commands.insert_resource(ScaleWidgetState::default());

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
    config.line.width = 3.0;
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
    commands.remove_resource::<PendingEditorCourse>();
    commands.remove_resource::<ui::PendingCourseDelete>();
    commands.remove_resource::<ui::PropEditorMeshes>();
    commands.remove_resource::<ui::CameraEditorMeshes>();
    for entity in &placed_query {
        commands.entity(entity).despawn();
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = 0.0;
    config.line.width = 2.0;
}

// --- Placement and Selection ---

fn handle_placement_and_selection(
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
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
        || move_widget.hovered
        || rotate_widget.active
        || rotate_widget.hovered
        || scale_widget.active_drag.is_some()
        || scale_widget.hovered_axis.is_some()
        || scale_widget.hovered_center
    {
        return;
    }

    let over_ui = interaction_query.iter().any(|i| *i != Interaction::None);

    // Clicking in the 3D viewport while editing the course name unfocuses the text field.
    if course_state.editing_name {
        if mouse_buttons.just_pressed(MouseButton::Left) && !over_ui {
            course_state.editing_name = false;
        }
        return;
    }

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
    course_state: Res<EditorCourse>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if course_state.editing_name {
        return;
    }

    if keyboard.just_pressed(KeyCode::Delete)
        && let Some(entity) = selection.entity.take()
    {
        commands.entity(entity).despawn();
    }
}

fn handle_transform_mode_keys(
    mut transform_state: ResMut<EditorTransform>,
    course_state: Res<EditorCourse>,
    mut move_widget: ResMut<MoveWidgetState>,
    mut rotate_widget: ResMut<RotateWidgetState>,
    mut scale_widget: ResMut<ScaleWidgetState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if course_state.editing_name {
        return;
    }

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
        move_widget.hovered = false;
        rotate_widget.active = false;
        rotate_widget.hovered = false;
        scale_widget.active_drag = None;
        scale_widget.hovered_axis = None;
        scale_widget.hovered_center = false;
    }
}

fn handle_flip_gate_key(
    selection: Res<EditorSelection>,
    course_state: Res<EditorCourse>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut placed_query: Query<&mut PlacedObstacle>,
) {
    if course_state.editing_name {
        return;
    }
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
