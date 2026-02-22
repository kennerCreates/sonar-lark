pub mod ui;

use bevy::prelude::*;

use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::states::EditorMode;

#[derive(Resource)]
pub struct WorkshopState {
    pub obstacle_id: String,
    pub scene_name: String,
    pub is_gate: bool,
    pub has_trigger: bool,
    pub trigger_offset: Vec3,
    pub trigger_half_extents: Vec3,
    pub preview_entity: Option<Entity>,
    pub available_scenes: Vec<String>,
    pub scenes_loaded: bool,
    pub editing_id: bool,
    pub editing_scene: bool,
}

impl Default for WorkshopState {
    fn default() -> Self {
        Self {
            obstacle_id: String::new(),
            scene_name: String::new(),
            is_gate: true,
            has_trigger: true,
            trigger_offset: Vec3::new(0.0, 1.0, 0.0),
            trigger_half_extents: Vec3::new(2.0, 2.0, 0.5),
            preview_entity: None,
            available_scenes: Vec::new(),
            scenes_loaded: false,
            editing_id: false,
            editing_scene: false,
        }
    }
}

#[derive(Component)]
pub struct PreviewObstacle;

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
                populate_scene_list,
                ui::handle_scene_selection,
                ui::handle_library_selection,
                ui::handle_adjust_buttons,
                ui::handle_is_gate_toggle,
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
                ui::handle_id_text_input,
                ui::handle_id_field_focus,
                ui::handle_scene_text_input,
                ui::handle_scene_field_focus,
                ui::update_display_values,
                ui::handle_button_hover,
                spawn_placeholder_preview,
                draw_trigger_gizmo,
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
}

fn cleanup_workshop(
    mut commands: Commands,
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    commands.remove_resource::<WorkshopState>();
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }
}

fn populate_scene_list(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    container_query: Query<Entity, With<ui::SceneListContainer>>,
) {
    if state.scenes_loaded {
        return;
    }
    let Some(handle) = gltf_handle else { return };
    let Some(gltf) = gltf_assets.get(&handle.0) else {
        return;
    };

    let mut scenes: Vec<String> = gltf.named_scenes.keys().map(|k| k.to_string()).collect();
    scenes.sort();
    state.available_scenes = scenes.clone();
    state.scenes_loaded = true;

    let Ok(container) = container_query.single() else {
        return;
    };

    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        if scenes.is_empty() {
            parent.spawn((
                Text::new("No scenes found in glb"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        } else {
            for scene in &scenes {
                ui::spawn_scene_button(parent, scene);
            }
        }
    });

    info!("Found {} named scenes in obstacles.glb", scenes.len());
}

pub fn spawn_preview(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    gltf_handle: &ObstaclesGltfHandle,
    scene_name: &str,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let scene_handle = gltf.named_scenes.get(scene_name)?;

    let entity = commands
        .spawn((
            SceneRoot(scene_handle.clone()),
            Transform::default(),
            PreviewObstacle,
        ))
        .id();

    Some(entity)
}

fn spawn_placeholder_preview(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    if state.scene_name.is_empty() || state.preview_entity.is_some() {
        return;
    }

    // Try to spawn from glTF first
    if let Some(handle) = &gltf_handle {
        if let Some(entity) = spawn_preview(&mut commands, &gltf_assets, handle, &state.scene_name)
        {
            state.preview_entity = Some(entity);
            return;
        }
    }

    // No matching glTF scene — spawn a placeholder cube
    if preview_query.is_empty() {
        let entity = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.5, 0.5, 0.6),
                    ..default()
                })),
                Transform::default(),
                PreviewObstacle,
            ))
            .id();
        state.preview_entity = Some(entity);
    }
}

fn draw_trigger_gizmo(mut gizmos: Gizmos, state: Res<WorkshopState>) {
    if !state.has_trigger || state.scene_name.is_empty() {
        return;
    }

    let color = if state.is_gate {
        Color::srgb(0.2, 1.0, 0.2)
    } else {
        Color::srgb(1.0, 0.8, 0.2)
    };

    let center = state.trigger_offset;
    let size = state.trigger_half_extents * 2.0;
    let transform = Transform::from_translation(center).with_scale(size);

    gizmos.cube(transform, color);
}
