use bevy::prelude::*;

use crate::palette;
use crate::pilot::portrait::loader::PortraitParts;
use crate::pilot::portrait::rasterize::rasterize_portrait;

use crate::dev_menu::portrait_config::{
    DRONE_COLOR_INDEX, MIN_DRONE_COLORS, PALETTE_COLORS, PortraitColorSlot,
    PortraitPaletteConfig,
};
use super::{
    AutoAssignAllButton, BackButton, ColorNameLabel, DroneColorPickerCell, DroneWarningLabel,
    EditorTab, MakeUniqueButton, PairingPickerCell, PartTab, PortraitEditorState, PreviewImage,
    PrimaryColorCell, ResetAllButton, ResetSlotButton, SaveButton, SecondaryPairingCell,
    TAB_ACTIVE, TAB_NORMAL, UniqueStatusRow, VariantPanel,
};

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
