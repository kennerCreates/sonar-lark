use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::editor::course_editor::{
    EditorTab, PlacedCamera, PlacedObstacle, PlacedProp, PlacementState,
};
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::ObstaclesGltfHandle;
use crate::palette;
use crate::states::AppState;
use crate::rendering::{cel_material_from_color, CelLightDir, CelMaterial};

use super::types::*;

// --- Interaction Systems ---

pub fn handle_palette_selection(
    mut commands: Commands,
    mut state: ResMut<PlacementState>,
    query: Query<(&Interaction, &PaletteButton), Changed<Interaction>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
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
                let order = state.next_gate_order;
                state.next_gate_order += 1;
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
            state.selected_entity = Some(entity);
            state.selected_palette_id = None;
        } else {
            warn!(
                "Failed to spawn obstacle '{}' (node '{}')",
                def.id.0, def.glb_node_name
            );
        }
    }
}

pub fn handle_name_field_focus(
    mut state: ResMut<PlacementState>,
    query: Query<&Interaction, (Changed<Interaction>, With<CourseNameField>)>,
    mut border: Query<&mut BorderColor, With<CourseNameField>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.editing_name = true;
            if let Ok(mut b) = border.single_mut() {
                *b = BorderColor::all(palette::SKY);
            }
        }
    }
}

pub fn handle_name_text_input(
    mut state: ResMut<PlacementState>,
    mut events: MessageReader<KeyboardInput>,
    mut border: Query<&mut BorderColor, With<CourseNameField>>,
) {
    if !state.editing_name {
        return;
    }

    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Enter | Key::Escape => {
                state.editing_name = false;
                if let Ok(mut b) = border.single_mut() {
                    *b = BorderColor::all(palette::STEEL);
                }
            }
            Key::Backspace => {
                state.course_name.pop();
            }
            Key::Space => {
                state.course_name.push('_');
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                        state.course_name.push(ch.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn update_display_values(
    state: Res<PlacementState>,
    mut name_text: Query<
        &mut Text,
        (With<CourseNameDisplayText>, Without<GateOrderModeText>),
    >,
    mut name_border: Query<
        &mut BorderColor,
        (With<CourseNameField>, Without<GateOrderModeButton>),
    >,
    mut gate_mode_text: Query<
        &mut Text,
        (With<GateOrderModeText>, Without<CourseNameDisplayText>),
    >,
    mut gate_mode_bg: Query<
        &mut BackgroundColor,
        (With<GateOrderModeButton>, Without<PaletteButton>),
    >,
    mut palette_bgs: Query<
        (&PaletteButton, &mut BackgroundColor),
        Without<GateOrderModeButton>,
    >,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut border) = name_border.single_mut() {
        *border = if state.editing_name {
            BorderColor::all(palette::SKY)
        } else {
            BorderColor::all(palette::STEEL)
        };
    }

    if let Ok(mut text) = name_text.single_mut() {
        **text = if state.course_name.is_empty() {
            if state.editing_name {
                "|".to_string()
            } else {
                "(type a name)".to_string()
            }
        } else if state.editing_name {
            format!("{}|", state.course_name)
        } else {
            state.course_name.clone()
        };
    }

    if let Ok(mut text) = gate_mode_text.single_mut() {
        **text = if state.gate_order_mode {
            "Gate Mode: ON".to_string()
        } else {
            "Gate Mode: OFF".to_string()
        };
    }

    if let Ok(mut bg) = gate_mode_bg.single_mut() {
        *bg = BackgroundColor(if state.gate_order_mode {
            TOGGLE_ON
        } else {
            TOGGLE_OFF
        });
    }

    for (btn, mut bg) in &mut palette_bgs {
        *bg = BackgroundColor(
            if state.selected_palette_id.as_ref() == Some(&btn.0) {
                BUTTON_SELECTED
            } else {
                BUTTON_NORMAL
            },
        );
    }
}

pub fn handle_button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            Or<(
                With<BackToWorkshopButton>,
                With<BackToMenuButton>,
                With<NewCourseButton>,
                With<ExistingCourseButton>,
                With<ClearGateOrdersButton>,
                With<DeleteCourseButton>,
                With<ConfirmDeleteYesButton>,
                With<ConfirmDeleteCancelButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg) in &mut query {
        match *interaction {
            Interaction::Pressed => *bg = BackgroundColor(BUTTON_PRESSED),
            Interaction::Hovered => *bg = BackgroundColor(BUTTON_HOVERED),
            Interaction::None => *bg = BackgroundColor(BUTTON_NORMAL),
        }
    }
}

pub fn handle_transform_mode_buttons(
    mut state: ResMut<PlacementState>,
    query: Query<(&Interaction, &TransformModeButton), Changed<Interaction>>,
) {
    for (interaction, btn) in &query {
        if *interaction == Interaction::Pressed {
            state.transform_mode = btn.0;
        }
    }
}

pub fn update_transform_mode_ui(
    state: Res<PlacementState>,
    mut buttons: Query<(&TransformModeButton, &mut BackgroundColor)>,
) {
    if !state.is_changed() {
        return;
    }
    for (btn, mut bg) in &mut buttons {
        *bg = BackgroundColor(if btn.0 == state.transform_mode {
            BUTTON_SELECTED
        } else {
            BUTTON_NORMAL
        });
    }
}

pub fn update_gate_count_display(
    placed_query: Query<&PlacedObstacle>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<GateCountText>>,
) {
    let gate_count = placed_query.iter().filter(|p| p.gate_order.is_some()).count();
    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = format!("Gates: {gate_count} (loop)");
        *color = if gate_count >= 2 {
            TextColor(palette::SKY)
        } else {
            TextColor(palette::BRONZE)
        };
    }
}

