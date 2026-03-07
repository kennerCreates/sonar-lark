use bevy::prelude::*;

use crate::camera::orbit::MainCamera;
use crate::editor::course_editor::ui::{InventoryPaletteButton, PaletteButton};
use crate::editor::course_editor::{
    EditorCourse, EditorSelection, EditorTransform, PlacedObstacle,
};
use crate::editor::undo::{CameraSnapshot, CourseEditorAction, UndoStack};
use crate::league::LeagueState;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{ObstaclesGltfHandle, SpawnObstacleContext};
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};
use crate::states::AppState;

use super::ui::{spawn_gate_camera, DEFAULT_CAMERA_OFFSET, CameraEditorMeshes, DEFAULT_GATE_COLOR};

/// Marker component for the ghost obstacle shown during drag placement.
#[derive(Component)]
pub struct GhostObstacle;

/// Tracks an active drag-placement operation.
#[derive(Resource)]
pub struct DragPlacement {
    pub obstacle_id: ObstacleId,
    pub ghost_entity: Entity,
    /// Whether this gate came from inventory (free) or will be purchased.
    pub from_inventory: bool,
    /// False until the initial mouse button is released after picking up.
    /// Prevents the pick-up click from immediately placing.
    pub ready: bool,
}

/// Spawn a ghost obstacle (seafoam-colored) from the obstacle library.
fn spawn_ghost(
    commands: &mut Commands,
    gltf_assets: &Assets<bevy::gltf::Gltf>,
    node_assets: &Assets<bevy::gltf::GltfNode>,
    mesh_assets: &Assets<bevy::gltf::GltfMesh>,
    cel_materials: &mut Assets<CelMaterial>,
    light_dir: Vec3,
    gltf_handle: &ObstaclesGltfHandle,
    node_name: &str,
    model_offset: Vec3,
    model_rotation: Quat,
) -> Option<Entity> {
    let gltf = gltf_assets.get(&gltf_handle.0)?;
    let node_handle = gltf.named_nodes.get(node_name)?;
    let node = node_assets.get(node_handle)?;
    let gltf_mesh_handle = node.mesh.as_ref()?;
    let gltf_mesh = mesh_assets.get(gltf_mesh_handle)?;

    let ghost_mat = cel_materials.add(cel_material_from_color(palette::SEA_FOAM, light_dir));

    let mesh_transform = Transform {
        translation: model_offset,
        rotation: model_rotation * node.transform.rotation,
        scale: node.transform.scale,
    };

    let primitives: Vec<(Handle<Mesh>, MeshMaterial3d<CelMaterial>)> = gltf_mesh
        .primitives
        .iter()
        .map(|p| (p.mesh.clone(), MeshMaterial3d(ghost_mat.clone())))
        .collect();

    let parent = commands
        .spawn((
            Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)),
            Visibility::default(),
            GhostObstacle,
        ))
        .id();

    for (mesh, material) in primitives {
        commands
            .spawn((Mesh3d(mesh), material, mesh_transform))
            .set_parent_in_place(parent);
    }

    Some(parent)
}

/// System: when a palette button is pressed, begin a drag-placement.
#[allow(clippy::too_many_arguments)]
pub fn begin_drag_on_palette_press(
    mut commands: Commands,
    query: Query<(&Interaction, &PaletteButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    league: Option<Res<LeagueState>>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
    existing_drag: Option<Res<DragPlacement>>,
) {
    if existing_drag.is_some() {
        return;
    }

    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(def) = library.get(&btn.0) else {
            continue;
        };

        let cost = crate::course::data::gate_cost(&btn.0 .0, &library);
        if cost > 0 {
            let money = league.as_ref().map_or(0.0, |l| l.money);
            if money < cost as f32 {
                warn!("Cannot afford {} (${cost}, have ${:.0})", btn.0 .0, money);
                continue;
            }
        }
        let from_inventory = false;

        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot place obstacle");
            continue;
        };

        let Some(ghost_entity) = spawn_ghost(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            light_dir.0,
            handle,
            &def.glb_node_name,
            def.model_offset,
            def.model_rotation,
        ) else {
            warn!("Failed to spawn ghost for '{}'", def.id.0);
            continue;
        };

        commands.insert_resource(DragPlacement {
            obstacle_id: btn.0.clone(),
            ghost_entity,
            from_inventory,
            ready: false,
        });
        break;
    }
}

