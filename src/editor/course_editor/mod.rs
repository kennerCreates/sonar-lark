pub mod ui;

mod overlays;
mod transform_gizmos;

use bevy::{
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
    prelude::*,
};

use crate::course::data::PropKind;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::states::EditorMode;

use transform_gizmos::{MoveWidgetState, RotateWidgetState, ScaleWidgetState};

// --- Transform mode ---

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformMode {
    #[default]
    Move,
    Rotate,
    Scale,
}

// --- Resources ---

/// Inserted before entering the editor to request auto-loading a specific course.
/// Consumed by `auto_load_pending_course` once glTF assets are ready.
#[derive(Resource)]
pub struct PendingEditorCourse {
    pub path: String,
}

/// Tracks the last course loaded or saved in the editor.
/// Persists across states so the editor can reopen it.
#[derive(Resource)]
pub struct LastEditedCourse {
    pub path: String,
}

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
    /// Active tab in the left panel palette.
    pub active_tab: EditorTab,
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
            active_tab: EditorTab::default(),
        }
    }
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    Obstacles,
    Props,
}

/// Query filter matching any editor-placed entity (obstacle or prop).
type PlacedFilter = Or<(With<PlacedObstacle>, With<PlacedProp>)>;

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
                ui::auto_load_pending_course.run_if(in_state(EditorMode::CourseEditor)),
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
                    handle_flip_gate_key,
                    overlays::draw_trigger_gizmos,
                    overlays::draw_gate_sequence_lines,
                    overlays::draw_gate_forward_arrows,
                    overlays::draw_selection_highlight,
                    overlays::draw_flight_spline_preview,
                    overlays::draw_prop_gizmos,
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
    placed_query: Query<Entity, PlacedFilter>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.remove_resource::<PlacementState>();
    commands.remove_resource::<MoveWidgetState>();
    commands.remove_resource::<RotateWidgetState>();
    commands.remove_resource::<ScaleWidgetState>();
    commands.remove_resource::<PendingEditorCourse>();
    commands.remove_resource::<ui::PendingCourseDelete>();
    commands.remove_resource::<ui::PropEditorMeshes>();
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
    mut obstacle_query: Query<(Entity, &mut PlacedObstacle)>,
    prop_query: Query<Entity, With<PlacedProp>>,
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
        &obstacle_query,
        &prop_query,
        &parent_query,
    );

    if state.gate_order_mode {
        if let Some(entity) = nearest {
            if let Ok((_, mut placed)) = obstacle_query.get_mut(entity) {
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

fn handle_flip_gate_key(
    state: Res<PlacementState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut placed_query: Query<&mut PlacedObstacle>,
) {
    if state.editing_name {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    let Some(entity) = state.selected_entity else {
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
    parent_query: &Query<&ChildOf>,
) -> Option<Entity> {
    let hits = ray_cast.cast_ray(ray, &MeshRayCastSettings::default());
    for (hit_entity, _) in hits {
        let mut current = *hit_entity;
        loop {
            if obstacle_query.get(current).is_ok() || prop_query.get(current).is_ok() {
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
