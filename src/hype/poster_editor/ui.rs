use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};
use bevy::ui::RelativeCursorPosition;

use crate::dev_menu::color_picker_data::PALETTE_COLORS;
use crate::editor::undo::UndoStack;
use crate::palette;
use crate::states::HypeMode;
use crate::ui_theme;
use crate::ui_theme::UiFont;

use super::canvas::{self, CANVAS_DISPLAY_HEIGHT, CANVAS_DISPLAY_WIDTH};
use super::{
    BrushCursorPreview, BrushSizeButton, BrushSizePanel, CanvasContainer, CursorBlinkTimer,
    EraseToolButton, PaintToolButton, PosterAction, PosterCanvas, PosterColorCell,
    PosterEditorState, PosterStartRaceButton, PosterTool, TextFontButton, TextFontPanel,
    TextSizeButton, TextSizePanel, TextToolButton, ToolButtonMarker, POSTER_FONTS,
};

const COLOR_CELL_SIZE: f32 = 28.0;
const COLOR_GRID_COLS: usize = 8;
const COLOR_GAP: f32 = 2.0;

const BRUSH_SMALL: f32 = 4.0;
const BRUSH_MEDIUM: f32 = 10.0;
const BRUSH_LARGE: f32 = 20.0;

const TEXT_SMALL: f32 = 18.0;
const TEXT_MEDIUM: f32 = 28.0;
const TEXT_LARGE: f32 = 40.0;
const TEXT_XLARGE: f32 = 56.0;

pub fn setup_poster_editor(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    font: Res<UiFont>,
) {
    let ui_font = font.0.clone();
    let canvas_image = canvas::create_blank_canvas();
    let canvas_handle = images.add(canvas_image);

    commands.insert_resource(PosterEditorState {
        active_tool: PosterTool::Paint,
        brush_color: [0, 0, 0, 255],
        brush_radius: BRUSH_MEDIUM,
        canvas_handle: canvas_handle.clone(),
        actions: Vec::new(),
        current_stroke: None,
        editing_text: None,
        text_size: TEXT_MEDIUM,
        text_font_index: 0,
    });
    commands.insert_resource(UndoStack::<PosterAction>::default());
    commands.insert_resource(CursorBlinkTimer {
        timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        visible: true,
    });

    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(palette::SMOKY_BLACK),
            DespawnOnExit(HypeMode::PosterEditor),
        ))
        .with_children(|root| {
            // Left panel — tools
            spawn_left_panel(root, &asset_server, ui_font.clone());

            // Center — canvas
            spawn_canvas_area(root, canvas_handle);

            // Right panel — color palette
            spawn_right_panel(root);

            // Start Race button (absolute positioned)
            spawn_start_race_button(root, ui_font.clone());
        });
}

fn spawn_left_panel(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer, ui_font: Handle<Font>) {
    parent
        .spawn(Node {
            width: Val::Px(220.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|panel| {
            // Paint button (selected by default)
            spawn_tool_button(panel, "Paint", PosterTool::Paint, true, ui_font.clone());
            spawn_tool_button(panel, "Text", PosterTool::Text, false, ui_font.clone());
            spawn_tool_button(panel, "Erase", PosterTool::Erase, false, ui_font.clone());

            // Brush size section (hidden when not in paint/erase mode)
            panel
                .spawn((
                    BrushSizePanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                ))
                .with_children(|section| {
                    section.spawn((
                        Text::new("Brush Size"),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));

                    let ui_font_brush = ui_font.clone();
                    section
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_brush_size_button(row, "S", BRUSH_SMALL, false, ui_font_brush.clone());
                            spawn_brush_size_button(row, "M", BRUSH_MEDIUM, true, ui_font_brush.clone());
                            spawn_brush_size_button(row, "L", BRUSH_LARGE, false, ui_font_brush.clone());
                            spawn_brush_size_button(row, "Fill", super::BRUSH_FILL, false, ui_font_brush.clone());
                        });
                });

            // Text size section (hidden when not in text mode)
            panel
                .spawn((
                    TextSizePanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .with_children(|section| {
                    section.spawn((
                        Text::new("Text Size"),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));

                    let ui_font_text = ui_font.clone();
                    section
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_text_size_button(row, "S", TEXT_SMALL, false, ui_font_text.clone());
                            spawn_text_size_button(row, "M", TEXT_MEDIUM, true, ui_font_text.clone());
                            spawn_text_size_button(row, "L", TEXT_LARGE, false, ui_font_text.clone());
                            spawn_text_size_button(row, "XL", TEXT_XLARGE, false, ui_font_text.clone());
                        });
                });

            // Font section (hidden when not in text mode)
            panel
                .spawn((
                    TextFontPanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .with_children(|section| {
                    section.spawn((
                        Text::new("Font"),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));

                    section
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|col| {
                            for (i, (name, path)) in POSTER_FONTS.iter().enumerate() {
                                let font_handle: Handle<Font> = asset_server.load(*path);
                                spawn_font_button(col, name, font_handle, i, i == 0);
                            }
                        });
                });
        });
}

fn spawn_tool_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    tool: PosterTool,
    selected: bool,
    ui_font: Handle<Font>,
) {
    let bg = if selected {
        ui_theme::BUTTON_SELECTED
    } else {
        ui_theme::BUTTON_NORMAL
    };
    let marker: Box<dyn Fn(&mut EntityCommands)> = match tool {
        PosterTool::Paint => Box::new(|cmd: &mut EntityCommands| {
            cmd.insert(PaintToolButton);
        }),
        PosterTool::Text => Box::new(|cmd: &mut EntityCommands| {
            cmd.insert(TextToolButton);
        }),
        PosterTool::Erase => Box::new(|cmd: &mut EntityCommands| {
            cmd.insert(EraseToolButton);
        }),
    };
    let mut cmd = parent.spawn((
        Button,
        ToolButtonMarker(tool),
        Node {
            width: Val::Px(140.0),
            height: Val::Px(50.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(if selected {
            palette::VANILLA
        } else {
            ui_theme::BORDER_NORMAL
        }),
    ));
    marker(&mut cmd);
    cmd.with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont {
                font: ui_font.clone(),
                font_size: 22.0,
                ..default()
            },
            TextColor(palette::VANILLA),
        ));
    });
}

