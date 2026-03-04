use bevy::prelude::*;

use crate::editor::course_editor::{
    EditorCourse, EditorSelection, EditorTransform, PlacedCamera, PlacedFilter, PlacedObstacle,
    PlacedProp,
};
use crate::editor::undo::{
    CameraSnapshot, CourseEditorAction, UndoStack, remap_entity_in_stack,
};
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::rendering::{CelLightDir, CelMaterial};
use crate::states::AppState;

use super::ui::{CameraEditorMeshes, PropEditorMeshes, spawn_gate_camera};

/// System that handles Ctrl+Z (undo) and Ctrl+Y (redo) in the course editor.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn handle_course_undo_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    course_state: Res<EditorCourse>,
    mut selection: ResMut<EditorSelection>,
    mut transform_state: ResMut<EditorTransform>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
    mut placed_transforms: Query<&mut Transform, PlacedFilter>,
    mut component_queries: ParamSet<(
        Query<&mut PlacedObstacle>,
        Query<&mut PlacedProp>,
        Query<&mut PlacedCamera>,
    )>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    (node_assets, mesh_assets): (Res<Assets<bevy::gltf::GltfNode>>, Res<Assets<bevy::gltf::GltfMesh>>),
    (mut cel_materials, std_materials, light_dir): (ResMut<Assets<CelMaterial>>, Res<Assets<StandardMaterial>>, Res<CelLightDir>),
    prop_meshes: Option<Res<PropEditorMeshes>>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
) {
    if course_state.editing_name {
        return;
    }

    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl {
        return;
    }

    let is_undo = keyboard.just_pressed(KeyCode::KeyZ);
    let is_redo = keyboard.just_pressed(KeyCode::KeyY);
    if !is_undo && !is_redo {
        return;
    }

    let action = if is_undo {
        undo_stack.pop_undo()
    } else {
        undo_stack.pop_redo()
    };
    let Some(action) = action else { return };

    match action.clone() {
        CourseEditorAction::TransformChange {
            entity,
            before,
            after,
        } => {
            let target_transform = if is_undo { before } else { after };
            if let Ok(mut transform) = placed_transforms.get_mut(entity) {
                *transform = target_transform;
                selection.entity = Some(entity);
            }
        }

        CourseEditorAction::SpawnObstacle {
            entity,
            obstacle_id,
            transform,
            gate_order,
            gate_forward_flipped,
            camera,
            color_override,
        } => {
            if is_undo {
                // Undo spawn = despawn
                if selection.entity == Some(entity) {
                    selection.entity = None;
                }
                if gate_order.is_some() && transform_state.next_gate_order > 0 {
                    transform_state.next_gate_order -= 1;
                }
                commands.entity(entity).despawn();
            } else {
                // Redo spawn = respawn
                if let Some(new_entity) = respawn_obstacle(
                    &mut commands,
                    &library,
                    gltf_handle.as_deref(),
                    &gltf_assets,
                    &node_assets,
                    &mesh_assets,
                    &mut cel_materials,
                    &std_materials,
                    light_dir.0,
                    &obstacle_id,
                    transform,
                    gate_order,
                    gate_forward_flipped,
                    camera.as_ref(),
                    camera_meshes.as_deref(),
                    color_override,
                ) {
                    remap_entity_in_stack(&mut undo_stack, entity, new_entity);
                    selection.entity = Some(new_entity);
                    if gate_order.is_some() {
                        transform_state.next_gate_order += 1;
                    }
                }
            }
        }

        CourseEditorAction::SpawnProp {
            entity,
            kind,
            transform,
            color_override,
        } => {
            if is_undo {
                if selection.entity == Some(entity) {
                    selection.entity = None;
                }
                commands.entity(entity).despawn();
            } else if let Some(ref pm) = prop_meshes {
                let new_entity = respawn_prop(&mut commands, pm, kind, transform, color_override);
                remap_entity_in_stack(&mut undo_stack, entity, new_entity);
                selection.entity = Some(new_entity);
            }
        }

        CourseEditorAction::SpawnCamera {
            camera_entity,
            parent_gate,
            offset,
            rotation,
            is_primary,
        } => {
            if is_undo {
                if selection.entity == Some(camera_entity) {
                    selection.entity = None;
                }
                commands.entity(camera_entity).despawn();
            } else if let Some(ref cm) = camera_meshes {
                let new_entity =
                    spawn_gate_camera(&mut commands, parent_gate, cm, is_primary, offset, rotation);
                remap_entity_in_stack(&mut undo_stack, camera_entity, new_entity);
                selection.entity = Some(new_entity);
            }
        }

        CourseEditorAction::DeleteObstacle {
            old_entity,
            obstacle_id,
            transform,
            gate_order,
            gate_forward_flipped,
            camera,
            color_override,
        } => {
            if is_undo {
                // Undo delete = respawn
                if let Some(new_entity) = respawn_obstacle(
                    &mut commands,
                    &library,
                    gltf_handle.as_deref(),
                    &gltf_assets,
                    &node_assets,
                    &mesh_assets,
                    &mut cel_materials,
                    &std_materials,
                    light_dir.0,
                    &obstacle_id,
                    transform,
                    gate_order,
                    gate_forward_flipped,
                    camera.as_ref(),
                    camera_meshes.as_deref(),
                    color_override,
                ) {
                    remap_entity_in_stack(&mut undo_stack, old_entity, new_entity);
                    selection.entity = Some(new_entity);
                    if gate_order.is_some() {
                        transform_state.next_gate_order += 1;
                    }
                }
            } else {
                // Redo delete = despawn again
                // The entity was remapped, so old_entity should be current
                if selection.entity == Some(old_entity) {
                    selection.entity = None;
                }
                if gate_order.is_some() && transform_state.next_gate_order > 0 {
                    transform_state.next_gate_order -= 1;
                }
                commands.entity(old_entity).despawn();
            }
        }

        CourseEditorAction::DeleteProp {
            old_entity,
            kind,
            transform,
            color_override,
        } => {
            if is_undo {
                if let Some(ref pm) = prop_meshes {
                    let new_entity =
                        respawn_prop(&mut commands, pm, kind, transform, color_override);
                    remap_entity_in_stack(&mut undo_stack, old_entity, new_entity);
                    selection.entity = Some(new_entity);
                }
            } else {
                if selection.entity == Some(old_entity) {
                    selection.entity = None;
                }
                commands.entity(old_entity).despawn();
            }
        }

        CourseEditorAction::DeleteCamera {
            old_entity,
            parent_gate,
            offset,
            rotation,
            is_primary,
        } => {
            if is_undo {
                if let Some(ref cm) = camera_meshes {
                    let new_entity = spawn_gate_camera(
                        &mut commands,
                        parent_gate,
                        cm,
                        is_primary,
                        offset,
                        rotation,
                    );
                    remap_entity_in_stack(&mut undo_stack, old_entity, new_entity);
                    selection.entity = Some(new_entity);
                }
            } else {
                if selection.entity == Some(old_entity) {
                    selection.entity = None;
                }
                commands.entity(old_entity).despawn();
            }
        }

        CourseEditorAction::FlipGate { entity } => {
            if let Ok(mut placed) = component_queries.p0().get_mut(entity) {
                placed.gate_forward_flipped = !placed.gate_forward_flipped;
                selection.entity = Some(entity);
            }
        }

        CourseEditorAction::PropColorChange {
            entity,
            before,
            after,
        } => {
            let target = if is_undo { before } else { after };
            if let Ok(mut prop) = component_queries.p1().get_mut(entity) {
                prop.color_override = target;
                selection.entity = Some(entity);
            }
        }

        CourseEditorAction::GateColorChange {
            entity,
            before,
            after,
        } => {
            let target = if is_undo { before } else { after };
            if let Ok(mut placed) = component_queries.p0().get_mut(entity) {
                placed.color_override = target;
                selection.entity = Some(entity);
            }
        }
    }

    // Move the action to the opposite stack
    if is_undo {
        undo_stack.push_redo(action);
    } else {
        undo_stack.push_undo_only(action);
    }
}

