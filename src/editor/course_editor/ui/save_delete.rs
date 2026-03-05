use std::fs;

use bevy::prelude::*;

use crate::course::loader::{SelectedCourse, save_course};
use crate::editor::course_editor::{
    DEFAULT_COURSE_NAME, EditorCourse, EditorSelection, EditorTransform, PlacedCamera,
    PlacedObstacle, PlacedProp,
};
use crate::states::{AppState, LastEditedCourse};

use super::data::build_course_data;
use super::types::*;

/// Resource inserted when a thumbnail render is pending after a save.
#[derive(Resource)]
pub struct PendingThumbnailSave {
    pub course_name: String,
    pub frames_waited: u8,
}

pub fn handle_back_to_menu(
    query: Query<&Interaction, (Changed<Interaction>, With<BackToMenuButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_save_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SaveCourseButton>)>,
    mut course_state: ResMut<EditorCourse>,
    placed_query: Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), (Without<PlacedObstacle>, Without<PlacedCamera>)>,
    camera_child_query: Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
    child_of_query: Query<(Entity, &ChildOf), With<PlacedCamera>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Auto-assign name if still default
        if course_state.name == DEFAULT_COURSE_NAME || course_state.name.is_empty() {
            course_state.name = next_auto_name();
        }

        let camera_count = child_of_query.iter().count();
        if camera_count > 9 {
            warn!(
                "Course has {camera_count} cameras (soft cap is 9). Consider reducing."
            );
        }

        let obstacles_with_cameras =
            placed_query
                .iter()
                .map(|(entity, placed, transform)| {
                    let camera = child_of_query
                        .iter()
                        .find(|(_, child_of)| child_of.parent() == entity)
                        .and_then(|(cam_entity, _)| camera_child_query.get(cam_entity).ok());
                    (placed, transform, camera)
                });

        let course = build_course_data(
            course_state.name.clone(),
            obstacles_with_cameras,
            prop_query.iter(),
        );

        let path_str = format!("assets/courses/{}.course.ron", course_state.name);
        let path = std::path::Path::new(&path_str);
        match save_course(&course, path) {
            Ok(()) => {
                info!(
                    "Saved course '{}' ({} obstacles, {} cameras) to {}",
                    course_state.name,
                    course.instances.len(),
                    camera_count,
                    path_str
                );
                commands.insert_resource(LastEditedCourse {
                    path: path_str.clone(),
                });

                // Trigger thumbnail capture (handled by separate system)
                commands.insert_resource(PendingThumbnailSave {
                    course_name: course_state.name.clone(),
                    frames_waited: 0,
                });
            }
            Err(e) => {
                error!("Failed to save course: {e}");
                continue;
            }
        }
    }
}

/// Scan `assets/courses/` for `course_NNN.course.ron` and return the next name.
fn next_auto_name() -> String {
    let courses_dir = std::path::Path::new("assets/courses");
    let mut max_num: u32 = 0;

    if let Ok(entries) = fs::read_dir(courses_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(stem) = name_str.strip_suffix(".course.ron")
                && let Some(num_str) = stem.strip_prefix("course_")
                && let Ok(num) = num_str.parse::<u32>()
            {
                max_num = max_num.max(num);
            }
        }
    }

    format!("course_{:03}", max_num + 1)
}

pub fn handle_gate_order_toggle(
    mut transform_state: ResMut<EditorTransform>,
    mut selection: ResMut<EditorSelection>,
    query: Query<&Interaction, (Changed<Interaction>, With<GateOrderModeButton>)>,
    placed_query: Query<&PlacedObstacle>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            transform_state.gate_order_mode = !transform_state.gate_order_mode;
            if transform_state.gate_order_mode {
                transform_state.next_gate_order = placed_query
                    .iter()
                    .filter_map(|p| p.gate_order)
                    .max()
                    .map(|m| m + 1)
                    .unwrap_or(0);
                selection.entity = None;
            }
        }
    }
}

pub fn handle_start_race(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartRaceButton>)>,
    mut course_state: ResMut<EditorCourse>,
    mut next_state: ResMut<NextState<AppState>>,
    placed_query: Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), (Without<PlacedObstacle>, Without<PlacedCamera>)>,
    camera_child_query: Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
    child_of_query: Query<(Entity, &ChildOf), With<PlacedCamera>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if course_state.name == DEFAULT_COURSE_NAME || course_state.name.is_empty() {
            course_state.name = next_auto_name();
        }

        let obstacles_with_cameras =
            placed_query
                .iter()
                .map(|(entity, placed, transform)| {
                    let camera = child_of_query
                        .iter()
                        .find(|(_, child_of)| child_of.parent() == entity)
                        .and_then(|(cam_entity, _)| camera_child_query.get(cam_entity).ok());
                    (placed, transform, camera)
                });

        let course = build_course_data(
            course_state.name.clone(),
            obstacles_with_cameras,
            prop_query.iter(),
        );

        let path_str = format!("assets/courses/{}.course.ron", course_state.name);
        let path = std::path::Path::new(&path_str);
        match save_course(&course, path) {
            Ok(()) => {
                info!(
                    "Saved course '{}' — starting race",
                    course_state.name,
                );
                commands.insert_resource(LastEditedCourse {
                    path: path_str.clone(),
                });
                commands.insert_resource(SelectedCourse {
                    path: path_str,
                });
                next_state.set(AppState::Race);
            }
            Err(e) => {
                error!("Failed to save course before racing: {e}");
            }
        }
    }
}

pub fn handle_clear_gate_orders_button(
    mut transform_state: ResMut<EditorTransform>,
    query: Query<&Interaction, (Changed<Interaction>, With<ClearGateOrdersButton>)>,
    mut placed_query: Query<&mut PlacedObstacle>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut placed in &mut placed_query {
            placed.gate_order = None;
        }
        transform_state.next_gate_order = 0;
        info!("Cleared all gate orders");
    }
}
