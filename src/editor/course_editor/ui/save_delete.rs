use bevy::prelude::*;
use std::path::Path;

use crate::course::loader::{delete_course, save_course};
use crate::editor::course_editor::{
    self, EditorCourse, EditorSelection, EditorTransform, PlacedCamera, PlacedObstacle, PlacedProp,
};
use crate::palette;
use crate::states::{AppState, LastEditedCourse};
use crate::ui_theme;

use super::discover::discover_existing_courses;
use super::right_panel::spawn_existing_course_button;
use super::data::build_course_data;
use super::types::*;

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

pub fn handle_new_course_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<NewCourseButton>)>,
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
    mut transform_state: ResMut<EditorTransform>,
    placed_query: Query<
        Entity,
        Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>,
    >,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        for entity in &placed_query {
            commands.entity(entity).despawn();
        }

        course_editor::reset_editor_to_default(
            &mut selection,
            &mut course_state,
            &mut transform_state,
        );

        commands.remove_resource::<LastEditedCourse>();
        info!("Cleared editor for new course");
    }
}

fn rebuild_courses_list(commands: &mut Commands, container: Entity) {
    let courses = discover_existing_courses();
    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        if courses.is_empty() {
            parent.spawn((
                Text::new("No courses found"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));
        } else {
            for course in &courses {
                spawn_existing_course_button(parent, &course.name, &course.path);
            }
        }
    });
}

fn spawn_delete_confirmation(commands: &mut Commands, container: Entity, display_name: &str) {
    commands.entity(container).despawn_related::<Children>();
    commands.entity(container).with_children(|parent| {
        parent.spawn((
            Text::new(format!("Delete \"{display_name}\"?")),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(palette::PEACH),
        ));
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Button,
                    ConfirmDeleteYesButton,
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(palette::MAROON),
                    BorderColor::all(palette::GRAPE),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Yes"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(palette::PALE_PINK),
                    ));
                });

                row.spawn((
                    Button,
                    ConfirmDeleteCancelButton,
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_NORMAL),
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Cancel"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(palette::SAND),
                    ));
                });
            });
    });
}

pub fn handle_save_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SaveCourseButton>)>,
    course_state: Res<EditorCourse>,
    placed_query: Query<(Entity, &PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), (Without<PlacedObstacle>, Without<PlacedCamera>)>,
    camera_child_query: Query<(&PlacedCamera, &Transform), Without<PlacedObstacle>>,
    child_of_query: Query<(Entity, &ChildOf), With<PlacedCamera>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if course_state.name.is_empty() {
            warn!("Cannot save: course name is empty");
            continue;
        }

        let camera_count = child_of_query.iter().count();
        if camera_count > 9 {
            warn!(
                "Course has {camera_count} cameras (soft cap is 9). Consider reducing."
            );
        }

        // For each obstacle, find its camera child (if any)
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
            }
            Err(e) => {
                error!("Failed to save course: {e}");
                continue;
            }
        }

        if let Ok(container) = existing_courses_container.single() {
            rebuild_courses_list(&mut commands, container);
        }
    }
}

pub fn handle_delete_button(
    mut commands: Commands,
    query: Query<(&Interaction, &DeleteCourseButton), Changed<Interaction>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
    pending: Option<Res<PendingCourseDelete>>,
) {
    if pending.is_some() {
        return;
    }

    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let path = &btn.0;
        let display_name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .trim_end_matches(".course")
            .to_string();

        commands.insert_resource(PendingCourseDelete {
            path: path.clone(),
            display_name: display_name.clone(),
        });

        if let Ok(container) = existing_courses_container.single() {
            spawn_delete_confirmation(&mut commands, container, &display_name);
        }

        break;
    }
}

pub fn handle_confirm_delete(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<ConfirmDeleteYesButton>)>,
    pending: Option<Res<PendingCourseDelete>>,
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
    mut transform_state: ResMut<EditorTransform>,
    placed_query: Query<
        Entity,
        Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>,
    >,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
    last_edited: Option<Res<LastEditedCourse>>,
) {
    let Some(pending) = pending else { return };

    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let path = Path::new(&pending.path);
        match delete_course(path) {
            Ok(()) => {
                info!("Deleted course '{}'", pending.display_name);

                // If we deleted the currently loaded course, clear the editor
                if course_state.name == pending.display_name {
                    for entity in &placed_query {
                        commands.entity(entity).despawn();
                    }
                    course_editor::reset_editor_to_default(
                        &mut selection,
                        &mut course_state,
                        &mut transform_state,
                    );
                }

                // If the deleted course was the last edited, remove that resource
                if let Some(last) = &last_edited
                    && last.path == pending.path
                {
                    commands.remove_resource::<LastEditedCourse>();
                }
            }
            Err(e) => {
                error!("Failed to delete course: {e}");
            }
        }

        commands.remove_resource::<PendingCourseDelete>();

        if let Ok(container) = existing_courses_container.single() {
            rebuild_courses_list(&mut commands, container);
        }

        break;
    }
}

pub fn handle_cancel_delete(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<ConfirmDeleteCancelButton>)>,
    pending: Option<Res<PendingCourseDelete>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
) {
    let Some(_pending) = pending else { return };

    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        commands.remove_resource::<PendingCourseDelete>();

        if let Ok(container) = existing_courses_container.single() {
            rebuild_courses_list(&mut commands, container);
        }

        break;
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
                // Continue from max existing gate order so the user can add gates
                // incrementally without losing previous assignments.
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