// --- Tab switching ---

pub fn handle_tab_switch(
    mut state: ResMut<PlacementState>,
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
    if tab == state.active_tab {
        return;
    }
    state.active_tab = tab;

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
        EditorTab::Obstacles => (BUTTON_SELECTED, BUTTON_NORMAL, BUTTON_NORMAL),
        EditorTab::Props => (BUTTON_NORMAL, BUTTON_SELECTED, BUTTON_NORMAL),
        EditorTab::Cameras => (BUTTON_NORMAL, BUTTON_NORMAL, BUTTON_SELECTED),
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

// --- Prop palette ---

pub fn handle_prop_palette_selection(
    mut commands: Commands,
    mut state: ResMut<PlacementState>,
    query: Query<(&Interaction, &PropPaletteButton), Changed<Interaction>>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Get or create prop meshes
        let (mesh, material) = if let Some(ref pm) = prop_meshes {
            match btn.0 {
                PropKind::ConfettiEmitter => (pm.confetti_mesh.clone(), pm.confetti_material.clone()),
                PropKind::ShellBurstEmitter => (pm.shell_mesh.clone(), pm.shell_material.clone()),
            }
        } else {
            let cube = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
            let color = match btn.0 {
                PropKind::ConfettiEmitter => palette::SUNSHINE,
                PropKind::ShellBurstEmitter => palette::TANGERINE,
            };
            let mat = cel_materials.add(cel_material_from_color(color, light_dir.0));
            commands.insert_resource(PropEditorMeshes {
                confetti_mesh: cube.clone(),
                shell_mesh: cube.clone(),
                confetti_material: mat.clone(),
                shell_material: mat.clone(),
            });
            (cube, mat)
        };

        let transform = Transform::from_translation(Vec3::ZERO);
        let entity = commands
            .spawn((
                transform,
                Visibility::default(),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                PlacedProp {
                    kind: btn.0,
                    color_override: None,
                },
            ))
            .id();

        state.selected_entity = Some(entity);
        state.selected_palette_id = None;
    }
}

pub fn setup_prop_editor_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
) {
    let cube = meshes.add(Cuboid::new(0.4, 0.4, 0.4));
    let confetti_mat = cel_materials.add(cel_material_from_color(palette::SUNSHINE, light_dir.0));
    let shell_mat = cel_materials.add(cel_material_from_color(palette::TANGERINE, light_dir.0));
    commands.insert_resource(PropEditorMeshes {
        confetti_mesh: cube.clone(),
        shell_mesh: cube,
        confetti_material: confetti_mat,
        shell_material: shell_mat,
    });
}

// --- Prop color cycling ---

pub fn handle_prop_color_cycle(
    query: Query<&Interaction, (Changed<Interaction>, With<PropColorButton>)>,
    state: Res<PlacementState>,
    mut prop_query: Query<&mut PlacedProp>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = state.selected_entity else {
            continue;
        };
        let Ok(mut prop) = prop_query.get_mut(entity) else {
            continue;
        };

        // Find current index in the cycle
        let current_idx = COLOR_CYCLE
            .iter()
            .position(|(_, c)| *c == prop.color_override)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % COLOR_CYCLE.len();
        prop.color_override = COLOR_CYCLE[next_idx].1;
    }
}

