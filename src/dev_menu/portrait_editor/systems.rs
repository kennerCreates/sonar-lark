use std::collections::HashSet;

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;

use crate::palette;
use crate::pilot::portrait::loader::PortraitParts;
use crate::pilot::portrait::rasterize::rasterize_portrait;
use crate::pilot::portrait::{
    ALL_ACCESSORIES, ALL_EYE_STYLES, ALL_FACE_SHAPES, ALL_HAIR_STYLES, ALL_MOUTH_STYLES,
    ALL_SHIRT_STYLES,
};
use crate::states::AppState;

use crate::dev_menu::portrait_config::{
    DRONE_COLOR_INDEX, MIN_DRONE_COLORS, PALETTE_COLORS, PortraitColorSlot,
    PortraitPaletteConfig, save_config,
};
use super::{
    AllowedGridPanel, AutoAssignAllButton, BackButton, COLOR_CELL_SIZE, COLOR_GRID_COLS,
    ColorNameLabel, COMPLEMENTARY_BORDER, DroneColorPickerCell, DroneWarningLabel, EditorTab,
    MakeUniqueButton, PairingPickerCell, PairingPickerPanel, PartTab, PortraitEditorState,
    PreviewImage, PrimaryColorCell, PrimarySection, ResetAllButton,
    ResetSlotButton, SELECTED_BORDER, SaveButton, SecondaryPairingCell, TAB_ACTIVE, TAB_NORMAL,
    UniqueStatusRow, VariantButton, VariantPanel, VetoedGridPanel,
};

// ── Interaction systems ─────────────────────────────────────────────────────

pub fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_part_tabs(
    query: Query<(&Interaction, &PartTab), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, tab) in &query {
        if *interaction == Interaction::Pressed && state.active_tab != tab.0 {
            state.active_tab = tab.0;
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
        }
    }
}

pub fn handle_variant_selection(
    query: Query<(&Interaction, &VariantButton), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, vb) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match vb.tab {
            EditorTab::Face => {
                state.face_shape = ALL_FACE_SHAPES[vb.index];
            }
            EditorTab::Eyes => {
                state.eye_style = ALL_EYE_STYLES[vb.index];
            }
            EditorTab::Mouth => {
                state.mouth_style = ALL_MOUTH_STYLES[vb.index];
            }
            EditorTab::Hair => {
                state.hair_style = ALL_HAIR_STYLES[vb.index];
            }
            EditorTab::Shirt => {
                state.shirt_style = ALL_SHIRT_STYLES[vb.index];
            }
            EditorTab::Accessory => {
                if vb.index >= ALL_ACCESSORIES.len() {
                    state.accessory = None;
                } else {
                    state.accessory = Some(ALL_ACCESSORIES[vb.index]);
                }
            }
            EditorTab::Drone => {}
        }
        state.preview_dirty = true;
    }
}

pub fn handle_primary_color_click(
    query: Query<(&Interaction, &PrimaryColorCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            state.primary_colors.insert(slot, cell.0);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_primary_color_veto(
    query: Query<(&Interaction, &PrimaryColorCell)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut config: ResMut<PortraitPaletteConfig>,
    state: Res<PortraitEditorState>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(slot) = state.active_tab.color_slot() else {
        return;
    };
    let vi = state.current_variant_index();
    for (interaction, cell) in &query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            // Block vetoing if it would drop drone colors below minimum
            if slot == PortraitColorSlot::Drone
                && !config.is_vetoed(slot, cell.0)
                && config.drone_colors_allowed() <= MIN_DRONE_COLORS
            {
                return;
            }
            config.toggle_veto_for(slot, vi, cell.0);
        }
    }
}

pub fn handle_save_button(
    query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    config: Res<PortraitPaletteConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if config.drone_colors_allowed() < MIN_DRONE_COLORS {
                warn!(
                    "Cannot save: need at least {} drone colors, have {}",
                    MIN_DRONE_COLORS,
                    config.drone_colors_allowed()
                );
                return;
            }
            save_config(&config);
        }
    }
}

pub fn handle_reset_slot_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ResetSlotButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            config.reset_slot(slot);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_reset_all_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ResetAllButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            config.reset_all();
            state.preview_dirty = true;
        }
    }
}

