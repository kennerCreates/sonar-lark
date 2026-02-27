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

/// Build a `CourseData` from placed obstacle and prop data.
/// Pure function — no ECS dependencies.
pub fn build_course_data<'a>(
    name: String,
    obstacles: impl IntoIterator<Item = (&'a PlacedObstacle, &'a Transform)>,
    props: impl IntoIterator<Item = (&'a PlacedProp, &'a Transform)>,
) -> CourseData {
    let instances = obstacles
        .into_iter()
        .map(|(placed, transform)| ObstacleInstance {
            obstacle_id: placed.obstacle_id.clone(),
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
            gate_order: placed.gate_order,
            gate_forward_flipped: placed.gate_forward_flipped,
        })
        .collect();

    let props = props
        .into_iter()
        .map(|(prop, transform)| PropInstance {
            kind: prop.kind,
            translation: transform.translation,
            rotation: transform.rotation,
            color_override: prop.color_override,
        })
        .collect();

    CourseData {
        name,
        instances,
        props,
    }
}

/// Compute the next gate order value from a course's obstacle instances.
/// Returns one past the maximum existing gate order, or 0 if none.
pub fn next_gate_order_from_instances(instances: &[ObstacleInstance]) -> u32 {
    instances
        .iter()
        .filter_map(|i| i.gate_order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0)
}

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

        let course = build_course_data(
            state.course_name.clone(),
            placed_query.iter(),
            prop_query.iter(),
        );

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
    state.next_gate_order = next_gate_order_from_instances(&course.instances);

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
            def.collision_volume.as_ref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obstacle::definition::ObstacleId;
    use bevy::math::{Quat, Vec3};

    // --- build_course_data ---

    #[test]
    fn build_course_data_empty() {
        let course = build_course_data(
            "empty".to_string(),
            Vec::<(&PlacedObstacle, &Transform)>::new(),
            Vec::<(&PlacedProp, &Transform)>::new(),
        );
        assert_eq!(course.name, "empty");
        assert!(course.instances.is_empty());
        assert!(course.props.is_empty());
    }

    #[test]
    fn build_course_data_maps_obstacle_fields() {
        let placed = PlacedObstacle {
            obstacle_id: ObstacleId("gate_air".to_string()),
            gate_order: Some(2),
            gate_forward_flipped: true,
        };
        let transform = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(1.0),
            scale: Vec3::new(0.5, 1.0, 1.5),
        };

        let course = build_course_data(
            "test".to_string(),
            vec![(&placed, &transform)],
            Vec::<(&PlacedProp, &Transform)>::new(),
        );

        assert_eq!(course.instances.len(), 1);
        let inst = &course.instances[0];
        assert_eq!(inst.obstacle_id.0, "gate_air");
        assert_eq!(inst.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(inst.scale, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(inst.gate_order, Some(2));
        assert!(inst.gate_forward_flipped);
    }

    #[test]
    fn build_course_data_maps_prop_fields() {
        let prop = PlacedProp {
            kind: PropKind::ConfettiEmitter,
            color_override: Some([1.0, 0.0, 0.0, 1.0]),
        };
        let transform = Transform::from_translation(Vec3::new(5.0, 0.0, -10.0));

        let course = build_course_data(
            "props_test".to_string(),
            Vec::<(&PlacedObstacle, &Transform)>::new(),
            vec![(&prop, &transform)],
        );

        assert_eq!(course.props.len(), 1);
        let p = &course.props[0];
        assert_eq!(p.kind, PropKind::ConfettiEmitter);
        assert_eq!(p.translation, Vec3::new(5.0, 0.0, -10.0));
        assert_eq!(p.color_override, Some([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn build_course_data_multiple_obstacles_and_props() {
        let obs1 = PlacedObstacle {
            obstacle_id: ObstacleId("gate1".to_string()),
            gate_order: Some(0),
            gate_forward_flipped: false,
        };
        let obs2 = PlacedObstacle {
            obstacle_id: ObstacleId("wall".to_string()),
            gate_order: None,
            gate_forward_flipped: false,
        };
        let t1 = Transform::from_translation(Vec3::ZERO);
        let t2 = Transform::from_translation(Vec3::X);

        let prop = PlacedProp {
            kind: PropKind::ShellBurstEmitter,
            color_override: None,
        };
        let tp = Transform::from_translation(Vec3::Y);

        let course = build_course_data(
            "mixed".to_string(),
            vec![(&obs1, &t1), (&obs2, &t2)],
            vec![(&prop, &tp)],
        );

        assert_eq!(course.instances.len(), 2);
        assert_eq!(course.props.len(), 1);
        assert_eq!(course.instances[0].obstacle_id.0, "gate1");
        assert_eq!(course.instances[1].obstacle_id.0, "wall");
        assert_eq!(course.props[0].kind, PropKind::ShellBurstEmitter);
        assert!(course.props[0].color_override.is_none());
    }

    // --- next_gate_order_from_instances ---

    #[test]
    fn next_gate_order_empty() {
        assert_eq!(next_gate_order_from_instances(&[]), 0);
    }

    #[test]
    fn next_gate_order_no_gates() {
        let instances = vec![ObstacleInstance {
            obstacle_id: ObstacleId("wall".to_string()),
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gate_order: None,
            gate_forward_flipped: false,
        }];
        assert_eq!(next_gate_order_from_instances(&instances), 0);
    }

    #[test]
    fn next_gate_order_sequential() {
        let instances = vec![
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(0),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(1),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(2),
                gate_forward_flipped: false,
            },
        ];
        assert_eq!(next_gate_order_from_instances(&instances), 3);
    }

    #[test]
    fn next_gate_order_with_gaps() {
        let instances = vec![
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(0),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(5),
                gate_forward_flipped: false,
            },
        ];
        assert_eq!(next_gate_order_from_instances(&instances), 6);
    }

    #[test]
    fn next_gate_order_mixed_gates_and_walls() {
        let instances = vec![
            ObstacleInstance {
                obstacle_id: ObstacleId("gate".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(3),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("wall".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: None,
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("gate".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(1),
                gate_forward_flipped: false,
            },
        ];
        assert_eq!(next_gate_order_from_instances(&instances), 4);
    }
}
