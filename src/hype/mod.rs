mod poster_editor;

use bevy::prelude::*;

use crate::palette;
use crate::states::{AdCampaign, HypeMode, SelectedAdCampaign, AD_CAMPAIGNS};
use crate::ui_theme;
use crate::ui_theme::UiFont;

pub struct HypePlugin;

impl Plugin for HypePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(HypeMode::CampaignSelector), setup_ui)
            .add_systems(
                Update,
                (handle_campaign_card, handle_select_button)
                    .run_if(in_state(HypeMode::CampaignSelector)),
            )
            .add_plugins(poster_editor::PosterEditorPlugin);
    }
}

#[derive(Resource)]
struct SelectedCard(AdCampaign);

#[derive(Component)]
struct CampaignCard(AdCampaign);

#[derive(Component)]
struct SelectButton;

#[derive(Component)]
struct SelectButtonText;

fn setup_ui(mut commands: Commands, font: Res<UiFont>) {
    let ui_font = font.0.clone();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(30.0),
                ..default()
            },
            BackgroundColor(palette::SMOKY_BLACK),
            DespawnOnExit(HypeMode::CampaignSelector),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Choose your Ad Campaign..."),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            let ui_font_inner = ui_font.clone();
            // Card row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(60.0),
                    margin: UiRect::vertical(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    for campaign in AD_CAMPAIGNS {
                        spawn_campaign_card(row, campaign, ui_font_inner.clone());
                    }
                });

            let ui_font_btn = ui_font.clone();
            // SELECT button (bottom-right)
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
                            SelectButton,
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(60.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(3.0)),
                                ..default()
                            },
                            BackgroundColor(ui_theme::BUTTON_DISABLED),
                            BorderColor::all(ui_theme::BORDER_DISABLED),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("SELECT"),
                                TextFont {
                                    font: ui_font_btn.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(palette::CHAINMAIL),
                                SelectButtonText,
                            ));
                        });
                });
        });
}

fn spawn_campaign_card(parent: &mut ChildSpawnerCommands, campaign: AdCampaign, ui_font: Handle<Font>) {
    let enabled = campaign.is_enabled();
    let (bg, border, text_color) = if enabled {
        (
            ui_theme::BUTTON_NORMAL,
            ui_theme::BORDER_NORMAL,
            palette::VANILLA,
        )
    } else {
        (
            ui_theme::BUTTON_DISABLED,
            ui_theme::BORDER_DISABLED,
            palette::CHAINMAIL,
        )
    };

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|col| {
            // Card button
            let mut card = col.spawn((
                Button,
                CampaignCard(campaign),
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(220.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(bg),
                BorderColor::all(border),
            ));
            if enabled {
                card.insert(ui_theme::ThemedButton);
            }
            let ui_font_card = ui_font.clone();
            card.with_children(|btn| {
                btn.spawn((
                    Text::new(campaign.label()),
                    TextFont {
                        font: ui_font_card.clone(),
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(text_color),
                    TextLayout::new_with_justify(Justify::Center),
                ));
            });

            // Price label
            col.spawn((
                Text::new(campaign.cost_label()),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
}

fn handle_campaign_card(
    mut commands: Commands,
    card_query: Query<(&Interaction, &CampaignCard), Changed<Interaction>>,
    mut all_cards: Query<(&CampaignCard, &mut BorderColor)>,
    mut select_btn: Query<&mut BackgroundColor, With<SelectButton>>,
    mut select_border: Query<&mut BorderColor, (With<SelectButton>, Without<CampaignCard>)>,
    mut select_text: Query<&mut TextColor, With<SelectButtonText>>,
) {
    for (interaction, clicked) in &card_query {
        if *interaction != Interaction::Pressed || !clicked.0.is_enabled() {
            continue;
        }

        commands.insert_resource(SelectedCard(clicked.0));

        // Highlight selected card, reset others
        for (card, mut border) in &mut all_cards {
            if card.0 == clicked.0 {
                *border = BorderColor::all(palette::VANILLA);
            } else {
                *border = BorderColor::all(ui_theme::BORDER_NORMAL);
            }
        }

        // Enable the SELECT button
        if let Ok(mut bg) = select_btn.single_mut() {
            *bg = BackgroundColor(ui_theme::BUTTON_NORMAL);
        }
        if let Ok(mut border) = select_border.single_mut() {
            *border = BorderColor::all(ui_theme::BORDER_NORMAL);
        }
        if let Ok(mut text_color) = select_text.single_mut() {
            *text_color = TextColor(palette::VANILLA);
        }
    }
}

fn handle_select_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SelectButton>)>,
    selected: Option<Res<SelectedCard>>,
    mut next_hype: ResMut<NextState<HypeMode>>,
) {
    let Some(selected) = selected else { return };

    for interaction in &query {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(SelectedAdCampaign(selected.0));
            commands.remove_resource::<SelectedCard>();
            next_hype.set(HypeMode::PosterEditor);
        }
    }
}