/// System: when an inventory button is pressed, begin a drag-placement from inventory.
#[allow(clippy::too_many_arguments)]
pub fn begin_drag_on_inventory_press(
    mut commands: Commands,
    query: Query<(&Interaction, &InventoryPaletteButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    course_state: Res<EditorCourse>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
    existing_drag: Option<Res<DragPlacement>>,
) {
    if existing_drag.is_some() {
        return;
    }

    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if course_state.inventory.count(&btn.0) == 0 {
            continue;
        }

        let Some(def) = library.get(&btn.0) else {
            continue;
        };

        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot place obstacle");
            continue;
        };

        let Some(ghost_entity) = spawn_ghost(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            light_dir.0,
            handle,
            &def.glb_node_name,
            def.model_offset,
            def.model_rotation,
        ) else {
            warn!("Failed to spawn ghost for '{}'", def.id.0);
            continue;
        };

        commands.insert_resource(DragPlacement {
            obstacle_id: btn.0.clone(),
            ghost_entity,
            from_inventory: true,
            ready: false,
        });
        break;
    }
}

/// System: move ghost obstacle to follow mouse on the Y=0 ground plane.
pub fn update_ghost_position(
    drag: Option<Res<DragPlacement>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut ghost_query: Query<&mut Transform, With<GhostObstacle>>,
) {
    let Some(drag) = drag else { return };

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    let Some(distance) = ray_y_plane_intersection(ray) else {
        return;
    };
    let world_pos = ray.origin + ray.direction * distance;

    if let Ok(mut transform) = ghost_query.get_mut(drag.ghost_entity) {
        transform.translation = Vec3::new(world_pos.x, 0.0, world_pos.z);
    }
}

/// System: once the initial mouse button is released after picking up, mark drag as ready.
pub fn arm_drag_placement(
    mut drag: Option<ResMut<DragPlacement>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    let Some(ref mut drag) = drag else { return };
    if !drag.ready && !mouse_buttons.pressed(MouseButton::Left) {
        drag.ready = true;
    }
}

/// System: on left click (while drag is ready), finalize placement at ghost position.
/// Gated by `resource_exists::<DragPlacement>` so `drag` is always available.
#[allow(clippy::too_many_arguments)]
pub fn finalize_drag_placement(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    drag: Res<DragPlacement>,
    ghost_query: Query<&Transform, With<GhostObstacle>>,
    interaction_query: Query<&Interaction>,
    mut selection: ResMut<EditorSelection>,
    mut transform_state: ResMut<EditorTransform>,
    mut course_state: ResMut<EditorCourse>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
) {
    if !drag.ready || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let ghost_pos = ghost_query
        .get(drag.ghost_entity)
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    commands.entity(drag.ghost_entity).despawn();

    let over_ui = interaction_query.iter().any(|i| *i != Interaction::None);
    if over_ui || ghost_pos.y < -50.0 {
        commands.remove_resource::<DragPlacement>();
        return;
    }

    let Some(def) = library.get(&drag.obstacle_id) else {
        commands.remove_resource::<DragPlacement>();
        return;
    };

    let Some(handle) = &gltf_handle else {
        commands.remove_resource::<DragPlacement>();
        return;
    };

    let transform = Transform::from_translation(ghost_pos);
    let mut ctx = SpawnObstacleContext::from_res(
        &gltf_assets,
        &node_assets,
        &mesh_assets,
        &mut cel_materials,
        &std_materials,
        &light_dir,
        handle,
    );

    let spawned = crate::obstacle::spawning::spawn_obstacle(
        &mut commands,
        &mut ctx,
        &def.id,
        &def.glb_node_name,
        transform,
        def.model_offset,
        def.model_rotation,
        def.trigger_volume.as_ref(),
        None,
        false,
        &def.collision_volumes,
        if def.is_gate {
            Some(crate::palette::VANILLA)
        } else {
            None
        },
    );

    if let Some(entity) = spawned {
        let gate_order = if def.is_gate {
            let order = transform_state.next_gate_order;
            transform_state.next_gate_order += 1;
            Some(order)
        } else {
            None
        };
        let color_override = if def.is_gate {
            Some(DEFAULT_GATE_COLOR)
        } else {
            None
        };
        commands
            .entity(entity)
            .remove::<DespawnOnExit<AppState>>();
        commands.entity(entity).insert(PlacedObstacle {
            obstacle_id: drag.obstacle_id.clone(),
            gate_order,
            gate_forward_flipped: false,
            color_override,
            from_inventory: drag.from_inventory,
        });

        if drag.from_inventory {
            course_state.inventory.remove(&drag.obstacle_id);
        } else {
            let cost = crate::course::data::gate_cost(&drag.obstacle_id.0, &library);
            if cost > 0 {
                // Money deduction handled by post_finalize_drag system
            }
        }

        selection.entity = Some(entity);
        selection.palette_id = None;

        // Store placement info for post-finalize
        commands.insert_resource(PendingPlacement {
            entity,
            obstacle_id: drag.obstacle_id.clone(),
            transform,
            gate_order,
            color_override,
            from_inventory: drag.from_inventory,
            default_camera_offset: def
                .default_camera
                .as_ref()
                .map(|c| c.offset)
                .unwrap_or(DEFAULT_CAMERA_OFFSET),
            default_camera_rotation: def
                .default_camera
                .as_ref()
                .map(|c| c.rotation)
                .unwrap_or(Quat::IDENTITY),
        });
    }

    commands.remove_resource::<DragPlacement>();
}

