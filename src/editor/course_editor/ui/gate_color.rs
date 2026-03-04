use bevy::prelude::*;

use crate::dev_menu::color_picker_data::PALETTE_COLORS;
use crate::editor::course_editor::{EditorSelection, PlacedObstacle};
use crate::editor::undo::{CourseEditorAction, UndoStack};
use crate::obstacle::spawning::gate_color;
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};

use super::types::*;

pub fn handle_gate_color_click(
    query: Query<(&Interaction, &GateColorCell), Changed<Interaction>>,
    selection: Res<EditorSelection>,
    mut placed_query: Query<&mut PlacedObstacle>,
    child_of_query: Query<(Entity, &ChildOf)>,
    mut material_query: Query<&mut MeshMaterial3d<CelMaterial>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
) {
    for (interaction, cell) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = selection.entity else {
            continue;
        };
        let Ok(mut placed) = placed_query.get_mut(entity) else {
            continue;
        };

        let rgb = &PALETTE_COLORS[cell.0].1;
        let new_color = Some([rgb[0], rgb[1], rgb[2], 1.0]);
        let before = placed.color_override;
        placed.color_override = new_color;

        undo_stack.push(CourseEditorAction::GateColorChange {
            entity,
            before,
            after: new_color,
        });

        apply_gate_color_to_children(
            entity,
            new_color,
            &placed.obstacle_id,
            &child_of_query,
            &mut material_query,
            &mut cel_materials,
            light_dir.0,
        );
    }
}

pub fn handle_gate_color_default(
    query: Query<&Interaction, (Changed<Interaction>, With<GateColorDefaultButton>)>,
    selection: Res<EditorSelection>,
    mut placed_query: Query<&mut PlacedObstacle>,
    child_of_query: Query<(Entity, &ChildOf)>,
    mut material_query: Query<&mut MeshMaterial3d<CelMaterial>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
    mut undo_stack: ResMut<UndoStack<CourseEditorAction>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entity) = selection.entity else {
            continue;
        };
        let Ok(mut placed) = placed_query.get_mut(entity) else {
            continue;
        };

        let before = placed.color_override;
        placed.color_override = None;

        undo_stack.push(CourseEditorAction::GateColorChange {
            entity,
            before,
            after: None,
        });

        apply_gate_color_to_children(
            entity,
            None,
            &placed.obstacle_id,
            &child_of_query,
            &mut material_query,
            &mut cel_materials,
            light_dir.0,
        );
    }
}

pub fn update_gate_color_label(
    selection: Res<EditorSelection>,
    placed_query: Query<&PlacedObstacle>,
    mut label_query: Query<(&mut Text, &mut TextColor), With<GateColorLabel>>,
    mut cell_query: Query<(&GateColorCell, &mut BorderColor)>,
) {
    if !selection.is_changed() {
        return;
    }

    let Ok((mut text, mut color)) = label_query.single_mut() else {
        return;
    };

    let placed = selection
        .entity
        .and_then(|e| placed_query.get(e).ok());

    if let Some(placed) = placed {
        if let Some(rgba) = placed.color_override {
            let name = PALETTE_COLORS
                .iter()
                .find(|(_, rgb)| {
                    (rgb[0] - rgba[0]).abs() < 0.001
                        && (rgb[1] - rgba[1]).abs() < 0.001
                        && (rgb[2] - rgba[2]).abs() < 0.001
                })
                .map(|(name, _)| *name)
                .unwrap_or("Custom");
            **text = format!("Color: {name}");
            *color = TextColor(Color::srgb(rgba[0], rgba[1], rgba[2]));
        } else {
            **text = "Color: Default".to_string();
            *color = TextColor(palette::SIDEWALK);
        }

        let selected_idx = placed.color_override.and_then(|rgba| {
            PALETTE_COLORS.iter().position(|(_, rgb)| {
                (rgb[0] - rgba[0]).abs() < 0.001
                    && (rgb[1] - rgba[1]).abs() < 0.001
                    && (rgb[2] - rgba[2]).abs() < 0.001
            })
        });
        for (cell, mut border) in &mut cell_query {
            *border = if Some(cell.0) == selected_idx {
                BorderColor::all(palette::VANILLA)
            } else {
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.3))
            };
        }
    } else {
        **text = "Color: (select a gate)".to_string();
        *color = TextColor(palette::CHAINMAIL);
        for (_, mut border) in &mut cell_query {
            *border = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.3));
        }
    }
}

fn apply_gate_color_to_children(
    entity: Entity,
    color_override: Option<[f32; 4]>,
    obstacle_id: &crate::obstacle::definition::ObstacleId,
    child_of_query: &Query<(Entity, &ChildOf)>,
    material_query: &mut Query<&mut MeshMaterial3d<CelMaterial>>,
    cel_materials: &mut Assets<CelMaterial>,
    light_dir: Vec3,
) {
    let base_color = color_override
        .map(|rgba| Color::srgb(rgba[0], rgba[1], rgba[2]))
        .or_else(|| gate_color(obstacle_id))
        .unwrap_or(palette::CHAINMAIL);

    let new_handle = cel_materials.add(cel_material_from_color(base_color, light_dir));

    for (child_entity, child_of) in child_of_query.iter() {
        if child_of.parent() == entity
            && let Ok(mut mat) = material_query.get_mut(child_entity)
        {
            mat.0 = new_handle.clone();
        }
    }
}
