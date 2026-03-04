use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::editor::course_editor::{EditorSelection, PlacedProp};
use crate::editor::undo::{CourseEditorAction, UndoStack};
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};

use super::types::*;

pub fn handle_prop_palette_selection(
    mut commands: Commands,
    mut selection: ResMut<EditorSelection>,
    query: Query<(&Interaction, &PropPaletteButton), Changed<Interaction>>,
    prop_meshes: Option<Res<PropEditorMeshes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
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

        undo_stack.push(CourseEditorAction::SpawnProp {
            entity,
            kind: btn.0,
            transform,
            color_override: None,
        });

        selection.entity = Some(entity);
        selection.palette_id = None;
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

pub fn handle_prop_color_cycle(
    query: Query<&Interaction, (Changed<Interaction>, With<PropColorButton>)>,
    selection: Res<EditorSelection>,
    mut prop_query: Query<&mut PlacedProp>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = selection.entity else {
            continue;
        };
        let Ok(mut prop) = prop_query.get_mut(entity) else {
            continue;
        };

        let before = prop.color_override;
        // Find current index in the cycle
        let current_idx = COLOR_CYCLE
            .iter()
            .position(|(_, c)| *c == prop.color_override)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % COLOR_CYCLE.len();
        prop.color_override = COLOR_CYCLE[next_idx].1;

        undo_stack.push(CourseEditorAction::PropColorChange {
            entity,
            before,
            after: prop.color_override,
        });
    }
}

pub fn update_prop_color_label(
    selection: Res<EditorSelection>,
    prop_query: Query<&PlacedProp>,
    mut label_query: Query<(&mut Text, &mut TextColor), With<PropColorLabel>>,
) {
    let Ok((mut text, mut color)) = label_query.single_mut() else {
        return;
    };

    let prop = selection
        .entity
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