fn spawn_brush_size_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    size: f32,
    selected: bool,
    ui_font: Handle<Font>,
) {
    parent
        .spawn((
            Button,
            BrushSizeButton(size),
            Node {
                width: Val::Px(44.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(if selected {
                ui_theme::BUTTON_SELECTED
            } else {
                ui_theme::BUTTON_NORMAL
            }),
            BorderColor::all(if selected {
                palette::VANILLA
            } else {
                ui_theme::BORDER_NORMAL
            }),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

fn spawn_text_size_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    size: f32,
    selected: bool,
    ui_font: Handle<Font>,
) {
    parent
        .spawn((
            Button,
            TextSizeButton(size),
            Node {
                width: Val::Px(44.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(if selected {
                ui_theme::BUTTON_SELECTED
            } else {
                ui_theme::BUTTON_NORMAL
            }),
            BorderColor::all(if selected {
                palette::VANILLA
            } else {
                ui_theme::BORDER_NORMAL
            }),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

fn spawn_font_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    font_handle: Handle<Font>,
    index: usize,
    selected: bool,
) {
    parent
        .spawn((
            Button,
            TextFontButton(index),
            Node {
                width: Val::Px(190.0),
                height: Val::Px(36.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(if selected {
                ui_theme::BUTTON_SELECTED
            } else {
                ui_theme::BUTTON_NORMAL
            }),
            BorderColor::all(if selected {
                palette::VANILLA
            } else {
                ui_theme::BORDER_NORMAL
            }),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font_handle,
                    font_size: 18.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

fn spawn_canvas_area(parent: &mut ChildSpawnerCommands, canvas_handle: Handle<Image>) {
    parent
        .spawn(Node {
            flex_grow: 1.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|center| {
            // Canvas container with rounded corners
            center
                .spawn((
                    CanvasContainer,
                    Interaction::default(),
                    RelativeCursorPosition::default(),
                    Node {
                        width: Val::Px(CANVAS_DISPLAY_WIDTH),
                        height: Val::Px(CANVAS_DISPLAY_HEIGHT),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                ))
                .with_children(|container| {
                    // The actual image canvas
                    container.spawn((
                        PosterCanvas,
                        ImageNode::new(canvas_handle),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));

                    // Brush cursor preview (circle outline, hidden by default)
                    // Pickable::IGNORE prevents it from blocking canvas picking
                    container.spawn((
                        BrushCursorPreview,
                        Pickable::IGNORE,
                        ZIndex(2),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(1.5)),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BorderColor::all(Color::BLACK),
                        BackgroundColor(Color::NONE),
                        Visibility::Hidden,
                    ));
                });
        });
}

fn spawn_right_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            width: Val::Px(280.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|panel| {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(COLOR_GAP),
                    row_gap: Val::Px(COLOR_GAP),
                    max_width: Val::Px(
                        (COLOR_CELL_SIZE + COLOR_GAP) * COLOR_GRID_COLS as f32,
                    ),
                    ..default()
                })
                .with_children(|grid| {
                    for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
                        let selected = i == 0; // Black selected by default
                        grid.spawn((
                            Button,
                            PosterColorCell(i),
                            Node {
                                width: Val::Px(COLOR_CELL_SIZE),
                                height: Val::Px(COLOR_CELL_SIZE),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
                            BorderColor::all(if selected {
                                palette::VANILLA
                            } else {
                                Color::srgba(0.0, 0.0, 0.0, 0.3)
                            }),
                        ));
                    }
                });
        });
}

fn spawn_start_race_button(parent: &mut ChildSpawnerCommands, ui_font: Handle<Font>) {
    parent
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(40.0),
            right: Val::Px(60.0),
            ..default()
        })
        .with_children(|anchor| {
            anchor
                .spawn((
                    Button,
                    ui_theme::ThemedButton,
                    PosterStartRaceButton,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_NORMAL),
                    BorderColor::all(ui_theme::BORDER_NORMAL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("START RACE"),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
        });
}
