use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::camera::settings::CameraSettings;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, FOG_END, FOG_START, cel_material_from_color, fog_color};
use crate::ui_theme::UiFont;

use super::{PreviewObstacle, WorkshopState};

// --- Camera View (render-to-texture) ---

const VIEW_WIDTH: u32 = 384;
const VIEW_HEIGHT: u32 = 216;
const VIEW_MARGIN: f32 = 12.0;
const VIEW_BORDER: f32 = 3.0;
const RIGHT_PANEL_WIDTH: f32 = 280.0;

#[derive(Resource)]
pub(super) struct CameraViewState {
    pub camera_entity: Entity,
}

#[derive(Component)]
pub(super) struct CameraViewCamera;

#[derive(Component)]
pub(super) struct CameraViewOverlay;

/// Spawn a preview from a named node in the glTF.
///
/// Parent entity gets `model_offset` as translation and `model_rotation` as rotation.
/// Child meshes get the node's Blender-authored rotation and scale.
pub fn spawn_preview(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    cel_materials: &mut Assets<CelMaterial>,
    std_materials: &Assets<StandardMaterial>,
    light_dir: Vec3,
    gltf_handle: &ObstaclesGltfHandle,
    node_name: &str,
    model_offset: Vec3,
    model_rotation: Quat,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    let parent_transform =
        Transform::from_translation(model_offset).with_rotation(model_rotation);
    let child_transform = Transform {
        rotation: node.transform.rotation,
        scale: node.transform.scale,
        ..default()
    };

    // Pre-collect materials before spawning to avoid borrow conflicts
    let primitives: Vec<(Handle<Mesh>, MeshMaterial3d<CelMaterial>)> = gltf_mesh
        .primitives
        .iter()
        .map(|p| {
            let base_color = p.material
                .as_ref()
                .and_then(|h| std_materials.get(h))
                .map(|m| m.base_color)
                .unwrap_or(Color::srgb(0.502, 0.475, 0.502)); // Chainmail #807980
            let mat = MeshMaterial3d(cel_materials.add(cel_material_from_color(base_color, light_dir)));
            (p.mesh.clone(), mat)
        })
        .collect();

    let parent = commands
        .spawn((
            parent_transform,
            Visibility::default(),
            PreviewObstacle,
        ))
        .id();

    for (mesh, material) in primitives {
        commands
            .spawn((
                Mesh3d(mesh),
                material,
                child_transform,
            ))
            .set_parent_in_place(parent);
    }

    Some(parent)
}

pub(super) fn spawn_placeholder_preview(
    mut commands: Commands,
    mut state: ResMut<WorkshopState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
) {
    if state.node_name.is_empty() || state.preview_entity.is_some() {
        return;
    }

    let offset = state.model_offset;
    let rotation = state.model_rotation;

    // Try to spawn from glTF first
    if let Some(handle) = &gltf_handle
        && let Some(entity) = spawn_preview(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            &std_materials,
            light_dir.0,
            handle,
            &state.node_name,
            offset,
            rotation,
        )
    {
        state.preview_entity = Some(entity);
        return;
    }

    // No matching glTF node — spawn a placeholder cube
    let entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(cel_materials.add(cel_material_from_color(
                palette::CHAINMAIL,
                light_dir.0,
            ))),
            Transform::from_translation(offset).with_rotation(rotation),
            PreviewObstacle,
        ))
        .id();
    state.preview_entity = Some(entity);
}

pub(super) fn setup_camera_view(mut commands: Commands, mut images: ResMut<Assets<Image>>, font: Res<UiFont>) {
    let ui_font = font.0.clone();
    let image = Image::new_target_texture(
        VIEW_WIDTH,
        VIEW_HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let image_handle = images.add(image);

    let camera_entity = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: -1,
                is_active: false,
                clear_color: ClearColorConfig::Custom(fog_color()),
                ..default()
            },
            RenderTarget::from(image_handle.clone()),
            Transform::default(),
            DistanceFog {
                color: fog_color(),
                directional_light_color: Color::NONE,
                directional_light_exponent: 0.0,
                falloff: FogFalloff::Linear {
                    start: FOG_START,
                    end: FOG_END,
                },
            },
            CameraViewCamera,
        ))
        .id();

    commands
        .spawn((
            CameraViewOverlay,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(VIEW_MARGIN + RIGHT_PANEL_WIDTH),
                bottom: Val::Px(VIEW_MARGIN),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(VIEW_BORDER)),
                        ..default()
                    },
                    BackgroundColor(palette::SAND),
                ))
                .with_children(|border| {
                    border.spawn((
                        ImageNode::new(image_handle.clone()),
                        Node {
                            width: Val::Px(VIEW_WIDTH as f32),
                            height: Val::Px(VIEW_HEIGHT as f32),
                            ..default()
                        },
                    ));
                });

            parent.spawn((
                Text::new("Camera View"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::SAND),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));
        });

    commands.insert_resource(CameraViewState { camera_entity });
}

pub(super) fn sync_camera_view(
    state: Res<WorkshopState>,
    preview_query: Query<&Transform, With<PreviewObstacle>>,
    mut view_camera: Query<
        (&mut Camera, &mut Transform, &mut Projection),
        (With<CameraViewCamera>, Without<PreviewObstacle>),
    >,
    mut overlay_vis: Query<&mut Visibility, With<CameraViewOverlay>>,
    settings: Res<CameraSettings>,
) {
    let Ok((mut cam, mut cam_tf, mut projection)) = view_camera.single_mut() else {
        return;
    };

    let should_show = state.has_camera && state.preview_entity.is_some();

    if should_show {
        let preview_pos = state
            .preview_entity
            .and_then(|e| preview_query.get(e).ok())
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        *cam_tf = Transform::from_translation(preview_pos + state.camera_offset)
            .with_rotation(state.camera_rotation);

        if !cam.is_active {
            cam.is_active = true;
        }

        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = settings.fov_degrees.to_radians();
        }

        if let Ok(mut vis) = overlay_vis.single_mut() {
            *vis = Visibility::Inherited;
        }
    } else {
        if cam.is_active {
            cam.is_active = false;
        }

        if let Ok(mut vis) = overlay_vis.single_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

pub(super) fn cleanup_camera_view(
    mut commands: Commands,
    view_state: Option<Res<CameraViewState>>,
    overlay_query: Query<Entity, With<CameraViewOverlay>>,
) {
    if let Some(vs) = view_state {
        commands.entity(vs.camera_entity).despawn();
    }
    commands.remove_resource::<CameraViewState>();
    for entity in &overlay_query {
        commands.entity(entity).despawn();
    }
}

