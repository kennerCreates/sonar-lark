mod gizmos;
mod preview;
pub mod ui;
mod widgets;

use bevy::{
    asset::AssetEvent,
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    prelude::*,
};

use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{ObstaclesGltfHandle, obstacles_gltf_ready};
use crate::palette;
use crate::states::DevMenuPage;

use super::gizmos::{Axis, Sign};

#[derive(Clone)]
pub struct CollisionVolumeData {
    pub offset: Vec3,
    pub half_extents: Vec3,
    pub rotation: Quat,
}

impl Default for CollisionVolumeData {
    fn default() -> Self {
        Self {
            offset: Vec3::ZERO,
            half_extents: Vec3::new(3.0, 3.0, 1.5),
            rotation: Quat::IDENTITY,
        }
    }
}

#[derive(Resource)]
pub struct WorkshopState {
    pub obstacle_name: String,
    pub node_name: String,
    pub is_gate: bool,
    pub has_trigger: bool,
    pub trigger_offset: Vec3,
    pub trigger_half_extents: Vec3,
    pub trigger_rotation: Quat,
    pub has_collision: bool,
    /// Working copy of the active collision volume (drives widgets/gizmos).
    pub collision_offset: Vec3,
    pub collision_half_extents: Vec3,
    pub collision_rotation: Quat,
    /// All collision volumes for this obstacle.
    pub collision_volumes: Vec<CollisionVolumeData>,
    /// Index of the currently-edited volume in `collision_volumes`.
    pub active_collision_idx: usize,
    pub has_camera: bool,
    pub camera_offset: Vec3,
    pub camera_rotation: Quat,
    pub preview_entity: Option<Entity>,
    pub available_nodes: Vec<String>,
    pub nodes_loaded: bool,
    pub editing_name: bool,
    pub edit_target: EditTarget,
    pub model_offset: Vec3,
    pub model_rotation: Quat,
    pub transform_mode: TransformMode,
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
            trigger_rotation: Quat::IDENTITY,
            has_collision: false,
            collision_offset: Vec3::ZERO,
            collision_half_extents: Vec3::new(3.0, 3.0, 1.5),
            collision_rotation: Quat::IDENTITY,
            collision_volumes: Vec::new(),
            active_collision_idx: 0,
            has_camera: false,
            camera_offset: Vec3::new(0.0, 2.0, -5.0),
            camera_rotation: Quat::IDENTITY,
            preview_entity: None,
            available_nodes: Vec::new(),
            nodes_loaded: false,
            editing_name: false,
            edit_target: EditTarget::default(),
            model_offset: Vec3::ZERO,
            model_rotation: Quat::IDENTITY,
            transform_mode: TransformMode::default(),
        }
    }
}

impl WorkshopState {
    /// Save the working-copy fields back into the volumes vec at the active index.
    pub fn sync_active_to_vec(&mut self) {
        if let Some(vol) = self.collision_volumes.get_mut(self.active_collision_idx) {
            vol.offset = self.collision_offset;
            vol.half_extents = self.collision_half_extents;
            vol.rotation = self.collision_rotation;
        }
    }

