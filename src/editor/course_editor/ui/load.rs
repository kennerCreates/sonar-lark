use bevy::prelude::*;

use crate::course::data::{CourseData, PropKind};
use crate::course::loader::load_course_from_file;
use crate::editor::course_editor::{
    self, EditorCourse, EditorSelection, EditorTransform, PlacedCamera, PlacedObstacle, PlacedProp,
};
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::rendering::{CelLightDir, CelMaterial};
use crate::states::{AppState, LastEditedCourse, PendingEditorCourse};

use super::data::next_gate_order_from_instances;
use super::types::{CameraEditorMeshes, ExistingCourseButton, PropEditorMeshes};

/// Shared logic: despawn existing obstacles/props/cameras, load course data, spawn all.
#[allow(clippy::too_many_arguments)]
fn load_course_into_editor(
    commands: &mut Commands,
    selection: &mut EditorSelection,
    course_state: &mut EditorCourse,
    transform_state: &mut EditorTransform,
    placed_query: &Query<
        Entity,
        Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>,
    >,
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
    camera_meshes: Option<&CameraEditorMeshes>,
) {
    for entity in placed_query {
        commands.entity(entity).despawn();
    }

    course_editor::reset_editor_to_default(selection, course_state, transform_state);
    course_state.name = course.name.clone();
    transform_state.next_gate_order = next_gate_order_from_instances(&course.instances);

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
            commands.entity(entity).remove::<DespawnOnExit<AppState>>();
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: instance.obstacle_id.clone(),
                gate_order: instance.gate_order,
                gate_forward_flipped: instance.gate_forward_flipped,
            });

            // Spawn camera child if this obstacle has one
            if let Some(ref cam) = instance.camera
                && let Some(meshes) = camera_meshes
            {
                let material = if cam.is_primary {
                    meshes.primary_material.clone()
                } else {
                    meshes.material.clone()
                };
                let cam_transform = Transform::from_translation(cam.offset)
                    .with_rotation(cam.rotation);
                let cam_entity = commands
                    .spawn((
                        cam_transform,
                        Visibility::default(),
                        Mesh3d(meshes.mesh.clone()),
                        MeshMaterial3d(material),
                        PlacedCamera {
                            is_primary: cam.is_primary,
                            label: cam.label.clone(),
                        },
                    ))
                    .id();
                commands.entity(entity).add_child(cam_entity);
            }
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

    let camera_count = course
        .instances
        .iter()
        .filter(|i| i.camera.is_some())
        .count();
    info!(
        "Loaded course '{}' for editing ({} obstacles, {} props, {} cameras)",
        course.name,
        course.instances.len(),
        course.props.len(),
        camera_count,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn handle_load_button(
    mut commands: Commands,
    query: Query<(&Interaction, &ExistingCourseButton), Changed<Interaction>>,
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
    mut transform_state: ResMut<EditorTransform>,
    placed_query: Query<
        Entity,
        Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>,
    >,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
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
            &mut selection,
            &mut course_state,
            &mut transform_state,
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
            camera_meshes.as_deref(),
        );

        commands.insert_resource(LastEditedCourse {
            path: btn.0.clone(),
        });
    }
}

/// Loads a `PendingEditorCourse` once glTF assets are ready.
/// Gated by `run_if(resource_exists::<PendingEditorCourse>)` and `run_if(obstacles_gltf_ready)`.
#[allow(clippy::too_many_arguments)]
pub fn auto_load_pending_course(
    mut commands: Commands,
    pending: Res<PendingEditorCourse>,
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
    mut transform_state: ResMut<EditorTransform>,
    placed_query: Query<
        Entity,
        Or<(With<PlacedObstacle>, With<PlacedProp>, With<PlacedCamera>)>,
    >,
    library: Res<ObstacleLibrary>,
    gltf_handle: Res<ObstaclesGltfHandle>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
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
        &mut selection,
        &mut course_state,
        &mut transform_state,
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
        camera_meshes.as_deref(),
    );

    commands.insert_resource(LastEditedCourse {
        path: pending.path.clone(),
    });
    commands.remove_resource::<PendingEditorCourse>();
}