pub fn handle_make_unique_button(
    query: Query<&Interaction, (Changed<Interaction>, With<MakeUniqueButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
            && let Some(vi) = state.current_variant_index()
        {
            if config.is_variant_unique(slot, vi) {
                config.revert_variant_to_default(slot, vi);
            } else {
                config.make_variant_unique(slot, vi);
            }
            state.preview_dirty = true;
        }
    }
}

// ── Pairing interaction handlers ─────────────────────────────────────────────

/// Clicking a secondary pairing cell opens the picker for that primary color.
pub fn handle_secondary_pairing_click(
    query: Query<(&Interaction, &SecondaryPairingCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed {
            if state.selected_pairing_primary == Some(cell.0) {
                // Toggle off if clicking the same cell
                state.selected_pairing_primary = None;
            } else {
                state.selected_pairing_primary = Some(cell.0);
            }
        }
    }
}

/// Clicking a color in the picker sets the complementary and closes the picker.
pub fn handle_pairing_picker_click(
    query: Query<(&Interaction, &PairingPickerCell), Changed<Interaction>>,
    drone_query: Query<&Interaction, (Changed<Interaction>, With<DroneColorPickerCell>)>,
    mut state: ResMut<PortraitEditorState>,
    mut config: ResMut<PortraitPaletteConfig>,
) {
    let Some(primary_idx) = state.selected_pairing_primary else {
        return;
    };

    // Handle drone-color rainbow cell
    for interaction in &drone_query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.set_complementary_for(slot, vi, primary_idx, DRONE_COLOR_INDEX);
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
            return;
        }
    }

    // Handle normal palette cell
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.set_complementary_for(slot, vi, primary_idx, cell.0);
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
        }
    }
}

/// Dismiss the pairing picker when clicking outside it.
pub fn dismiss_pairing_picker(
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<PortraitEditorState>,
    picker_cells: Query<&Interaction, With<PairingPickerCell>>,
    drone_cell: Query<&Interaction, With<DroneColorPickerCell>>,
    secondary_cells: Query<&Interaction, With<SecondaryPairingCell>>,
) {
    if state.selected_pairing_primary.is_none() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Don't dismiss if hovering over picker, drone-color, or secondary cells
    for interaction in &picker_cells {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    for interaction in &drone_cell {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    for interaction in &secondary_cells {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    state.selected_pairing_primary = None;
}

pub fn handle_auto_assign_all(
    query: Query<&Interaction, (Changed<Interaction>, With<AutoAssignAllButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && state.show_pairing()
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.auto_assign_all_for(slot, vi);
            state.preview_dirty = true;
        }
    }
}

// ── Preview update ──────────────────────────────────────────────────────────

pub fn update_preview(
    mut state: ResMut<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    portrait_parts: Option<Res<PortraitParts>>,
    mut images: ResMut<Assets<Image>>,
    mut preview_query: Query<(Entity, Option<&mut ImageNode>), With<PreviewImage>>,
    mut commands: Commands,
) {
    if !state.preview_dirty {
        return;
    }
    state.preview_dirty = false;

    let Some(parts) = portrait_parts else {
        return;
    };

    let descriptor = state.build_descriptor(&config);
    let bg = PALETTE_COLORS[state.primary_colors[&PortraitColorSlot::Drone]].1;
    let image = rasterize_portrait(&descriptor, bg, 512, &parts);
    let handle = images.add(image);
    state.preview_handle = Some(handle.clone());

    for (entity, existing_image) in &mut preview_query {
        if let Some(mut img) = existing_image {
            img.image = handle.clone();
        } else {
            commands.entity(entity).insert(ImageNode::new(handle.clone()));
        }
    }
}

// ── Visual updates (tab highlights, variant highlights, grid updates) ───────

pub fn update_tab_visuals(
    state: Res<PortraitEditorState>,
    mut query: Query<(&PartTab, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !state.is_changed() {
        return;
    }
    for (tab, mut bg, mut border) in &mut query {
        let is_active = tab.0 == state.active_tab;
        *bg = BackgroundColor(if is_active { TAB_ACTIVE } else { TAB_NORMAL });
        *border = BorderColor::all(if is_active {
            palette::SKY
        } else {
            palette::STEEL
        });
    }
}

pub fn rebuild_variant_panel(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    variant_panel: Query<Entity, With<VariantPanel>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    for entity in &variant_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            super::build::spawn_variant_buttons(parent, &state, &config);
        });
    }
}