/// Snapshot a placed entity's full state before deletion (for undo).
pub(super) fn snapshot_for_delete(
    entity: Entity,
    obstacle_query: &Query<&PlacedObstacle>,
    prop_query: &Query<&PlacedProp, Without<PlacedObstacle>>,
    camera_query: &Query<&PlacedCamera, (Without<PlacedObstacle>, Without<PlacedProp>)>,
    transform_query: &Query<&Transform, PlacedFilter>,
    camera_child_query: &Query<(Entity, &ChildOf, &PlacedCamera, &Transform)>,
) -> Option<CourseEditorAction> {
    let Ok(transform) = transform_query.get(entity) else {
        return None;
    };

    if let Ok(placed) = obstacle_query.get(entity) {
        // Check for a camera child
        let camera = camera_child_query
            .iter()
            .find(|(_, child_of, _, _)| child_of.parent() == entity)
            .map(|(_, _, cam, cam_tf)| CameraSnapshot {
                offset: cam_tf.translation,
                rotation: cam_tf.rotation,
                is_primary: cam.is_primary,
                label: cam.label.clone(),
            });

        return Some(CourseEditorAction::DeleteObstacle {
            old_entity: entity,
            obstacle_id: placed.obstacle_id.clone(),
            transform: *transform,
            gate_order: placed.gate_order,
            gate_forward_flipped: placed.gate_forward_flipped,
            camera,
            color_override: placed.color_override,
        });
    }

    if let Ok(prop) = prop_query.get(entity) {
        return Some(CourseEditorAction::DeleteProp {
            old_entity: entity,
            kind: prop.kind,
            transform: *transform,
            color_override: prop.color_override,
        });
    }

    if let Ok(cam) = camera_query.get(entity) {
        // Find parent gate
        let parent_gate = camera_child_query
            .iter()
            .find(|(e, _, _, _)| *e == entity)
            .map(|(_, child_of, _, _)| child_of.parent());

        if let Some(parent) = parent_gate {
            return Some(CourseEditorAction::DeleteCamera {
                old_entity: entity,
                parent_gate: parent,
                offset: transform.translation,
                rotation: transform.rotation,
                is_primary: cam.is_primary,
            });
        }
    }

    None
}

