use bevy::prelude::*;
use std::path::Path;

use crate::course::data::{CourseData, ObstacleInstance, PropInstance, PropKind};
use crate::course::loader::{delete_course, load_course_from_file, save_course};
use crate::editor::course_editor::{
    LastEditedCourse, PendingEditorCourse, PlacedObstacle, PlacedProp, PlacementState,
};
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial};
use crate::states::{AppState, EditorMode};

use super::build::{discover_existing_courses, spawn_existing_course_button};
use super::types::*;

pub fn handle_back_to_workshop(
    query: Query<&Interaction, (Changed<Interaction>, With<BackToWorkshopButton>)>,
    mut next_mode: ResMut<NextState<EditorMode>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_mode.set(EditorMode::ObstacleWorkshop);
        }
    }
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
                spawn_existing_course_button(parent, &course.display_name, &course.path);
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
                    BackgroundColor(BUTTON_NORMAL),
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
    state: Res<PlacementState>,
    placed_query: Query<(&PlacedObstacle, &Transform)>,
    prop_query: Query<(&PlacedProp, &Transform), Without<PlacedObstacle>>,
    existing_courses_container: Query<Entity, With<ExistingCoursesContainer>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if state.course_name.is_empty() {
            warn!("Cannot save: course name is empty");
            continue;
        }

        let instances: Vec<ObstacleInstance> = placed_query
            .iter()
            .map(|(placed, transform)| ObstacleInstance {
                obstacle_id: placed.obstacle_id.clone(),
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
                gate_order: placed.gate_order,
                gate_forward_flipped: placed.gate_forward_flipped,
            })
            .collect();

        let props: Vec<PropInstance> = prop_query
            .iter()
            .map(|(prop, transform)| PropInstance {
                kind: prop.kind,
                translation: transform.translation,
                rotation: transform.rotation,
                color_override: prop.color_override,
            })
            .collect();

        let course = CourseData {
            name: state.course_name.clone(),
            instances,
            props,
        };

        let path_str = format!("assets/courses/{}.course.ron", state.course_name);
        let path = std::path::Path::new(&path_str);
        match save_course(&course, path) {
            Ok(()) => {
                info!(
                    "Saved course '{}' ({} obstacles) to {}",
                    state.course_name,
                    course.instances.len(),
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
    mut state: ResMut<PlacementState>,
    placed_query: Query<Entity, Or<(With<PlacedObstacle>, With<PlacedProp>)>>,
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
                if state.course_name == pending.display_name {
                    for entity in &placed_query {
                        commands.entity(entity).despawn();
                    }
                    state.selected_entity = None;
                    state.selected_palette_id = None;
                    state.course_name = "new_course".to_string();
                    state.next_gate_order = 0;
                }

                // If the deleted course was the last edited, remove that resource
                if let Some(last) = &last_edited {
                    if last.path == pending.path {
                        commands.remove_resource::<LastEditedCourse>();
                    }
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

/// Shared logic: despawn existing obstacles/props, load course data, spawn obstacles + props.
#[allow(clippy::too_many_arguments)]
fn load_course_into_editor(
    commands: &mut Commands,
    state: &mut PlacementState,
    placed_query: &Query<Entity, Or<(With<PlacedObstacle>, With<PlacedProp>)>>,
    library: &ObstacleLibrary,
    gltf_handle: &ObstaclesGltfHandle,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    cel_materials: &mut Assets<CelMaterial>,
    std_materials: &Assets<StandardMaterial>,
    light_dir: Vec3,
    course: &CourseData,
    prop_meshes: Option<&PropEditorMeshes>,
) {
    for entity in placed_query {
        commands.entity(entity).despawn();
    }

    state.selected_entity = None;
    state.selected_palette_id = None;
    state.course_name = course.name.clone();
    state.next_gate_order = course
        .instances
        .iter()
        .filter_map(|i| i.gate_order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    for instance in &course.instances {
        let Some(def) = library.get(&instance.obstacle_id) else {
            warn!(
                "Unknown obstacle '{}' in course file, skipping",
                instance.obstacle_id.0
            );
            continue;
        };

        let transform = Transform {
            translation: instance.translation,
            rotation: instance.rotation,
            scale: instance.scale,
        };

        let spawned = crate::obstacle::spawning::spawn_obstacle(
            commands,
            gltf_assets,
            node_assets,
            mesh_assets,
            cel_materials,
            std_materials,
            light_dir,
            gltf_handle,
            &def.id,
            &def.glb_node_name,
            transform,
            def.model_offset,
            def.trigger_volume.as_ref(),
            None,
            instance.gate_forward_flipped,
        );

        if let Some(entity) = spawned {
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: instance.obstacle_id.clone(),
                gate_order: instance.gate_order,
                gate_forward_flipped: instance.gate_forward_flipped,
            });
        } else {
            warn!(
                "Failed to spawn '{}' (node '{}') from loaded course",
                instance.obstacle_id.0, def.glb_node_name
            );
        }
    }

    // Spawn props from course data
    if let Some(meshes) = prop_meshes {
        for prop in &course.props {
            let (mesh, material) = match prop.kind {
                PropKind::ConfettiEmitter => {
                    (meshes.confetti_mesh.clone(), meshes.confetti_material.clone())
                }
                PropKind::ShellBurstEmitter => {
                    (meshes.shell_mesh.clone(), meshes.shell_material.clone())
                }
            };
            let transform =
                Transform::from_translation(prop.translation).with_rotation(prop.rotation);
            commands.spawn((
                transform,
                Visibility::default(),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                PlacedProp {
                    kind: prop.kind,
                    color_override: prop.color_override,
                },
            ));
        }
    }

    info!(
        "Loaded course '{}' for editing ({} obstacles, {} props)",
        course.name,
        course.instances.len(),
        course.props.len(),
    );
}

pub fn handle_load_button(
    mut commands: Commands,
    query: Query<(&Interaction, &ExistingCourseButton), Changed<Interaction>>,
    mut state: ResMut<PlacementState>,
    placed_query: Query<Entity, Or<(With<PlacedObstacle>, With<PlacedProp>)>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let path = std::path::Path::new(&btn.0);
        let course = match load_course_from_file(path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to load course: {e}");
                continue;
            }
        };

        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot spawn loaded course obstacles");
            continue;
        };

        load_course_into_editor(
            &mut commands,
            &mut state,
            &placed_query,
            &library,
            handle,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            &std_materials,
            light_dir.0,
            &course,
            prop_meshes.as_deref(),
        );

        commands.insert_resource(LastEditedCourse {
            path: btn.0.clone(),
        });
    }
}

