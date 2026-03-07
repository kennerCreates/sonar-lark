use bevy::prelude::*;

use crate::course::data::{CourseData, PropKind};
use crate::course::location::{LocationSaveData, SelectedLocation};
use crate::editor::course_editor::{
    self, EditorCourse, EditorSelection, EditorTransform, PlacedCamera, PlacedFilter, PlacedObstacle,
    PlacedProp,
};
use crate::editor::undo::{CourseEditorAction, UndoStack};
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{ObstaclesGltfHandle, SpawnObstacleContext};
use crate::persistence;
use crate::rendering::{CelLightDir, CelMaterial};
use crate::states::{AppState, PendingEditorCourse};

use super::data::next_gate_order_from_instances;
use super::types::{CameraEditorMeshes, PropEditorMeshes};

/// Shared logic: despawn existing obstacles/props/cameras, load course data, spawn all.
pub(crate) fn load_course_into_editor(
    commands: &mut Commands,
    selection: &mut EditorSelection,
    course_state: &mut EditorCourse,
    transform_state: &mut EditorTransform,
    placed_query: &Query<
        Entity,
        PlacedFilter,
    >,
    library: &ObstacleLibrary,
    ctx: &mut SpawnObstacleContext,
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
            ctx,
            &def.id,
            &def.glb_node_name,
            transform,
            def.model_offset,
            def.model_rotation,
            def.trigger_volume.as_ref(),
            None,
            instance.gate_forward_flipped,
            &def.collision_volumes,
            instance.color_override.map(|rgba| Color::srgb(rgba[0], rgba[1], rgba[2])),
        );

        if let Some(entity) = spawned {
            commands.entity(entity).remove::<DespawnOnExit<AppState>>();
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: instance.obstacle_id.clone(),
                gate_order: instance.gate_order,
                gate_forward_flipped: instance.gate_forward_flipped,
                color_override: instance.color_override,
                from_inventory: false, // loaded gates are already owned
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

/// Loads a `PendingEditorCourse` once glTF assets are ready.
/// Tries LocationSaveData format first, falls back to raw CourseData.
#[allow(clippy::too_many_arguments)]
pub fn auto_load_pending_course(
    mut commands: Commands,
    pending: Res<PendingEditorCourse>,
    mut selection: ResMut<EditorSelection>,
    mut course_state: ResMut<EditorCourse>,
    mut transform_state: ResMut<EditorTransform>,
    selected_location: Option<Res<SelectedLocation>>,
    placed_query: Query<
        Entity,
        PlacedFilter,
    >,
    library: Res<ObstacleLibrary>,
    gltf_handle: Res<ObstaclesGltfHandle>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    (node_assets, mesh_assets): (Res<Assets<bevy::gltf::GltfNode>>, Res<Assets<bevy::gltf::GltfMesh>>),
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
    (camera_meshes, mut undo_stack): (Option<Res<CameraEditorMeshes>>, ResMut<UndoStack<CourseEditorAction>>),
) {
    let path = std::path::Path::new(&pending.path);

    // Try LocationSaveData first, fall back to raw CourseData
    let (course, inventory) = if let Ok(save_data) = persistence::load_ron::<LocationSaveData>(path)
    {
        (save_data.course, save_data.inventory)
    } else if let Ok(course) = persistence::load_ron::<CourseData>(path) {
        (course, Default::default())
    } else {
        error!("Failed to load course from: {}", pending.path);
        commands.remove_resource::<PendingEditorCourse>();
        return;
    };

    let mut ctx = SpawnObstacleContext::from_res(
        &gltf_assets,
        &node_assets,
        &mesh_assets,
        &mut cel_materials,
        &std_materials,
        &light_dir,
        &gltf_handle,
    );

    load_course_into_editor(
        &mut commands,
        &mut selection,
        &mut course_state,
        &mut transform_state,
        &placed_query,
        &library,
        &mut ctx,
        &course,
        prop_meshes.as_deref(),
        camera_meshes.as_deref(),
    );

    // Set inventory and location index from loaded data
    course_state.inventory = inventory;
    if let Some(loc) = selected_location.as_ref() {
        course_state.location_index = loc.0;
    }

    undo_stack.clear();
    commands.remove_resource::<PendingEditorCourse>();
}