pub fn rebuild_unique_status_row(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    row_query: Query<Entity, With<UniqueStatusRow>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    let slot = state.active_tab.color_slot();
    let vi = state.current_variant_index();
    // Only show for tabs with both a color slot and a variant
    let show = slot.is_some() && vi.is_some();

    for entity in &row_query {
        commands.entity(entity).insert(if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        commands.entity(entity).despawn_children();
        if let (Some(slot), Some(vi)) = (slot, vi) {
            let is_unique = config.is_variant_unique(slot, vi);
            commands.entity(entity).with_children(|row| {
                let (label, bg_color, border_color) = if is_unique {
                    ("REVERT TO DEFAULT", palette::SAPPHIRE, palette::SUNSHINE)
                } else {
                    ("MAKE UNIQUE", palette::INDIGO, palette::STEEL)
                };
                row.spawn((
                    Button,
                    MakeUniqueButton,
                    Node {
                        height: Val::Px(24.0),
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(bg_color),
                    BorderColor::all(border_color),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
                if is_unique {
                    row.spawn((
                        Text::new("(variant has its own veto set)"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::SUNSHINE),
                    ));
                }
            });
        }
    }
}

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

pub fn update_drone_warning(
    config: Res<PortraitPaletteConfig>,
    state: Res<PortraitEditorState>,
    mut query: Query<(&mut Text, &mut Visibility), With<DroneWarningLabel>>,
) {
    if !config.is_changed() && !state.is_changed() {
        return;
    }
    let allowed = config.drone_colors_allowed();
    let show = state.active_tab == EditorTab::Drone && allowed <= MIN_DRONE_COLORS;
    for (mut text, mut vis) in &mut query {
        if show {
            *vis = Visibility::Visible;
            let msg = format!(
                "Need at least {} drone colors ({} allowed) \u{2014} cannot save",
                MIN_DRONE_COLORS, allowed,
            );
            if text.0 != msg {
                text.0 = msg;
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

pub fn update_color_name_on_hover(
    primary_query: Query<(&Interaction, &PrimaryColorCell)>,
    pairing_cell_query: Query<(&Interaction, &SecondaryPairingCell)>,
    pairing_picker_query: Query<(&Interaction, &PairingPickerCell)>,
    drone_picker_query: Query<&Interaction, With<DroneColorPickerCell>>,
    mut label_query: Query<&mut Text, With<ColorNameLabel>>,
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
) {
    let mut name = "";
    for interaction in &drone_picker_query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            name = "Drone Color";
            break;
        }
    }
    for (interaction, cell) in &primary_query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            name = PALETTE_COLORS[cell.0].0;
            break;
        }
    }
    if name.is_empty()
        && let Some(slot) = state.active_tab.color_slot()
    {
        let vi = state.current_variant_index();
        for (interaction, cell) in &pairing_cell_query {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                let comp = config.get_complementary_for(slot, vi, cell.0);
                match comp {
                    Some(idx) if idx != DRONE_COLOR_INDEX => {
                        name = PALETTE_COLORS[idx].0;
                    }
                    _ => {
                        name = "Drone Color";
                    }
                }
                break;
            }
        }
    }
    if name.is_empty() {
        for (interaction, cell) in &pairing_picker_query {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                name = PALETTE_COLORS[cell.0].0;
                break;
            }
        }
    }
    for mut text in &mut label_query {
        if text.0 != name {
            text.0 = name.to_string();
        }
    }
}

pub fn handle_button_hover_visuals(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (
            Changed<Interaction>,
            Or<(
                With<BackButton>,
                With<SaveButton>,
                With<ResetSlotButton>,
                With<ResetAllButton>,
                With<AutoAssignAllButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg, mut border) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(palette::GREEN);
                *border = BorderColor::all(palette::VANILLA);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(palette::SAPPHIRE);
                *border = BorderColor::all(palette::SIDEWALK);
            }
            Interaction::None => {
                *bg = BackgroundColor(palette::INDIGO);
                *border = BorderColor::all(palette::STEEL);
            }
        }
    }
}
