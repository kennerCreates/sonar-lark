use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::editor::undo::UndoStack;
use crate::palette;
use crate::states::HypeMode;
use crate::ui_theme;

use super::canvas::{self, CANVAS_DISPLAY_HEIGHT, CANVAS_DISPLAY_WIDTH};
use super::{
    BrushSizeButton, CanvasContainer, EraseToolButton, PaintToolButton, PosterAction,
    PosterCanvas, PosterColorCell, PosterEditorState, PosterStartRaceButton, PosterTool,
    TextToolButton, ToolButtonMarker,
};

/// 9-color subset for the poster painter.
pub const POSTER_COLORS: [(&str, Color); 9] = [
    ("Black", palette::BLACK),
    ("White", palette::VANILLA),
    ("Blue", palette::CAROLINA),
    ("Purple", palette::ORCHID),
    ("Orange", palette::DANDELION),
    ("Green", palette::GREEN),
    ("Salmon", palette::PEACH),
    ("Pink", palette::PINK),
    ("Lime", palette::LIME),
];

const COLOR_CELL_SIZE: f32 = 60.0;
const COLOR_GRID_COLS: usize = 3;

const BRUSH_SMALL: f32 = 4.0;
const BRUSH_MEDIUM: f32 = 10.0;
const BRUSH_LARGE: f32 = 20.0;

pub fn setup_poster_editor(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let canvas_image = canvas::create_blank_canvas();
    let canvas_handle = images.add(canvas_image);

    commands.insert_resource(PosterEditorState {
        active_tool: PosterTool::Paint,
        brush_color: [0, 0, 0, 255],
        brush_radius: BRUSH_MEDIUM,
        canvas_handle: canvas_handle.clone(),
        strokes: Vec::new(),
        current_stroke: None,
        editing_text: None,
    });
    commands.insert_resource(UndoStack::<PosterAction>::default());

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
            spawn_left_panel(root);

            // Center — canvas
            spawn_canvas_area(root, canvas_handle);

            // Right panel — color palette
            spawn_right_panel(root);

            // Start Race button (absolute positioned)
            spawn_start_race_button(root);
        });
}

fn spawn_left_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            width: Val::Px(180.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|panel| {
            // Paint button (selected by default)
            spawn_tool_button(panel, "Paint", PosterTool::Paint, true);
            spawn_tool_button(panel, "Text", PosterTool::Text, false);
            spawn_tool_button(panel, "Erase", PosterTool::Erase, false);

            // Spacer
            panel.spawn(Node {
                height: Val::Px(20.0),
                ..default()
            });

            // Brush size label
            panel.spawn((
                Text::new("Brush Size"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Brush size buttons
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    spawn_brush_size_button(row, "S", BRUSH_SMALL, false);
                    spawn_brush_size_button(row, "M", BRUSH_MEDIUM, true);
                    spawn_brush_size_button(row, "L", BRUSH_LARGE, false);
                });
        });
}

fn spawn_tool_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    tool: PosterTool,
    selected: bool,
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
) {
    parent
        .spawn((
            Button,
            BrushSizeButton(size),
            Node {
                width: Val::Px(40.0),
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
                });
        });
}

fn spawn_right_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            width: Val::Px(240.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|panel| {
            // 3x3 color grid
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    max_width: Val::Px(
                        (COLOR_CELL_SIZE + 4.0) * COLOR_GRID_COLS as f32,
                    ),
                    ..default()
                })
                .with_children(|grid| {
                    for (i, &(_, color)) in POSTER_COLORS.iter().enumerate() {
                        let selected = i == 0; // Black selected by default
                        grid.spawn((
                            Button,
                            PosterColorCell(i),
                            Node {
                                width: Val::Px(COLOR_CELL_SIZE),
                                height: Val::Px(COLOR_CELL_SIZE),
                                border: UiRect::all(Val::Px(if selected {
                                    3.0
                                } else {
                                    1.0
                                })),
                                ..default()
                            },
                            BackgroundColor(color),
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

fn spawn_start_race_button(parent: &mut ChildSpawnerCommands) {
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
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
        });
}
