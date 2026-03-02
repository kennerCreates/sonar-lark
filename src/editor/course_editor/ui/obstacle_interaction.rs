use bevy::prelude::*;

use crate::editor::EditorTab;
use crate::editor::course_editor::{EditorSelection, EditorTransform, EditorUI, PlacedObstacle};
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::rendering::{CelLightDir, CelMaterial};
use crate::states::AppState;
use crate::ui_theme;

use super::camera_interaction::{DEFAULT_CAMERA_OFFSET, spawn_gate_camera};
use super::types::*;

pub fn handle_palette_selection(
    mut commands: Commands,
    mut selection: ResMut<EditorSelection>,
    mut transform_state: ResMut<EditorTransform>,
    query: Query<(&Interaction, &PaletteButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(def) = library.get(&btn.0) else {
            continue;
        };
        let Some(handle) = &gltf_handle else {
            warn!("glTF not loaded yet, cannot place obstacle");
            continue;
        };

        let transform = Transform::from_translation(Vec3::ZERO);
        let spawned = crate::obstacle::spawning::spawn_obstacle(
            &mut commands,
            &gltf_assets,
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            &std_materials,
            light_dir.0,
            handle,
            &def.id,
            &def.glb_node_name,
            transform,
            def.model_offset,
            def.trigger_volume.as_ref(),
            None,
            false,
            def.collision_volume.as_ref(),
        );

        if let Some(entity) = spawned {
            let gate_order = if def.is_gate {
                let order = transform_state.next_gate_order;
                transform_state.next_gate_order += 1;
                Some(order)
            } else {
                None
            };
            commands.entity(entity).remove::<DespawnOnExit<AppState>>();
            commands.entity(entity).insert(PlacedObstacle {
                obstacle_id: btn.0.clone(),
                gate_order,
                gate_forward_flipped: false,
            });

            // Auto-spawn a primary camera on the first gate (gate_order == 0)
            if gate_order == Some(0)
                && let Some(ref cam_meshes) = camera_meshes
            {
                spawn_gate_camera(
                    &mut commands,
                    entity,
                    cam_meshes,
                    true,
                    DEFAULT_CAMERA_OFFSET,
                    Quat::IDENTITY,
                );
            }

            selection.entity = Some(entity);
            selection.palette_id = None;
        } else {
            warn!(
                "Failed to spawn obstacle '{}' (node '{}')",
                def.id.0, def.glb_node_name
            );
        }
    }
}

pub fn handle_tab_switch(
    mut editor_ui: ResMut<EditorUI>,
    obstacle_tab: Query<&Interaction, (Changed<Interaction>, With<ObstacleTabButton>)>,
    props_tab: Query<&Interaction, (Changed<Interaction>, With<PropsTabButton>)>,
    cameras_tab: Query<&Interaction, (Changed<Interaction>, With<CamerasTabButton>)>,
    mut obstacle_content: Query<
        &mut Node,
        (
            With<ObstaclePaletteContent>,
            Without<PropPaletteContent>,
            Without<CameraPaletteContent>,
        ),
    >,
    mut prop_content: Query<
        &mut Node,
        (
            With<PropPaletteContent>,
            Without<ObstaclePaletteContent>,
            Without<CameraPaletteContent>,
        ),
    >,
    mut camera_content: Query<
        &mut Node,
        (
            With<CameraPaletteContent>,
            Without<ObstaclePaletteContent>,
            Without<PropPaletteContent>,
        ),
    >,
    mut obstacle_tab_bg: Query<
        &mut BackgroundColor,
        (
            With<ObstacleTabButton>,
            Without<PropsTabButton>,
            Without<CamerasTabButton>,
        ),
    >,
    mut props_tab_bg: Query<
        &mut BackgroundColor,
        (
            With<PropsTabButton>,
            Without<ObstacleTabButton>,
            Without<CamerasTabButton>,
        ),
    >,
    mut cameras_tab_bg: Query<
        &mut BackgroundColor,
        (
            With<CamerasTabButton>,
            Without<ObstacleTabButton>,
            Without<PropsTabButton>,
        ),
    >,
) {
    let mut new_tab = None;

    for interaction in &obstacle_tab {
        if *interaction == Interaction::Pressed {
            new_tab = Some(EditorTab::Obstacles);
        }
    }
    for interaction in &props_tab {
        if *interaction == Interaction::Pressed {
            new_tab = Some(EditorTab::Props);
        }
    }
    for interaction in &cameras_tab {
        if *interaction == Interaction::Pressed {
            new_tab = Some(EditorTab::Cameras);
        }
    }

    let Some(tab) = new_tab else { return };
    if tab == editor_ui.active_tab {
        return;
    }
    editor_ui.active_tab = tab;

    let (obs_display, prop_display, cam_display) = match tab {
        EditorTab::Obstacles => (Display::Flex, Display::None, Display::None),
        EditorTab::Props => (Display::None, Display::Flex, Display::None),
        EditorTab::Cameras => (Display::None, Display::None, Display::Flex),
    };

    if let Ok(mut node) = obstacle_content.single_mut() {
        node.display = obs_display;
    }
    if let Ok(mut node) = prop_content.single_mut() {
        node.display = prop_display;
    }
    if let Ok(mut node) = camera_content.single_mut() {
        node.display = cam_display;
    }

    let (obs_bg, prop_bg, cam_bg) = match tab {
        EditorTab::Obstacles => (ui_theme::BUTTON_SELECTED, ui_theme::BUTTON_NORMAL, ui_theme::BUTTON_NORMAL),
        EditorTab::Props => (ui_theme::BUTTON_NORMAL, ui_theme::BUTTON_SELECTED, ui_theme::BUTTON_NORMAL),
        EditorTab::Cameras => (ui_theme::BUTTON_NORMAL, ui_theme::BUTTON_NORMAL, ui_theme::BUTTON_SELECTED),
    };
    if let Ok(mut bg) = obstacle_tab_bg.single_mut() {
        *bg = BackgroundColor(obs_bg);
    }
    if let Ok(mut bg) = props_tab_bg.single_mut() {
        *bg = BackgroundColor(prop_bg);
    }
    if let Ok(mut bg) = cameras_tab_bg.single_mut() {
        *bg = BackgroundColor(cam_bg);
    }
}
