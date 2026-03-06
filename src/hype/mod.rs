pub(crate) mod poster_editor;

use bevy::prelude::*;

use crate::league::marketing::CampaignBudgets;
use crate::league::LeagueState;
use crate::palette;
use crate::states::{AdCampaign, AppState, HypeMode, SelectedAdCampaign};
use crate::ui_theme::{self, UiFont};

use poster_editor::PosterOrder;

pub struct HypePlugin;

impl Plugin for HypePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(HypeMode::CampaignSelector), setup_campaign_ui)
            .add_systems(
                Update,
                (
                    handle_campaign_toggle,
                    handle_edit_poster_button,
                    handle_ticket_price_buttons,
                    handle_start_race_button,
                )
                    .run_if(in_state(HypeMode::CampaignSelector)),
            )
            .add_plugins(poster_editor::PosterEditorPlugin);
    }
}

// --- Fixed campaign costs ---

const HIGHLIGHT_REEL_COST: f32 = 125.0;
const MERCH_COST: f32 = 200.0;

// --- Components ---

#[derive(Component)]
struct CampaignCheckbox(AdCampaign);

#[derive(Component)]
struct CampaignCheckboxFill(AdCampaign);

#[derive(Component)]
struct EditPosterButton;

#[derive(Component)]
struct StartRaceButton;

#[derive(Component)]
struct TicketPriceDownButton;

#[derive(Component)]
struct TicketPriceUpButton;

#[derive(Component)]
struct TicketPriceText;

#[derive(Component)]
struct PosterCostText;

// --- Toggle state ---

#[derive(Resource, Default)]
struct CampaignToggles {
    posters: bool,
    /// Saved poster count for when the checkbox is unchecked then rechecked.
    saved_poster_count: u32,
    ticket_price: u32,
}

fn setup_campaign_ui(
    mut commands: Commands,
    font: Res<UiFont>,
    poster_order: Option<Res<PosterOrder>>,
    league: Option<Res<LeagueState>>,
) {
    let ui_font = font.0.clone();
    let poster_count = poster_order.map(|o| o.count).unwrap_or(0);
    let poster_cost = poster_count as f32 / 25.0 * 5.0;
    let posters_auto_checked = poster_count > 0;
    let ticket_price = league.map(|l| l.ticket_price).unwrap_or(0);

    commands.insert_resource(CampaignToggles {
        posters: posters_auto_checked,
        saved_poster_count: poster_count,
        ticket_price,
    });

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
                Text::new("Choose your Marketing:"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 40.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Campaign rows
            let ui_font_row = ui_font.clone();
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(20.0),
                    align_items: AlignItems::Start,
                    ..default()
                })
                .with_children(|col| {
                    spawn_campaign_row(
                        col,
                        &ui_font_row,
                        "Posters",
                        AdCampaign::Posters,
                        poster_cost,
                        true,
                        posters_auto_checked,
                    );
                    spawn_campaign_row(
                        col,
                        &ui_font_row,
                        "Highlight Reel",
                        AdCampaign::HighlightReel,
                        HIGHLIGHT_REEL_COST,
                        false,
                        false,
                    );
                    spawn_campaign_row(
                        col,
                        &ui_font_row,
                        "Merch",
                        AdCampaign::Merch,
                        MERCH_COST,
                        false,
                        false,
                    );
                });

            // Ticket price row
            {
                let ui_font_ticket = ui_font.clone();
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("Ticket Price:"),
                            TextFont {
                                font: ui_font_ticket.clone(),
                                font_size: 28.0,
                                ..default()
                            },
                            TextColor(palette::VANILLA),
                        ));

                        // Down button
                        row.spawn((
                            Button,
                            ui_theme::ThemedButton,
                            TicketPriceDownButton,
                            Node {
                                width: Val::Px(36.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(ui_theme::BUTTON_NORMAL),
                            BorderColor::all(ui_theme::BORDER_NORMAL),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("-"),
                                TextFont {
                                    font: ui_font_ticket.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(palette::VANILLA),
                            ));
                        });

                        // Price text
                        let price_label = if ticket_price == 0 {
                            "FREE".to_string()
                        } else {
                            format!("${ticket_price}")
                        };
                        row.spawn((
                            TicketPriceText,
                            Text::new(price_label),
                            TextFont {
                                font: ui_font_ticket.clone(),
                                font_size: 28.0,
                                ..default()
                            },
                            TextColor(palette::FROG),
                            Node {
                                width: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            TextLayout::new_with_justify(Justify::Center),
                        ));

                        // Up button
                        row.spawn((
                            Button,
                            ui_theme::ThemedButton,
                            TicketPriceUpButton,
                            Node {
                                width: Val::Px(36.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(ui_theme::BUTTON_NORMAL),
                            BorderColor::all(ui_theme::BORDER_NORMAL),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("+"),
                                TextFont {
                                    font: ui_font_ticket.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(palette::VANILLA),
                            ));
                        });
                    });
            }

            // START RACE button — centered
            ui_theme::spawn_menu_button(
                parent,
                "START RACE",
                StartRaceButton,
                240.0,
                &ui_font,
            );
        });
}

