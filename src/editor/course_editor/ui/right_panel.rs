use bevy::prelude::*;

use crate::dev_menu::color_picker_data::PALETTE_COLORS;
use crate::editor::course_editor::TransformMode;
use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;
use crate::ui_theme;

use super::types::*;

pub fn build_right_panel(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn((
            Node {
                width: Val::Px(280.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(ui_theme::PANEL_BG),
        ))
        .with_children(|panel| {
            ui_theme::spawn_action_button(
                panel,
                "Save Course",
                SaveCourseButton,
                palette::JUNGLE,
                font,
            );

            ui_theme::spawn_action_button(
                panel,
                "Hype Race",
                StartRaceButton,
                palette::TANGERINE,
                font,
            );

            panel.spawn((
                Text::new("Budget: $---"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SUNSHINE),
                MoneyText,
            ));

            ui_theme::spawn_divider(panel);

            panel.spawn((
                Text::new("Gate Ordering"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            panel.spawn((
                Text::new("Gates: 0 (loop)"),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SKY),
                GateCountText,
            ));

            panel
                .spawn((
                    Button,
                    GateOrderModeButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::TOGGLE_OFF),
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Gate Mode: OFF"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                        GateOrderModeText,
                    ));
                });

            ui_theme::spawn_panel_button(panel, "Clear Gate Orders", ClearGateOrdersButton, font);

            ui_theme::spawn_divider(panel);

            // --- Gate Color section ---
            panel.spawn((
                Text::new("Gate Color"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            panel.spawn((
                Text::new("Color: (select a gate)"),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
                GateColorLabel,
            ));

            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(2.0),
                    row_gap: Val::Px(2.0),
                    max_width: Val::Px(
                        (GATE_COLOR_CELL_SIZE + 2.0) * GATE_COLOR_GRID_COLS as f32,
                    ),
                    ..default()
                })
                .with_children(|grid| {
                    for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
                        grid.spawn((
                            Button,
                            GateColorCell(i),
                            Node {
                                width: Val::Px(GATE_COLOR_CELL_SIZE),
                                height: Val::Px(GATE_COLOR_CELL_SIZE),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
                            BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                        ));
                    }
                });

            ui_theme::spawn_panel_button(panel, "Default Color", GateColorDefaultButton, font);

            ui_theme::spawn_divider(panel);

            panel.spawn((
                Text::new("Transform Mode"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            panel
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    width: Val::Percent(100.0),
                    ..default()
                },))
                .with_children(|row| {
                    spawn_transform_mode_button(row, "Move (1)", TransformMode::Move, font);
                    spawn_transform_mode_button(row, "Rotate (2)", TransformMode::Rotate, font);
                    spawn_transform_mode_button(row, "Scale (3)", TransformMode::Scale, font);
                });

            ui_theme::spawn_divider(panel);

            panel.spawn((
                Text::new("Del  →  delete selected"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("LMB obstacle  →  select"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("LMB palette + empty  →  place"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("Gate mode: LMB to assign order"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("F  →  flip gate direction"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("Shift  →  Y-move / axis-scale / Z-rotate"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));

            panel.spawn((
                Text::new("Ctrl  →  X-rotate"),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::CHAINMAIL),
            ));
        });
}

pub fn spawn_palette_button(parent: &mut ChildSpawnerCommands, id: &ObstacleId, font: &Handle<Font>, library: &ObstacleLibrary) {
    let cost = crate::course::data::gate_cost(&id.0, library);
    let label = if cost > 0 {
        format!("{} (${cost})", id.0)
    } else {
        id.0.clone()
    };

    parent
        .spawn((
            Button,
            PaletteButton(id.clone()),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(ui_theme::BUTTON_NORMAL),
            BorderColor::all(palette::SAPPHIRE),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::MINT),
            ));
        });
}

fn spawn_transform_mode_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    mode: TransformMode,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Button,
            TransformModeButton(mode),
            Node {
                flex_grow: 1.0,
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(ui_theme::BUTTON_NORMAL),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));
        });
}