/// Temporary resource to carry data from finalize to post_finalize
/// (split to stay under Bevy's 16-param limit).
#[derive(Resource)]
pub struct PendingPlacement {
    entity: Entity,
    obstacle_id: ObstacleId,
    transform: Transform,
    gate_order: Option<u32>,
    color_override: Option<[f32; 4]>,
    from_inventory: bool,
    default_camera_offset: Vec3,
    default_camera_rotation: Quat,
}

/// System: apply undo tracking, camera spawning, and money deduction after placement.
/// Gated by `resource_exists::<PendingPlacement>`.
pub fn post_finalize_drag(
    mut commands: Commands,
    pending: Res<PendingPlacement>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
    mut league: Option<ResMut<LeagueState>>,
    library: Res<ObstacleLibrary>,
) {
    let mut camera_snapshot = None;
    if pending.gate_order == Some(0)
        && let Some(ref cam_meshes) = camera_meshes
    {
        spawn_gate_camera(
            &mut commands,
            pending.entity,
            cam_meshes,
            true,
            pending.default_camera_offset,
            pending.default_camera_rotation,
        );
        camera_snapshot = Some(CameraSnapshot {
            offset: pending.default_camera_offset,
            rotation: pending.default_camera_rotation,
            is_primary: true,
            label: None,
        });
    }

    // Deduct money if not from inventory
    if !pending.from_inventory {
        let cost = crate::course::data::gate_cost(&pending.obstacle_id.0, &library);
        if cost > 0
            && let Some(ref mut league) = league
        {
            league.money -= cost as f32;
        }
    }

    undo_stack.push(CourseEditorAction::SpawnObstacle {
        entity: pending.entity,
        obstacle_id: pending.obstacle_id.clone(),
        transform: pending.transform,
        gate_order: pending.gate_order,
        gate_forward_flipped: false,
        camera: camera_snapshot,
        color_override: pending.color_override,
        from_inventory: pending.from_inventory,
    });

    commands.remove_resource::<PendingPlacement>();
}

/// System: cancel drag placement on Escape key or right-click.
pub fn cancel_drag_on_escape(
    mut commands: Commands,
    drag: Option<Res<DragPlacement>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    let Some(drag) = drag else { return };

    if keyboard.just_pressed(KeyCode::Escape)
        || mouse_buttons.just_pressed(MouseButton::Right)
    {
        commands.entity(drag.ghost_entity).despawn();
        commands.remove_resource::<DragPlacement>();
    }
}

/// Intersect a ray with the Y=0 horizontal plane.
fn ray_y_plane_intersection(ray: Ray3d) -> Option<f32> {
    let denom = ray.direction.y;
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = -ray.origin.y / denom;
    if t >= 0.0 { Some(t) } else { None }
}