    /// Load the volume at `active_collision_idx` into the working-copy fields.
    pub fn load_active_from_vec(&mut self) {
        if let Some(vol) = self.collision_volumes.get(self.active_collision_idx) {
            self.collision_offset = vol.offset;
            self.collision_half_extents = vol.half_extents;
            self.collision_rotation = vol.rotation;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum EditTarget {
    #[default]
    Model,
    Trigger,
    Collision,
    Camera,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformMode {
    #[default]
    Move,
    Rotate,
    Resize,
}

#[derive(Component)]
pub struct PreviewObstacle;

#[derive(Component)]
pub struct CameraPreview;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct TriggerGizmoGroup;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveDragMode {
    XzPlane,
    YAxis,
}

#[derive(Resource, Default)]
struct MoveWidgetState {
    active_drag: Option<MoveDragMode>,
    hovered: bool,
    drag_anchor: Vec3,
    start_offset: Vec3,
}

#[derive(Resource, Default)]
struct RotateWidgetState {
    active: bool,
    hovered: bool,
    active_axis: Axis,
    drag_start_angle: f32,
    entity_start_rotation: Quat,
}

#[derive(Resource, Default)]
struct ResizeWidgetState {
    active_handle: Option<(Axis, Sign)>,
    hovered_handle: Option<(Axis, Sign)>,
}

const ARROW_LENGTH: f32 = 3.75;
const HANDLE_SIZE: f32 = 0.2;
const HANDLE_HIT_THRESHOLD: f32 = 20.0; // pixels

pub struct WorkshopPlugin;

impl Plugin for WorkshopPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<TriggerGizmoGroup>()
        .add_systems(
            OnEnter(DevMenuPage::ObstacleWorkshop),
            (load_gltf_for_workshop, setup_workshop, preview::setup_camera_view),
        )
        // Populate node list once glTF is loaded (runs at most once per workshop entry)
        .add_systems(
            Update,
            populate_node_list
                .run_if(in_state(DevMenuPage::ObstacleWorkshop))
                .run_if(workshop_nodes_pending)
                .run_if(obstacles_gltf_ready),
        )
        .add_systems(
            Update,
            (
                ui::handle_node_selection,
                ui::handle_library_selection,
                ui::handle_trigger_toggle,
                ui::handle_collision_toggle,
                ui::handle_camera_toggle,
                ui::handle_add_collision_shape,
                ui::handle_remove_collision_shape,
                ui::handle_prev_collision_shape,
                ui::handle_next_collision_shape,
                ui::handle_save_button,
                ui::handle_new_button,
                ui::handle_delete_button,
                ui::handle_back_button,
            )
                .run_if(in_state(DevMenuPage::ObstacleWorkshop)),
        )
        .add_systems(
            Update,
            (
                ui::handle_name_text_input,
                ui::handle_name_field_focus,
                ui::handle_edit_target_toggle,
                ui::update_display_values,
                ui::handle_button_hover,
                preview::spawn_placeholder_preview,
                preview::update_camera_preview,
                preview::sync_camera_view,
                gizmos::draw_trigger_gizmo,
                gizmos::draw_collision_gizmo,
                gizmos::draw_camera_gizmo,
            )
                .run_if(in_state(DevMenuPage::ObstacleWorkshop)),
        )
        .add_systems(
            Update,
            (
                gizmos::draw_ground_gizmo,
                widgets::handle_transform_mode_keys,
                widgets::draw_move_arrows,
                widgets::handle_move_widget,
                widgets::draw_rotate_gizmo,
                widgets::handle_rotate_gizmo,
                widgets::draw_resize_handles,
                widgets::handle_resize_widget,
            )
                .run_if(in_state(DevMenuPage::ObstacleWorkshop)),
        )
        .add_systems(
            Update,
            detect_glb_reload.run_if(in_state(DevMenuPage::ObstacleWorkshop)),
        )
        .add_systems(OnExit(DevMenuPage::ObstacleWorkshop), (cleanup_workshop, preview::cleanup_camera_view));
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

fn setup_workshop(
    mut commands: Commands,
    library: Res<ObstacleLibrary>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let state = WorkshopState::default();
    ui::build_workshop_ui(&mut commands, &library);
    commands.insert_resource(state);
    commands.insert_resource(MoveWidgetState::default());
    commands.insert_resource(RotateWidgetState::default());
    commands.insert_resource(ResizeWidgetState::default());

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
}

fn cleanup_workshop(
    mut commands: Commands,
    preview_query: Query<Entity, With<PreviewObstacle>>,
    camera_preview_query: Query<Entity, With<CameraPreview>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.remove_resource::<WorkshopState>();
    commands.remove_resource::<MoveWidgetState>();
    commands.remove_resource::<RotateWidgetState>();
    commands.remove_resource::<ResizeWidgetState>();
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }
    for entity in &camera_preview_query {
        commands.entity(entity).despawn();
    }

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = 0.0;
}

/// Run condition: true when `WorkshopState` exists but nodes haven't been loaded yet.
fn workshop_nodes_pending(state: Option<Res<WorkshopState>>) -> bool {
    state.is_some_and(|s| !s.nodes_loaded)
}

/// Populates the workshop node list from the loaded glTF asset.
/// Gated by `run_if(workshop_nodes_pending)` and `run_if(obstacles_gltf_ready)`.
fn populate_node_list(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    gltf_handle: Res<ObstaclesGltfHandle>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    container_query: Query<Entity, With<ui::NodeListContainer>>,
) {
    let gltf = gltf_assets.get(&gltf_handle.0).expect("run condition guarantees loaded");

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
                TextColor(palette::CHAINMAIL),
            ));
        } else {
            for node in &nodes {
                ui::spawn_node_button(parent, node);
            }
        }
    });

    info!("Found {} named nodes in obstacles.glb", nodes.len());
}

/// Detects when the obstacles glTF is hot-reloaded and resets workshop state
/// so that `populate_node_list` and `spawn_placeholder_preview` re-trigger.
fn detect_glb_reload(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<bevy::gltf::Gltf>>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    mut state: ResMut<WorkshopState>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    let Some(handle) = gltf_handle else { return };
    if !events.read().any(|e| matches!(e, AssetEvent::Modified { id } if *id == handle.0.id())) {
        return;
    }
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }
    state.preview_entity = None;
    state.nodes_loaded = false;
    info!("obstacles.glb reloaded — refreshing workshop preview");
}
