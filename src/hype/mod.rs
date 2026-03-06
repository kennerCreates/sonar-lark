pub(crate) mod poster_editor;

use bevy::prelude::*;

use crate::league::marketing::CampaignBudgets;
use crate::league::LeagueState;
use crate::menu::ui::SkipToLocationSelect;
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

// --- Toggle state ---

#[derive(Resource, Default)]
struct CampaignToggles {
    posters: bool,
}

fn setup_campaign_ui(
    mut commands: Commands,
    font: Res<UiFont>,
    poster_order: Option<Res<PosterOrder>>,
) {
    let ui_font = font.0.clone();
    let poster_count = poster_order.map(|o| o.count).unwrap_or(25);
    let poster_cost = poster_count as f32 / 25.0 * 5.0;

    commands.insert_resource(CampaignToggles::default());

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
                    align_items: AlignItems::Center,
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
                    );
                    spawn_campaign_row(
                        col,
                        &ui_font_row,
                        "Highlight Reel",
                        AdCampaign::HighlightReel,
                        HIGHLIGHT_REEL_COST,
                        false,
                    );
                    spawn_campaign_row(
                        col,
                        &ui_font_row,
                        "Merch",
                        AdCampaign::Merch,
                        MERCH_COST,
                        false,
                    );
                });

            // START RACE button — bottom right
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::FlexEnd,
                    padding: UiRect::right(Val::Px(40.0)),
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                })
                .with_children(|row| {
                    ui_theme::spawn_menu_button(
                        row,
                        "START RACE",
                        StartRaceButton,
                        240.0,
                        &ui_font,
                    );
                });
        });
}

fn spawn_campaign_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    label: &str,
    campaign: AdCampaign,
    cost: f32,
    enabled: bool,
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
                        Text::new(""),
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

            // Cost
            row.spawn((
                Text::new(format!("${cost:.0}")),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(cost_color),
            ));
        });
}

fn handle_campaign_toggle(
    query: Query<(&Interaction, &CampaignCheckbox), Changed<Interaction>>,
    mut toggles: ResMut<CampaignToggles>,
    mut fills: Query<(&CampaignCheckboxFill, &mut Text)>,
) {
    for (interaction, checkbox) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if checkbox.0 == AdCampaign::Posters {
            toggles.posters = !toggles.posters;
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
                let count = poster_order.as_ref().map(|o| o.count).unwrap_or(25);
                let poster_cost = count as f32 / 25.0 * 5.0;
                budgets.posters = poster_cost;
                total_cost += poster_cost;
            }

            if total_cost <= league.money {
                league.money -= total_cost;
                league.campaign_budgets = budgets;

                let save_path = std::path::PathBuf::from("assets/league/league_state.ron");
                if let Err(e) = crate::persistence::save_ron(&**league, &save_path) {
                    error!("Failed to save league state: {e}");
                }
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
        commands.insert_resource(SkipToLocationSelect);
        next_state.set(AppState::Menu);
    }
}
