use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::camera::orbit::MainCamera;
use crate::camera::settings::CameraSettings;
use crate::palette;
use crate::rendering::{FOG_END, FOG_START, fog_color};
use crate::states::EditorMode;

use super::{PlacedCamera, PlacementState};

const PREVIEW_WIDTH: u32 = 384;
const PREVIEW_HEIGHT: u32 = 216;
const PREVIEW_MARGIN: f32 = 12.0;
const PREVIEW_BORDER: f32 = 2.0;
/// Offset from the right edge to clear the 280px right panel.
const RIGHT_PANEL_WIDTH: f32 = 280.0;

#[derive(Resource)]
pub struct CameraPreview {
    pub camera_entity: Entity,
}

#[derive(Component)]
pub struct PreviewCamera;

#[derive(Component)]
pub struct PreviewOverlay;

#[derive(Component)]
pub struct PreviewLabel;

pub fn setup_camera_preview(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_target_texture(
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
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
            PreviewCamera,
        ))
        .id();

    commands
        .spawn((
            PreviewOverlay,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(PREVIEW_MARGIN + RIGHT_PANEL_WIDTH),
                bottom: Val::Px(PREVIEW_MARGIN),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            DespawnOnExit(EditorMode::CourseEditor),
        ))
        .with_children(|parent| {
            // Border container
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(PREVIEW_BORDER)),
                        ..default()
                    },
                    BackgroundColor(palette::STEEL),
                ))
                .with_children(|border| {
                    border.spawn((
                        ImageNode::new(image_handle.clone()),
                        Node {
                            width: Val::Px(PREVIEW_WIDTH as f32),
                            height: Val::Px(PREVIEW_HEIGHT as f32),
                            ..default()
                        },
                    ));
                });

            // Label
            parent.spawn((
                PreviewLabel,
                Text::new("Camera Preview"),
                TextFont {
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

    commands.insert_resource(CameraPreview { camera_entity });
}

pub fn sync_preview_camera(
    state: Res<PlacementState>,
    placed_cameras: Query<(&PlacedCamera, &Transform), Without<PreviewCamera>>,
    mut preview_camera: Query<
        (&mut Camera, &mut Transform, &mut Projection),
        (With<PreviewCamera>, Without<MainCamera>),
    >,
    mut overlay_vis: Query<&mut Visibility, With<PreviewOverlay>>,
    mut label_text: Query<&mut Text, With<PreviewLabel>>,
    settings: Res<CameraSettings>,
) {
    let Ok((mut cam, mut cam_tf, mut projection)) = preview_camera.single_mut() else {
        return;
    };

    let selected_camera = state
        .selected_entity
        .and_then(|e| placed_cameras.get(e).ok());

    match selected_camera {
        Some((placed, transform)) => {
            *cam_tf = *transform;

            if !cam.is_active {
                cam.is_active = true;
            }

            // Keep FOV in sync with the editor camera settings
            if let Projection::Perspective(ref mut persp) = *projection {
                persp.fov = settings.fov_degrees.to_radians();
            }

            if let Ok(mut vis) = overlay_vis.single_mut() {
                *vis = Visibility::Inherited;
            }

            if let Ok(mut text) = label_text.single_mut() {
                let label = placed.label.as_deref().unwrap_or("Camera");
                let primary_tag = if placed.is_primary { " [PRIMARY]" } else { "" };
                **text = format!("{}{}", label, primary_tag);
            }
        }
        None => {
            if cam.is_active {
                cam.is_active = false;
            }

            if let Ok(mut vis) = overlay_vis.single_mut() {
                *vis = Visibility::Hidden;
            }
        }
    }
}

pub fn cleanup_camera_preview(mut commands: Commands, preview: Option<Res<CameraPreview>>) {
    if let Some(preview) = preview {
        commands.entity(preview.camera_entity).despawn();
    }
    commands.remove_resource::<CameraPreview>();
}
