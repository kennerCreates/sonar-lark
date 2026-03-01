use std::collections::HashSet;

use bevy::prelude::*;

use crate::palette;

use crate::dev_menu::portrait_config::{
    DRONE_COLOR_INDEX, PALETTE_COLORS, PortraitPaletteConfig,
};
use super::{
    AllowedGridPanel, AutoAssignAllButton, COLOR_CELL_SIZE, COLOR_GRID_COLS,
    COMPLEMENTARY_BORDER, DroneColorPickerCell, PairingPickerCell, PairingPickerPanel,
    PortraitEditorState, PrimaryColorCell, PrimarySection, SELECTED_BORDER,
    SecondaryPairingCell, VetoedGridPanel,
};

pub fn rebuild_primary_grid(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    allowed_panel: Query<Entity, With<AllowedGridPanel>>,
    vetoed_panel: Query<Entity, With<VetoedGridPanel>>,
    section_panel: Query<Entity, With<PrimarySection>>,
    auto_btn: Query<Entity, With<AutoAssignAllButton>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    let slot = state.active_tab.color_slot();
    let vi = state.current_variant_index();
    let selected = slot.and_then(|s| state.primary_colors.get(&s).copied());
    let show_pairing = state.show_pairing();

    // Hide entire primary color section when tab has no color slot (e.g. Mouth)
    for entity in &section_panel {
        commands.entity(entity).insert(if slot.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }

    // Show/hide auto-pair button
    for entity in &auto_btn {
        commands.entity(entity).insert(if show_pairing {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }

    // Use variant-aware allowed check
    let allowed_set: HashSet<usize> = slot
        .map(|s| config.allowed_indices_for(s, vi).into_iter().collect())
        .unwrap_or_default();

    // Collect allowed indices in palette order
    let allowed_ordered: Vec<usize> = PALETTE_COLORS
        .iter()
        .enumerate()
        .filter(|(i, _)| allowed_set.contains(i))
        .map(|(i, _)| i)
        .collect();

    for entity in &allowed_panel {
        commands.entity(entity).despawn_children();

        if show_pairing {
            // Paired layout: chunk into rows with secondary row below each primary row
            commands.entity(entity).insert(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            });
            let slot = slot.unwrap();
            commands.entity(entity).with_children(|parent| {
                for chunk in allowed_ordered.chunks(COLOR_GRID_COLS) {
                    // Row pair container
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        })
                        .with_children(|pair| {
                            // Primary row
                            pair.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|row| {
                                for &i in chunk {
                                    let rgb = &PALETTE_COLORS[i].1;
                                    spawn_color_cell(row, i, rgb, selected == Some(i));
                                }
                            });
                            // Secondary row
                            pair.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|row| {
                                for &i in chunk {
                                    let explicit = config.get_complementary_for(slot, vi, i);
                                    let is_active =
                                        state.selected_pairing_primary == Some(i);
                                    match explicit {
                                        Some(idx) if idx != DRONE_COLOR_INDEX => {
                                            let rgb = &PALETTE_COLORS[idx].1;
                                            spawn_secondary_pairing_cell(
                                                row,
                                                i,
                                                rgb,
                                                true,
                                                is_active,
                                            );
                                        }
                                        _ => {
                                            spawn_secondary_pairing_rainbow(
                                                row, i, is_active,
                                            );
                                        }
                                    }
                                }
                            });
                        });
                }
            });
        } else {
            // Flat wrapping layout for non-pairing tabs
            commands.entity(entity).insert(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(2.0),
                row_gap: Val::Px(2.0),
                max_width: Val::Px((COLOR_CELL_SIZE + 2.0) * COLOR_GRID_COLS as f32),
                ..default()
            });
            commands.entity(entity).with_children(|parent| {
                for &i in &allowed_ordered {
                    let rgb = &PALETTE_COLORS[i].1;
                    spawn_color_cell(parent, i, rgb, selected == Some(i));
                }
            });
        }
    }

    for entity in &vetoed_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
                if slot.is_none() || allowed_set.contains(&i) {
                    continue;
                }
                spawn_color_cell(parent, i, rgb, selected == Some(i));
            }
        });
    }
}

