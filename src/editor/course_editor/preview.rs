use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};

use crate::camera::orbit::MainCamera;
use crate::camera::settings::CameraSettings;
use crate::palette;
use crate::rendering::{FOG_END, FOG_START, fog_color};
use crate::states::EditorMode;

use super::PlacedCamera;
use super::ui::{PendingThumbnailSave, ThumbnailCamera, ThumbnailRenderTarget};
use super::EditorSelection;

const PREVIEW_WIDTH: u32 = 384;
const PREVIEW_HEIGHT: u32 = 216;
const PREVIEW_MARGIN: f32 = 12.0;
const PREVIEW_BORDER: f32 = 3.0;
/// Offset from the right edge to clear the 280px right panel.
const RIGHT_PANEL_WIDTH: f32 = 280.0;

const THUMBNAIL_WIDTH: u32 = 384;
const THUMBNAIL_HEIGHT: u32 = 216;

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
    // --- Gate camera preview ---
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
                    BackgroundColor(palette::SAND),
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

    // --- Thumbnail camera (for saving course screenshots) ---
    let mut thumb_image = Image::new_target_texture(
        THUMBNAIL_WIDTH,
        THUMBNAIL_HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    // COPY_SRC is required for GPU readback to extract pixel data.
    thumb_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let thumb_handle = images.add(thumb_image);

    let thumb_entity = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: -2,
                is_active: false,
                clear_color: ClearColorConfig::Custom(fog_color()),
                ..default()
            },
            RenderTarget::from(thumb_handle.clone()),
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
            ThumbnailCamera,
        ))
        .id();

    commands.insert_resource(ThumbnailRenderTarget {
        image_handle: thumb_handle,
        camera_entity: thumb_entity,
    });
}

pub fn sync_preview_camera(
    selection: Res<EditorSelection>,
    placed_cameras: Query<(&PlacedCamera, &GlobalTransform), Without<PreviewCamera>>,
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

    let selected_camera = selection
        .entity
        .and_then(|e| placed_cameras.get(e).ok());

    match selected_camera {
        Some((placed, gt)) => {
            let (_, rotation, translation) = gt.to_scale_rotation_translation();
            *cam_tf = Transform::from_translation(translation).with_rotation(rotation);

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

/// System that positions the thumbnail camera, waits for it to render, then
/// requests GPU readback. The actual PNG save happens in the readback observer.
pub fn save_thumbnail_when_ready(
    mut commands: Commands,
    mut pending: ResMut<PendingThumbnailSave>,
    thumbnail: Res<ThumbnailRenderTarget>,
    mut thumbnail_cam: Query<(&mut Camera, &mut Transform), With<ThumbnailCamera>>,
    editor_cam: Query<&GlobalTransform, (With<MainCamera>, Without<ThumbnailCamera>)>,
) {
    let Ok((mut cam, mut cam_tf)) = thumbnail_cam.single_mut() else {
        return;
    };

    // Frame 0: position and activate the thumbnail camera
    if pending.frames_waited == 0 {
        if let Ok(editor_gt) = editor_cam.single() {
            let (_, rotation, translation) = editor_gt.to_scale_rotation_translation();
            *cam_tf = Transform::from_translation(translation).with_rotation(rotation);
            cam.is_active = true;
        }
        pending.frames_waited = 1;
        return;
    }

    pending.frames_waited += 1;
    if pending.frames_waited < 3 {
        return;
    }

    // Request async GPU readback of the thumbnail render target.
    let course_name = pending.course_name.clone();
    let width = THUMBNAIL_WIDTH;
    let height = THUMBNAIL_HEIGHT;
    commands
        .spawn((
            Readback::texture(thumbnail.image_handle.clone()),
            DespawnOnExit(EditorMode::CourseEditor),
        ))
        .observe(move |event: On<ReadbackComplete>, mut commands: Commands| {
            let data = &event.data;
            let path_str = format!("assets/courses/{course_name}.png");
            if let Some(rgba) = image::RgbaImage::from_raw(width, height, data.clone()) {
                match rgba.save(&path_str) {
                    Ok(()) => info!("Saved thumbnail to {path_str}"),
                    Err(e) => error!("Failed to save thumbnail: {e}"),
                }
            } else {
                error!("Failed to construct image from render target data");
            }
            // Despawn the readback entity so it doesn't fire every frame.
            // Use try_despawn since DespawnOnExit may have already despawned it.
            commands.entity(event.observer()).try_despawn();
        });

    // Deactivate thumbnail camera
    cam.is_active = false;
    commands.remove_resource::<PendingThumbnailSave>();
}

pub fn cleanup_camera_preview(
    mut commands: Commands,
    preview: Option<Res<CameraPreview>>,
    thumbnail: Option<Res<ThumbnailRenderTarget>>,
) {
    if let Some(preview) = preview {
        commands.entity(preview.camera_entity).despawn();
    }
    commands.remove_resource::<CameraPreview>();

    if let Some(thumbnail) = thumbnail {
        commands.entity(thumbnail.camera_entity).despawn();
    }
    commands.remove_resource::<ThumbnailRenderTarget>();
    commands.remove_resource::<PendingThumbnailSave>();
}
