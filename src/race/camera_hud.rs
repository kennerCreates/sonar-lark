use bevy::prelude::*;

use crate::camera::switching::{CameraMode, CameraState, CourseCameras};
use crate::drone::spawning::DRONE_NAMES;
use crate::palette;
use crate::pilot::SelectedPilots;
use crate::states::AppState;

use super::progress::RaceProgress;

#[derive(Component)]
pub(crate) struct CameraHudRoot;

#[derive(Component)]
pub(crate) struct CameraHudModeText;

#[derive(Component)]
pub(crate) struct CameraHudHintText;

pub fn setup_camera_hud(mut commands: Commands) {
    commands
        .spawn((
            CameraHudRoot,
            DespawnOnExit(AppState::Race),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            GlobalZIndex(80),
        ))
        .with_children(|panel| {
            panel.spawn((
                CameraHudModeText,
                Text::new("CHASE CAM"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SKY),
            ));
            panel.spawn((
                CameraHudHintText,
                Text::new("[1] Chase  [2] Spectator  [3] FPV"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::STONE),
            ));
        });
}

pub fn update_camera_hud(
    camera_state: Res<CameraState>,
    progress: Option<Res<RaceProgress>>,
    course_cameras: Option<Res<CourseCameras>>,
    selected: Option<Res<SelectedPilots>>,
    mut mode_text: Query<&mut Text, (With<CameraHudModeText>, Without<CameraHudHintText>)>,
    mut hint_text: Query<&mut Text, (With<CameraHudHintText>, Without<CameraHudModeText>)>,
) {
    if !camera_state.is_changed()
        && (camera_state.mode != CameraMode::Fpv
            || progress.as_ref().is_none_or(|p| !p.is_changed()))
    {
        return;
    }

    let cam_count = course_cameras
        .as_ref()
        .map(|cc| cc.cameras.len())
        .unwrap_or(0);

    let mode_label = match camera_state.mode {
        CameraMode::Chase => "CHASE CAM".to_string(),
        CameraMode::Spectator => "SPECTATOR".to_string(),
        CameraMode::Fpv => {
            let drone_name = progress
                .as_ref()
                .and_then(|p| {
                    let standings = p.standings();
                    let idx = camera_state
                        .target_standings_index
                        .min(standings.len().saturating_sub(1));
                    standings.get(idx).map(|&(drone_idx, _)| {
                        selected
                            .as_ref()
                            .and_then(|s| s.pilots.get(drone_idx))
                            .map(|p| p.gamertag.as_str())
                            .unwrap_or(
                                DRONE_NAMES.get(drone_idx).copied().unwrap_or("???"),
                            )
                    })
                })
                .unwrap_or("---");
            format!("FPV: {drone_name}")
        }
        CameraMode::CourseCamera(idx) => {
            let label = course_cameras
                .as_ref()
                .and_then(|cc| cc.cameras.get(idx))
                .and_then(|entry| entry.label.as_deref());
            if let Some(name) = label {
                format!("CAM {}: {name}", idx + 1)
            } else {
                format!("CAM {}", idx + 1)
            }
        }
    };

    for mut text in &mut mode_text {
        text.0 = mode_label.clone();
    }

    let hint = if cam_count > 0 {
        // Key mapping: 1=Cam0, 2=Chase, 3=Cam1, 4=Cam2, ..., 9=Cam7, 0=Cam8
        let cam_keys = match cam_count.min(9) {
            1 => "[1] Cam".to_string(),
            2 => "[1,3] Cams".to_string(),
            n => {
                let last = if n <= 8 { format!("{}", n + 1) } else { "0".to_string() };
                format!("[1,3-{}] Cams", last)
            }
        };
        match camera_state.mode {
            CameraMode::Fpv => format!("{cam_keys}  [2] Chase  [Shift+F] Next  [Shift+S] Spec"),
            _ => format!("{cam_keys}  [2] Chase  [Shift+F] FPV  [Shift+S] Spec"),
        }
    } else {
        match camera_state.mode {
            CameraMode::Fpv => {
                "[1] Chase  [2] Chase  [Shift+F] Next  [Shift+S] Spec".to_string()
            }
            _ => "[1] Chase  [Shift+F] FPV  [Shift+S] Spectator".to_string(),
        }
    };
    for mut text in &mut hint_text {
        text.0 = hint.clone();
    }
}