pub fn update_prop_color_label(
    state: Res<PlacementState>,
    prop_query: Query<&PlacedProp>,
    mut label_query: Query<(&mut Text, &mut TextColor), With<PropColorLabel>>,
) {
    let Ok((mut text, mut color)) = label_query.single_mut() else {
        return;
    };

    let prop = state
        .selected_entity
        .and_then(|e| prop_query.get(e).ok());

    if let Some(prop) = prop {
        let (name, _) = COLOR_CYCLE
            .iter()
            .find(|(_, c)| *c == prop.color_override)
            .unwrap_or(&COLOR_CYCLE[0]);
        **text = format!("Color: {name}");
        if let Some(rgba) = prop.color_override {
            *color = TextColor(Color::srgb(rgba[0], rgba[1], rgba[2]));
        } else {
            *color = TextColor(palette::SUNSHINE);
        }
    } else {
        **text = "Color: (select a prop)".to_string();
        *color = TextColor(palette::CHAINMAIL);
    }
}

// --- Camera placement ---

pub fn setup_camera_editor_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
) {
    let mesh = meshes.add(Cuboid::new(0.3, 0.3, 0.5));
    let material = cel_materials.add(cel_material_from_color(palette::SKY, light_dir.0));
    let primary_material =
        cel_materials.add(cel_material_from_color(palette::SUNSHINE, light_dir.0));
    commands.insert_resource(CameraEditorMeshes {
        mesh,
        material,
        primary_material,
    });
}

pub fn handle_camera_placement(
    mut commands: Commands,
    mut state: ResMut<PlacementState>,
    query: Query<&Interaction, (Changed<Interaction>, With<PlaceCameraButton>)>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(ref meshes) = camera_meshes else {
            continue;
        };

        let transform = Transform::from_translation(Vec3::ZERO);
        let entity = commands
            .spawn((
                transform,
                Visibility::default(),
                Mesh3d(meshes.mesh.clone()),
                MeshMaterial3d(meshes.material.clone()),
                PlacedCamera {
                    is_primary: false,
                    label: None,
                },
            ))
            .id();

        state.selected_entity = Some(entity);
        state.selected_palette_id = None;
    }
}

pub fn handle_camera_primary_toggle(
    query: Query<&Interaction, (Changed<Interaction>, With<CameraPrimaryToggle>)>,
    state: Res<PlacementState>,
    mut camera_query: Query<(Entity, &mut PlacedCamera, &mut MeshMaterial3d<CelMaterial>)>,
    camera_meshes: Option<Res<CameraEditorMeshes>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = state.selected_entity else {
            continue;
        };
        let Some(ref meshes) = camera_meshes else {
            continue;
        };

        // Toggle the selected camera's primary status
        let Ok((_, mut cam, _)) = camera_query.get_mut(entity) else {
            continue;
        };
        let new_primary = !cam.is_primary;
        cam.is_primary = new_primary;

        // If becoming primary, clear all others
        if new_primary {
            for (e, mut other_cam, mut other_mat) in &mut camera_query {
                if e != entity && other_cam.is_primary {
                    other_cam.is_primary = false;
                    other_mat.0 = meshes.material.clone();
                }
            }
        }

        // Update the toggled camera's material
        if let Ok((_, _, mut mat)) = camera_query.get_mut(entity) {
            mat.0 = if new_primary {
                meshes.primary_material.clone()
            } else {
                meshes.material.clone()
            };
        }
    }
}

pub fn update_camera_primary_label(
    state: Res<PlacementState>,
    camera_query: Query<&PlacedCamera>,
    mut label_query: Query<(&mut Text, &mut TextColor), With<CameraPrimaryLabel>>,
) {
    let Ok((mut text, mut color)) = label_query.single_mut() else {
        return;
    };

    let cam = state
        .selected_entity
        .and_then(|e| camera_query.get(e).ok());

    if let Some(cam) = cam {
        if cam.is_primary {
            **text = "Primary: Yes".to_string();
            *color = TextColor(palette::SUNSHINE);
        } else {
            **text = "Primary: No".to_string();
            *color = TextColor(palette::SKY);
        }
    } else {
        **text = "Primary: (select a camera)".to_string();
        *color = TextColor(palette::CHAINMAIL);
    }
}
