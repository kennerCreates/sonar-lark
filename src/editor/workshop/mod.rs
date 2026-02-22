pub mod ui;

use bevy::prelude::*;

use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::states::EditorMode;

#[derive(Resource)]
pub struct WorkshopState {
    pub obstacle_id: String,
    pub node_name: String,
    pub is_gate: bool,
    pub has_trigger: bool,
    pub trigger_offset: Vec3,
    pub trigger_half_extents: Vec3,
    pub preview_entity: Option<Entity>,
    pub available_nodes: Vec<String>,
    pub nodes_loaded: bool,
    pub editing_id: bool,
    pub editing_node: bool,
}

impl Default for WorkshopState {
    fn default() -> Self {
        Self {
            obstacle_id: String::new(),
            node_name: String::new(),
            is_gate: true,
            has_trigger: true,
            trigger_offset: Vec3::new(0.0, 1.0, 0.0),
            trigger_half_extents: Vec3::new(2.0, 2.0, 0.5),
            preview_entity: None,
            available_nodes: Vec::new(),
            nodes_loaded: false,
            editing_id: false,
            editing_node: false,
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
                populate_node_list,
                ui::handle_node_selection,
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
                ui::handle_node_text_input,
                ui::handle_node_field_focus,
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

    let mut nodes: Vec<String> = gltf.named_nodes.keys().map(|k| k.to_string()).collect();
    nodes.sort();
    state.available_nodes = nodes.clone();
    state.nodes_loaded = true;

    let Ok(container) = container_query.single() else {
        return;
    };

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

/// Spawn a preview from a named node in the glTF.
pub fn spawn_preview(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    gltf_handle: &ObstaclesGltfHandle,
    node_name: &str,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    let parent = commands
        .spawn((
            node.transform,
            Visibility::default(),
            PreviewObstacle,
        ))
        .id();

    for primitive in &gltf_mesh.primitives {
        let mut prim_commands = commands.spawn((
            Mesh3d(primitive.mesh.clone()),
            Transform::default(),
        ));

        if let Some(ref material) = primitive.material {
            prim_commands.insert(MeshMaterial3d(material.clone()));
        }

        prim_commands.set_parent_in_place(parent);
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
    preview_query: Query<Entity, With<PreviewObstacle>>,
) {
    if state.node_name.is_empty() || state.preview_entity.is_some() {
        return;
    }

    // Try to spawn from glTF first
    if let Some(handle) = &gltf_handle {
        if let Some(entity) = spawn_preview(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            handle,
            &state.node_name,
        ) {
            state.preview_entity = Some(entity);
            return;
        }
    }

    // No matching glTF node — spawn a placeholder cube
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
    if !state.has_trigger || state.node_name.is_empty() {
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
