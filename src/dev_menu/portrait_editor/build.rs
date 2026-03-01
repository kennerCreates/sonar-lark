use bevy::prelude::*;

use crate::palette;
use crate::pilot::portrait::{
    ALL_ACCESSORIES, ALL_EYE_STYLES, ALL_FACE_SHAPES, ALL_HAIR_STYLES, ALL_MOUTH_STYLES,
    ALL_SHIRT_STYLES,
};
use crate::states::AppState;

use crate::dev_menu::portrait_config::PortraitPaletteConfig;
use super::{
    AllowedGridPanel, AutoAssignAllButton, BackButton, COLOR_CELL_SIZE, COLOR_GRID_COLS,
    ColorNameLabel, DroneWarningLabel, EditorTab, PANEL_BG, PairingPickerPanel, PartTab,
    PortraitEditorState, PreviewImage, PrimaryGridPanel, PrimarySection, RADIO_ACTIVE,
    RADIO_NORMAL, ResetAllButton, ResetSlotButton, SaveButton, TAB_ACTIVE, TAB_NORMAL,
    UniqueStatusRow, VariantButton, VariantPanel, VetoedGridPanel,
};

pub fn build_ui(
    commands: &mut Commands,
    state: &PortraitEditorState,
    config: &PortraitPaletteConfig,
    preview_handle: Option<Handle<Image>>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DespawnOnExit(AppState::DevMenu),
        ))
        .with_children(|root| {
            // Header row
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("PORTRAIT PALETTE EDITOR"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));
                header
                    .spawn((
                        Button,
                        BackButton,
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(palette::INDIGO),
                        BorderColor::all(palette::STEEL),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("BACK"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(palette::VANILLA),
                        ));
                    });
            });

            // Main content: preview (left) + controls (right)
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|main| {
                // Left column: preview
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    width: Val::Px(520.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|left| {
                    left.spawn((
                        Text::new("Preview"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(palette::SIDEWALK),
                    ));

                    // Preview image container
                    let mut preview_cmd = left.spawn((
                        PreviewImage,
                        Node {
                            width: Val::Px(512.0),
                            height: Val::Px(512.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor::all(palette::STEEL),
                        BackgroundColor(palette::SMOKY_BLACK),
                    ));
                    if let Some(handle) = preview_handle {
                        preview_cmd.insert(ImageNode::new(handle));
                    }

                    // Color name label
                    left.spawn((
                        ColorNameLabel,
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(palette::SIDEWALK),
                    ));
                });

                // Right column: controls
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|right| {
                    // Tab row
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|tab_row| {
                            for tab in EditorTab::ALL {
                                let is_active = tab == state.active_tab;
                                tab_row
                                    .spawn((
                                        Button,
                                        PartTab(tab),
                                        Node {
                                            width: Val::Px(56.0),
                                            height: Val::Px(28.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(if is_active {
                                            TAB_ACTIVE
                                        } else {
                                            TAB_NORMAL
                                        }),
                                        BorderColor::all(if is_active {
                                            palette::SKY
                                        } else {
                                            palette::STEEL
                                        }),
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(tab.label()),
                                            TextFont {
                                                font_size: 13.0,
                                                ..default()
                                            },
                                            TextColor(palette::VANILLA),
                                        ));
                                    });
                            }
                        });

                    // Variant radio buttons
                    right
                        .spawn((
                            VariantPanel,
                            Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(4.0),
                                row_gap: Val::Px(4.0),
                                ..default()
                            },
                        ))
                        .with_children(|variant_area| {
                            spawn_variant_buttons(variant_area, state, config);
                        });

                    // Unique variant toggle row
                    right.spawn((
                        UniqueStatusRow,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Visibility::Hidden,
                    ));

                    // Color grids row: primary (left) + pairing (right)
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(16.0),
                            align_items: AlignItems::Start,
                            ..default()
                        })
                        .with_children(|color_row| {
                            // Primary color grid (two-column: allowed / vetoed)
                            color_row
                                .spawn((
                                    PrimarySection,
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(4.0),
                                        ..default()
                                    },
                                    if state.active_tab.color_slot().is_some() {
                                        Visibility::Visible
                                    } else {
                                        Visibility::Hidden
                                    },
                                ))
                                .with_children(|section| {
                                    // Header row: label + auto-assign (for pairing tabs)
                                    section
                                        .spawn(Node {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(8.0),
                                            align_items: AlignItems::Center,
                                            ..default()
                                        })
                                        .with_children(|hdr| {
                                            hdr.spawn((
                                                Text::new(
                                                    "Primary Color (left-click = select, right-click = move)",
                                                ),
                                                TextFont {
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(palette::SIDEWALK),
                                            ));
                                            hdr.spawn((
                                                Button,
                                                AutoAssignAllButton,
                                                Node {
                                                    height: Val::Px(20.0),
                                                    padding: UiRect::horizontal(Val::Px(6.0)),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(palette::INDIGO),
                                                BorderColor::all(palette::STEEL),
                                                if state.show_pairing() {
                                                    Visibility::Visible
                                                } else {
                                                    Visibility::Hidden
                                                },
                                            ))
                                            .with_children(|btn| {
                                                btn.spawn((
                                                    Text::new("Auto-pair"),
                                                    TextFont {
                                                        font_size: 11.0,
                                                        ..default()
                                                    },
                                                    TextColor(palette::VANILLA),
                                                ));
                                            });
                                        });
                                    section
                                        .spawn((
                                            PrimaryGridPanel,
                                            Node {
                                                flex_direction: FlexDirection::Row,
                                                column_gap: Val::Px(16.0),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|grid_parent| {
                                            // Left: Allowed colors
                                            grid_parent
                                                .spawn(Node {
                                                    flex_direction: FlexDirection::Column,
                                                    row_gap: Val::Px(4.0),
                                                    ..default()
                                                })
                                                .with_children(|col| {
                                                    col.spawn((
                                                        Text::new("Allowed"),
                                                        TextFont {
                                                            font_size: 11.0,
                                                            ..default()
                                                        },
                                                        TextColor(palette::SIDEWALK),
                                                    ));
                                                    col.spawn((
                                                        AllowedGridPanel,
                                                        Node {
                                                            flex_direction: FlexDirection::Row,
                                                            flex_wrap: FlexWrap::Wrap,
                                                            column_gap: Val::Px(2.0),
                                                            row_gap: Val::Px(2.0),
                                                            max_width: Val::Px(
                                                                (COLOR_CELL_SIZE + 2.0)
                                                                    * COLOR_GRID_COLS as f32,
                                                            ),
                                                            ..default()
                                                        },
                                                    ));
                                                });
                                            // Right: Vetoed colors
                                            grid_parent
                                                .spawn(Node {
                                                    flex_direction: FlexDirection::Column,
                                                    row_gap: Val::Px(4.0),
                                                    ..default()
                                                })
                                                .with_children(|col| {
                                                    col.spawn((
                                                        Text::new("Vetoed"),
                                                        TextFont {
                                                            font_size: 11.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgba(
                                                            0.8, 0.3, 0.3, 1.0,
                                                        )),
                                                    ));
                                                    col.spawn((
                                                        VetoedGridPanel,
                                                        Node {
                                                            flex_direction: FlexDirection::Row,
                                                            flex_wrap: FlexWrap::Wrap,
                                                            column_gap: Val::Px(2.0),
                                                            row_gap: Val::Px(2.0),
                                                            max_width: Val::Px(
                                                                (COLOR_CELL_SIZE + 2.0)
                                                                    * COLOR_GRID_COLS as f32,
                                                            ),
                                                            ..default()
                                                        },
                                                    ));
                                                });
                                        });

                                    // Inline picker (hidden until a secondary cell is clicked)
                                    section.spawn((
                                        PairingPickerPanel,
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            flex_wrap: FlexWrap::Wrap,
                                            column_gap: Val::Px(2.0),
                                            row_gap: Val::Px(2.0),
                                            max_width: Val::Px(
                                                (COLOR_CELL_SIZE + 2.0)
                                                    * COLOR_GRID_COLS as f32,
                                            ),
                                            padding: UiRect::all(Val::Px(4.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
                                        BorderColor::all(palette::SUNSHINE),
                                        Visibility::Hidden,
                                    ));
                                });
                        });

                    // Drone color minimum warning (hidden by default)
                    right.spawn((
                        DroneWarningLabel,
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
                        Visibility::Hidden,
                    ));

                    // Action buttons
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|actions| {
                            spawn_action_button(actions, "SAVE", SaveButton);
                            spawn_action_button(actions, "RESET SLOT", ResetSlotButton);
                            spawn_action_button(actions, "RESET ALL", ResetAllButton);
                        });
                });
            });
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(palette::INDIGO),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

pub fn spawn_variant_buttons(
    parent: &mut ChildSpawnerCommands,
    state: &PortraitEditorState,
    config: &PortraitPaletteConfig,
) {
    let slot = state.active_tab.color_slot();
    match state.active_tab {
        EditorTab::Face => {
            for (i, shape) in ALL_FACE_SHAPES.iter().enumerate() {
                let is_active = *shape == state.face_shape;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(parent, EditorTab::Face, i, &format!("{shape:?}"), is_active, is_unique);
            }
        }
        EditorTab::Eyes => {
            for (i, style) in ALL_EYE_STYLES.iter().enumerate() {
                let is_active = *style == state.eye_style;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(parent, EditorTab::Eyes, i, &format!("{style:?}"), is_active, is_unique);
            }
        }
        EditorTab::Mouth => {
            for (i, style) in ALL_MOUTH_STYLES.iter().enumerate() {
                let is_active = *style == state.mouth_style;
                spawn_radio_button(
                    parent,
                    EditorTab::Mouth,
                    i,
                    &format!("{style:?}"),
                    is_active,
                    false,
                );
            }
        }
        EditorTab::Hair => {
            for (i, style) in ALL_HAIR_STYLES.iter().enumerate() {
                let is_active = *style == state.hair_style;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(parent, EditorTab::Hair, i, &format!("{style:?}"), is_active, is_unique);
            }
        }
        EditorTab::Shirt => {
            for (i, style) in ALL_SHIRT_STYLES.iter().enumerate() {
                let is_active = *style == state.shirt_style;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(
                    parent,
                    EditorTab::Shirt,
                    i,
                    &format!("{style:?}"),
                    is_active,
                    is_unique,
                );
            }
        }
        EditorTab::Accessory => {
            // "None" option (never unique)
            spawn_radio_button(
                parent,
                EditorTab::Accessory,
                ALL_ACCESSORIES.len(),
                "None",
                state.accessory.is_none(),
                false,
            );
            for (i, acc) in ALL_ACCESSORIES.iter().enumerate() {
                let is_active = state.accessory == Some(*acc);
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(
                    parent,
                    EditorTab::Accessory,
                    i,
                    &format!("{acc:?}"),
                    is_active,
                    is_unique,
                );
            }
        }
        EditorTab::Drone => {
            parent.spawn((
                Text::new("Portrait background color"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));
        }
    }
}

fn spawn_radio_button(
    parent: &mut ChildSpawnerCommands,
    tab: EditorTab,
    index: usize,
    label: &str,
    active: bool,
    is_unique: bool,
) {
    let border_color = if active {
        palette::SKY
    } else if is_unique {
        palette::SUNSHINE
    } else {
        palette::STEEL
    };
    parent
        .spawn((
            Button,
            VariantButton { tab, index },
            Node {
                height: Val::Px(24.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(if is_unique { 2.0 } else { 1.0 })),
                ..default()
            },
            BackgroundColor(if active { RADIO_ACTIVE } else { RADIO_NORMAL }),
            BorderColor::all(border_color),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(if active {
                    palette::VANILLA
                } else if is_unique {
                    palette::SUNSHINE
                } else {
                    palette::SIDEWALK
                }),
            ));
        });
}