// --- Respawn helpers ---

#[allow(clippy::too_many_arguments)]
fn respawn_obstacle(
    commands: &mut Commands,
    library: &ObstacleLibrary,
    gltf_handle: Option<&ObstaclesGltfHandle>,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    cel_materials: &mut Assets<CelMaterial>,
    std_materials: &Assets<StandardMaterial>,
    light_dir: Vec3,
    obstacle_id: &crate::obstacle::definition::ObstacleId,
    transform: Transform,
    gate_order: Option<u32>,
    gate_forward_flipped: bool,
    camera: Option<&CameraSnapshot>,
    camera_meshes: Option<&CameraEditorMeshes>,
    color_override: Option<[f32; 4]>,
) -> Option<Entity> {
    let handle = gltf_handle?;
    let def = library.get(obstacle_id)?;

    let entity = crate::obstacle::spawning::spawn_obstacle(
        commands,
        gltf_assets,
        node_assets,
        mesh_assets,
        cel_materials,
        std_materials,
        light_dir,
        handle,
        obstacle_id,
        &def.glb_node_name,
        transform,
        def.model_offset,
        def.model_rotation,
        def.trigger_volume.as_ref(),
        gate_order,
        gate_forward_flipped,
        &def.collision_volumes,
        color_override.map(|rgba| Color::srgb(rgba[0], rgba[1], rgba[2])),
    )?;

    commands.entity(entity).remove::<DespawnOnExit<AppState>>();
    commands.entity(entity).insert(PlacedObstacle {
        obstacle_id: obstacle_id.clone(),
        gate_order,
        gate_forward_flipped,
        color_override,
    });

    // Respawn camera child if present
    if let Some(cam) = camera
        && let Some(cm) = camera_meshes
    {
        spawn_gate_camera(commands, entity, cm, cam.is_primary, cam.offset, cam.rotation);
    }

    Some(entity)
}

fn respawn_prop(
    commands: &mut Commands,
    meshes: &PropEditorMeshes,
    kind: crate::course::data::PropKind,
    transform: Transform,
    color_override: Option<[f32; 4]>,
) -> Entity {
    use crate::course::data::PropKind;
    let (mesh, material) = match kind {
        PropKind::ConfettiEmitter => (meshes.confetti_mesh.clone(), meshes.confetti_material.clone()),
        PropKind::ShellBurstEmitter => (meshes.shell_mesh.clone(), meshes.shell_material.clone()),
    };
    commands
        .spawn((
            transform,
            Visibility::default(),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            PlacedProp {
                kind,
                color_override,
            },
        ))
        .id()
}
