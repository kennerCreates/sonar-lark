use bevy::prelude::*;

use crate::course::loader::SelectedCourse;
use crate::course::location::{LocationRegistry, LocationSaveData};
use crate::editor::course_editor::{
    EditorCourse, EditorSelection, EditorTransform, PlacedCamera, PlacedObstacle, PlacedProp,
};
use crate::persistence;
use crate::states::AppState;

use super::data::build_course_data;
use super::types::*;

/// Resource inserted when a thumbnail render is pending after a save.
#[derive(Resource)]
pub struct PendingThumbnailSave {
    pub course_name: String,
    pub frames_waited: u8,
}

/// Resource inserted when the user clicks "Start Race". The actual state
/// transition is deferred until the thumbnail readback entity has been spawned
/// (i.e. `PendingThumbnailSave` is removed).
#[derive(Resource)]
pub struct PendingRaceTransition;

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

/// Save location data (course + inventory) to the per-location file.
fn save_location_data<'a>(
    course_state: &EditorCourse,
    location_registry: &LocationRegistry,
    placed_query: &'a Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: impl IntoIterator<Item = (&'a PlacedProp, &'a Transform)>,
    child_of_query: &'a Query<(Entity, &ChildOf), With<PlacedCamera>>,
    camera_child_query: &'a Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
) -> Result<String, String> {
    let location = location_registry
        .locations
        .get(course_state.location_index)
        .ok_or_else(|| format!("Invalid location index {}", course_state.location_index))?;

    let obstacles_with_cameras = placed_query.iter().map(|(entity, placed, transform)| {
        let camera = child_of_query
            .iter()
            .find(|(_, child_of)| child_of.parent() == entity)
            .and_then(|(cam_entity, _)| camera_child_query.get(cam_entity).ok());
        (placed, transform, camera)
    });

    let course = build_course_data(
        course_state.name.clone(),
        location.name.clone(),
        obstacles_with_cameras,
        prop_query,
    );

    let save_data = LocationSaveData {
        course,
        inventory: course_state.inventory.clone(),
    };

    let path_str = location.save_path();
    let path = std::path::Path::new(&path_str);
    persistence::save_ron(&save_data, path)?;

    Ok(path_str)
}

#[allow(clippy::too_many_arguments)]
pub fn handle_save_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SaveCourseButton>)>,
    course_state: Res<EditorCourse>,
    placed_query: Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), (Without<PlacedObstacle>, Without<PlacedCamera>)>,
    camera_child_query: Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
    child_of_query: Query<(Entity, &ChildOf), With<PlacedCamera>>,
    location_registry: Res<LocationRegistry>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let camera_count = child_of_query.iter().count();
        if camera_count > 9 {
            warn!("Course has {camera_count} cameras (soft cap is 9). Consider reducing.");
        }

        match save_location_data(
            &course_state,
            &location_registry,
            &placed_query,
            prop_query.iter(),
            &child_of_query,
            &camera_child_query,
        ) {
            Ok(path_str) => {
                info!(
                    "Saved location '{}' ({} obstacles, {} cameras) to {}",
                    course_state.name,
                    placed_query.iter().count(),
                    camera_count,
                    path_str
                );

                commands.insert_resource(PendingThumbnailSave {
                    course_name: course_state.name.clone(),
                    frames_waited: 0,
                });
            }
            Err(e) => {
                error!("Failed to save: {e}");
                continue;
            }
        }
    }
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

#[allow(clippy::too_many_arguments)]
pub fn handle_start_race(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartRaceButton>)>,
    course_state: Res<EditorCourse>,
    placed_query: Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), (Without<PlacedObstacle>, Without<PlacedCamera>)>,
    camera_child_query: Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
    child_of_query: Query<(Entity, &ChildOf), With<PlacedCamera>>,
    location_registry: Res<LocationRegistry>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match save_location_data(
            &course_state,
            &location_registry,
            &placed_query,
            prop_query.iter(),
            &child_of_query,
            &camera_child_query,
        ) {
            Ok(path_str) => {
                info!("Saved '{}' — starting race", course_state.name);
                commands.insert_resource(SelectedCourse { path: path_str });

                commands.insert_resource(PendingThumbnailSave {
                    course_name: course_state.name.clone(),
                    frames_waited: 0,
                });
                commands.insert_resource(PendingRaceTransition);
            }
            Err(e) => {
                error!("Failed to save course before racing: {e}");
            }
        }
    }
}

/// Transitions to Race once the thumbnail readback entity has been spawned.
pub fn check_pending_race_transition(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    commands.remove_resource::<PendingRaceTransition>();
    next_state.set(AppState::HypeSetup);
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