fn spawn_campaign_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    label: &str,
    campaign: AdCampaign,
    cost: f32,
    enabled: bool,
    checked: bool,
) {
    let text_color = if enabled {
        palette::VANILLA
    } else {
        palette::CHAINMAIL
    };
    let cost_color = if enabled {
        palette::FROG
    } else {
        palette::CHAINMAIL
    };

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|row| {
            // Label
            row.spawn((
                Text::new(label),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(text_color),
                Node {
                    width: Val::Px(260.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Right),
            ));

            // Checkbox
            if enabled {
                row.spawn((
                    Button,
                    CampaignCheckbox(campaign),
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(palette::INDIGO),
                    BorderColor::all(palette::SIDEWALK),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        CampaignCheckboxFill(campaign),
                        Text::new(if checked { "X" } else { "" }),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
            } else {
                // Disabled checkbox — non-interactive
                row.spawn((
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(palette::SMOKY_BLACK),
                    BorderColor::all(palette::STEEL),
                ));
            }

            // Edit button
            if enabled {
                row.spawn((
                    Button,
                    ui_theme::ThemedButton,
                    EditPosterButton,
                    Node {
                        width: Val::Px(80.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_NORMAL),
                    BorderColor::all(ui_theme::BORDER_NORMAL),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("edit"),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
            } else {
                // Disabled edit button
                row.spawn((
                    Node {
                        width: Val::Px(80.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(ui_theme::BUTTON_DISABLED),
                    BorderColor::all(ui_theme::BORDER_DISABLED),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("edit"),
                        TextFont {
                            font: ui_font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(palette::CHAINMAIL),
                    ));
                });
            }

            // Cost — hidden unless checked (for poster row)
            let cost_vis = if checked {
                Visibility::Inherited
            } else if enabled {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            let mut cost_cmd = row.spawn((
                Text::new(format!("${cost:.0}")),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(cost_color),
                Node {
                    width: Val::Px(100.0),
                    ..default()
                },
                cost_vis,
            ));
            if campaign == AdCampaign::Posters {
                cost_cmd.insert(PosterCostText);
            }
        });
}

fn handle_campaign_toggle(
    query: Query<(&Interaction, &CampaignCheckbox), Changed<Interaction>>,
    mut toggles: ResMut<CampaignToggles>,
    mut fills: Query<(&CampaignCheckboxFill, &mut Text)>,
    mut cost_text: Query<(&mut Visibility, &mut Text), (With<PosterCostText>, Without<CampaignCheckboxFill>)>,
    mut poster_order: Option<ResMut<PosterOrder>>,
) {
    for (interaction, checkbox) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if checkbox.0 == AdCampaign::Posters {
            toggles.posters = !toggles.posters;

            if let Some(ref mut order) = poster_order {
                if toggles.posters {
                    // Restore saved count
                    order.count = toggles.saved_poster_count;
                } else {
                    // Save count before clearing
                    toggles.saved_poster_count = order.count;
                    order.count = 0;
                }
            }

            // Show/hide poster cost
            for (mut vis, mut text) in &mut cost_text {
                if toggles.posters {
                    let count = poster_order.as_ref().map(|o| o.count).unwrap_or(0);
                    let cost = count as f32 / 25.0 * 5.0;
                    text.0 = format!("${cost:.0}");
                    *vis = Visibility::Inherited;
                } else {
                    *vis = Visibility::Hidden;
                }
            }
        }

        let is_on = match checkbox.0 {
            AdCampaign::Posters => toggles.posters,
            _ => false,
        };
        for (fill, mut text) in &mut fills {
            if std::mem::discriminant(&fill.0) == std::mem::discriminant(&checkbox.0) {
                text.0 = if is_on { "X".into() } else { String::new() };
            }
        }
    }
}

fn handle_edit_poster_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<EditPosterButton>)>,
    mut next_hype: ResMut<NextState<HypeMode>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(SelectedAdCampaign(AdCampaign::Posters));
            next_hype.set(HypeMode::PosterEditor);
        }
    }
}

fn handle_ticket_price_buttons(
    down_query: Query<&Interaction, (Changed<Interaction>, With<TicketPriceDownButton>)>,
    up_query: Query<&Interaction, (Changed<Interaction>, With<TicketPriceUpButton>)>,
    mut toggles: ResMut<CampaignToggles>,
    mut text_query: Query<&mut Text, With<TicketPriceText>>,
) {
    let mut changed = false;
    for interaction in &down_query {
        if *interaction == Interaction::Pressed && toggles.ticket_price > 0 {
            toggles.ticket_price -= 1;
            changed = true;
        }
    }
    for interaction in &up_query {
        if *interaction == Interaction::Pressed {
            toggles.ticket_price += 1;
            changed = true;
        }
    }
    if changed {
        for mut text in &mut text_query {
            text.0 = if toggles.ticket_price == 0 {
                "FREE".to_string()
            } else {
                format!("${}", toggles.ticket_price)
            };
        }
    }
}

fn handle_start_race_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<StartRaceButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    toggles: Option<Res<CampaignToggles>>,
    poster_order: Option<Res<PosterOrder>>,
    mut league: Option<ResMut<LeagueState>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Apply selected campaigns
        if let (Some(toggles), Some(league)) = (&toggles, &mut league) {
            let mut budgets = CampaignBudgets::default();
            let mut total_cost = 0.0;

            if toggles.posters {
                let count = poster_order.as_ref().map(|o| o.count).unwrap_or(0);
                let poster_cost = count as f32 / 25.0 * 5.0;
                budgets.posters = poster_cost;
                total_cost += poster_cost;
            }

            league.ticket_price = toggles.ticket_price;

            if total_cost <= league.money {
                league.money -= total_cost;
                league.campaign_budgets = budgets;
            } else {
                warn!(
                    "Not enough money for campaigns (need ${total_cost:.0}, have ${:.0})",
                    league.money
                );
                league.campaign_budgets = CampaignBudgets::default();
            }
        } else if let Some(ref mut league) = league {
            league.campaign_budgets = CampaignBudgets::default();
        }

        commands.remove_resource::<CampaignToggles>();
        commands.remove_resource::<PosterOrder>();
        commands.remove_resource::<poster_editor::SavedPosterData>();
        next_state.set(AppState::Race);
    }
}
