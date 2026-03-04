use bevy::prelude::*;

use crate::obstacle::definition::ObstacleId;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;
use crate::states::DevMenuPage;
use crate::ui_theme;

pub(super) const RADIO_ACTIVE: Color = palette::TEAL;
pub(super) const RADIO_INACTIVE: Color = palette::INDIGO;

// Marker components
#[derive(Component)]
pub struct NodeListContainer;

#[derive(Component)]
pub struct LibraryListContainer;

#[derive(Component)]
pub struct NodeButton(pub String);

#[derive(Component)]
pub struct LibraryButton(pub String);

#[derive(Component)]
pub struct HasTriggerToggle;

#[derive(Component)]
pub struct HasCollisionToggle;

#[derive(Component)]
pub struct EditTargetRadioModel;

#[derive(Component)]
pub struct EditTargetRadioTrigger;

#[derive(Component)]
pub struct EditTargetRadioCollision;

#[derive(Component)]
pub struct SaveButton;

#[derive(Component)]
pub struct NewButton;

#[derive(Component)]
pub struct DeleteButton;

#[derive(Component)]
pub struct BackButton;

#[derive(Component)]
pub struct NameFieldButton;

#[derive(Component)]
pub struct NameDisplayText;

#[derive(Component)]
pub struct HasTriggerText;

#[derive(Component)]
pub struct HasCollisionText;

#[derive(Component)]
pub struct AddCollisionShapeButton;

#[derive(Component)]
pub struct RemoveCollisionShapeButton;

#[derive(Component)]
pub struct PrevCollisionShapeButton;

#[derive(Component)]
pub struct NextCollisionShapeButton;

#[derive(Component)]
pub struct CollisionShapeLabel;

pub fn build_workshop_ui(commands: &mut Commands, library: &ObstacleLibrary) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            DespawnOnExit(DevMenuPage::ObstacleWorkshop),
        ))
        .with_children(|root| {
            build_left_panel(root, library);
            build_right_panel(root);
        });
}

fn build_left_panel(parent: &mut ChildSpawnerCommands, library: &ObstacleLibrary) {
    parent
        .spawn((
            Node {
                width: Val::Px(260.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(ui_theme::PANEL_BG),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Obstacle Workshop"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            panel.spawn((
                Text::new("Imported Objects"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));

            // Node list container (populated async when glTF loads)
            panel
                .spawn((
                    NodeListContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    container.spawn((
                        Text::new("Loading glTF..."),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(palette::CHAINMAIL),
                    ));
                });

            ui_theme::spawn_divider(panel);

            panel.spawn((
                Text::new("Obstacle Library"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            panel
                .spawn((
                    LibraryListContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    if library.definitions.is_empty() {
                        container.spawn((
                            Text::new("No obstacles defined"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(palette::CHAINMAIL),
                        ));
                    } else {
                        let mut ids: Vec<&ObstacleId> = library.definitions.keys().collect();
                        ids.sort_by(|a, b| a.0.cmp(&b.0));
                        for id in ids {
                            spawn_library_button(container, &id.0);
                        }
                    }
                });

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            ui_theme::spawn_panel_button(panel, "Back", BackButton);
        });
}

fn build_right_panel(parent: &mut ChildSpawnerCommands) {
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
            // Obstacle Name
            panel.spawn((
                Text::new("Obstacle Name"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));

            panel
                .spawn((
                    Button,
                    NameFieldButton,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(palette::BLACK),
                    BorderColor::all(palette::STEEL),
                ))
                .with_children(|field| {
                    field.spawn((
                        Text::new("(type a name)"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(palette::CHAINMAIL),
                        NameDisplayText,
                    ));
                });

            ui_theme::spawn_divider(panel);

            // Edit target toggle
            spawn_edit_target_row(panel);

            ui_theme::spawn_divider(panel);

            spawn_toggle_row(panel, "Trigger Volume", HasTriggerToggle, HasTriggerText, true);
            spawn_toggle_row(panel, "Collision Volume", HasCollisionToggle, HasCollisionText, false);

            // Collision shape navigation: [<] Shape 1/1 [>] [+] [-]
            spawn_collision_shape_row(panel);

            panel.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            ui_theme::spawn_divider(panel);
            ui_theme::spawn_action_button(panel, "Save Obstacle", SaveButton, palette::JUNGLE);
            ui_theme::spawn_action_button(panel, "New / Clear", NewButton, ui_theme::BUTTON_NORMAL);
            ui_theme::spawn_action_button(panel, "Delete", DeleteButton, palette::MAROON);
        });
}

pub fn spawn_node_button(parent: &mut ChildSpawnerCommands, name: &str) {
    parent
        .spawn((
            Button,
            NodeButton(name.to_string()),
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
                Text::new(name),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));
        });
}

pub(super) fn spawn_library_button(parent: &mut ChildSpawnerCommands, id: &str) {
    parent
        .spawn((
            Button,
            LibraryButton(id.to_string()),
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
                Text::new(id),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SEA_FOAM),
            ));
        });
}

fn spawn_toggle_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    toggle_marker: impl Component,
    text_marker: impl Component,
    initial: bool,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Button,
                toggle_marker,
                Node {
                    width: Val::Px(50.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(if initial { ui_theme::TOGGLE_ON } else { ui_theme::TOGGLE_OFF }),
                BorderColor::all(palette::STEEL),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(if initial { "ON" } else { "OFF" }),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                    text_marker,
                ));
            });

            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::SAND),
            ));
        });
}

fn spawn_edit_target_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            spawn_radio_option(row, "Model", EditTargetRadioModel, true);
            spawn_radio_option(row, "Trigger", EditTargetRadioTrigger, false);
            spawn_radio_option(row, "Collision", EditTargetRadioCollision, false);
        });
}

fn spawn_radio_option(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    active: bool,
) {
    let bg = if active { RADIO_ACTIVE } else { RADIO_INACTIVE };
    parent
        .spawn((
            Button,
            marker,
            Node {
                flex_grow: 1.0,
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

fn spawn_collision_shape_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            // Prev button
            spawn_small_button(row, "<", PrevCollisionShapeButton);

            // "Shape 0/0" label
            row.spawn((
                Text::new("Shape 0/0"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::SAND),
                CollisionShapeLabel,
            ));

            // Next button
            spawn_small_button(row, ">", NextCollisionShapeButton);

            // Add button
            spawn_small_button(row, "+", AddCollisionShapeButton);

            // Remove button
            spawn_small_button(row, "-", RemoveCollisionShapeButton);
        });
}

fn spawn_small_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
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
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