fn spawn_color_cell(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    rgb: &[f32; 3],
    is_selected: bool,
) {
    parent.spawn((
        Button,
        PrimaryColorCell(index),
        Node {
            width: Val::Px(COLOR_CELL_SIZE),
            height: Val::Px(COLOR_CELL_SIZE),
            border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
            ..default()
        },
        BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
        BorderColor::all(if is_selected {
            SELECTED_BORDER
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.3)
        }),
    ));
}

fn spawn_secondary_pairing_cell(
    parent: &mut ChildSpawnerCommands,
    primary_index: usize,
    rgb: &[f32; 3],
    is_explicit: bool,
    is_active: bool,
) {
    let border = if is_active {
        SELECTED_BORDER
    } else if is_explicit {
        COMPLEMENTARY_BORDER
    } else {
        Color::srgba(0.3, 0.3, 0.3, 0.5)
    };
    parent.spawn((
        Button,
        SecondaryPairingCell(primary_index),
        Node {
            width: Val::Px(COLOR_CELL_SIZE),
            height: Val::Px(COLOR_CELL_SIZE),
            border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
            ..default()
        },
        BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
        BorderColor::all(border),
    ));
}

fn spawn_secondary_pairing_rainbow(
    parent: &mut ChildSpawnerCommands,
    primary_index: usize,
    is_active: bool,
) {
    let border = if is_active {
        SELECTED_BORDER
    } else {
        COMPLEMENTARY_BORDER
    };
    parent
        .spawn((
            Button,
            SecondaryPairingCell(primary_index),
            Node {
                width: Val::Px(COLOR_CELL_SIZE),
                height: Val::Px(COLOR_CELL_SIZE),
                border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(border),
        ))
        .with_children(|cell| {
            let stripe_h = COLOR_CELL_SIZE / RAINBOW_STRIPES.len() as f32;
            for &(r, g, b) in &RAINBOW_STRIPES {
                cell.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(stripe_h),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(r, g, b)),
                ));
            }
        });
}

/// Rainbow stripe colors for the drone-color cell.
const RAINBOW_STRIPES: [(f32, f32, f32); 6] = [
    (1.0, 0.0, 0.0),   // red
    (1.0, 0.5, 0.0),   // orange
    (1.0, 1.0, 0.0),   // yellow
    (0.0, 1.0, 0.0),   // green
    (0.0, 0.4, 1.0),   // blue
    (0.6, 0.0, 1.0),   // purple
];

pub fn rebuild_pairing_picker(
    state: Res<PortraitEditorState>,
    mut commands: Commands,
    picker_panel: Query<Entity, With<PairingPickerPanel>>,
) {
    if !state.is_changed() {
        return;
    }
    let show = state.selected_pairing_primary.is_some() && state.show_pairing();
    for entity in &picker_panel {
        commands.entity(entity).insert(if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        commands.entity(entity).despawn_children();
        if show {
            commands.entity(entity).with_children(|parent| {
                // Drone-color rainbow cell (first option)
                spawn_rainbow_cell(parent);

                for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
                    parent.spawn((
                        Button,
                        PairingPickerCell(i),
                        Node {
                            width: Val::Px(COLOR_CELL_SIZE),
                            height: Val::Px(COLOR_CELL_SIZE),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
                        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                    ));
                }
            });
        }
    }
}

fn spawn_rainbow_cell(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Button,
            DroneColorPickerCell,
            Node {
                width: Val::Px(COLOR_CELL_SIZE),
                height: Val::Px(COLOR_CELL_SIZE),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(palette::VANILLA),
        ))
        .with_children(|cell| {
            let stripe_h = COLOR_CELL_SIZE / RAINBOW_STRIPES.len() as f32;
            for &(r, g, b) in &RAINBOW_STRIPES {
                cell.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(stripe_h),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(r, g, b)),
                ));
            }
        });
}