/// Loads a `PendingEditorCourse` once glTF assets are ready.
/// Gated by `run_if(resource_exists::<PendingEditorCourse>)` and `run_if(obstacles_gltf_ready)`.
pub fn auto_load_pending_course(
    mut commands: Commands,
    pending: Res<PendingEditorCourse>,
    mut state: ResMut<PlacementState>,
    placed_query: Query<Entity, Or<(With<PlacedObstacle>, With<PlacedProp>)>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Res<ObstaclesGltfHandle>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
) {

    let path = std::path::Path::new(&pending.path);
    let course = match load_course_from_file(path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to auto-load course: {e}");
            commands.remove_resource::<PendingEditorCourse>();
            return;
        }
    };

    load_course_into_editor(
        &mut commands,
        &mut state,
        &placed_query,
        &library,
        &gltf_handle,
        &gltf_assets,
        &node_assets,
        &mesh_assets,
        &mut cel_materials,
        &std_materials,
        light_dir.0,
        &course,
        prop_meshes.as_deref(),
    );

    commands.insert_resource(LastEditedCourse {
        path: pending.path.clone(),
    });
    commands.remove_resource::<PendingEditorCourse>();
}

pub fn handle_gate_order_toggle(
    mut state: ResMut<PlacementState>,
    query: Query<&Interaction, (Changed<Interaction>, With<GateOrderModeButton>)>,
    placed_query: Query<&PlacedObstacle>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.gate_order_mode = !state.gate_order_mode;
            if state.gate_order_mode {
                // Continue from max existing gate order so the user can add gates
                // incrementally without losing previous assignments.
                state.next_gate_order = placed_query
                    .iter()
                    .filter_map(|p| p.gate_order)
                    .max()
                    .map(|m| m + 1)
                    .unwrap_or(0);
                state.selected_entity = None;
            }
        }
    }
}

pub fn handle_clear_gate_orders_button(
    mut state: ResMut<PlacementState>,
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
        state.next_gate_order = 0;
        info!("Cleared all gate orders");
    }
}
