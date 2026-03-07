pub(crate) mod poster_editor;

use bevy::prelude::*;

use crate::course::data::CourseData;
use crate::course::location::LocationRegistry;
use crate::league::marketing::CampaignBudgets;
use crate::league::LeagueState;
use crate::palette;
use crate::states::{AdCampaign, AppState, HypeMode, SelectedAdCampaign};
use crate::ui_theme::{self, UiFont};

use poster_editor::{canvas, poster_cost, PosterEditorOrigin, PosterOrder, SavedPosterData, POSTER_FREE_TIER};

pub struct HypePlugin;

impl Plugin for HypePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
                OnEnter(HypeMode::CampaignSelector),
                redirect_to_poster_editor.before(setup_campaign_ui),
            )
            .add_systems(OnEnter(HypeMode::CampaignSelector), setup_campaign_ui)
            .add_systems(
                Update,
                (
                    handle_campaign_toggle,
                    handle_edit_poster_button,
                    handle_ticket_price_buttons,
                    handle_campaign_poster_count,
                    handle_start_race_button,
                    update_money_summary,
                )
                    .run_if(in_state(HypeMode::CampaignSelector)),
            )
            .add_plugins(poster_editor::PosterEditorPlugin);
    }
}

// --- Fixed campaign costs (for future use when these campaigns are enabled) ---

#[allow(dead_code)]
const HIGHLIGHT_REEL_COST: f32 = 125.0;
#[allow(dead_code)]
const MERCH_COST: f32 = 200.0;

// --- Poster preview display dimensions (2:3 ratio matching canvas) ---

const PREVIEW_WIDTH: f32 = 200.0;
const PREVIEW_HEIGHT: f32 = 300.0;

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
struct MoneyTotalText;

#[derive(Component)]
struct MarketingCostText;

#[derive(Component)]
struct AfterMarketingText;

#[derive(Component)]
struct FinalRemainingText;

#[derive(Component)]
struct CampaignCountUp;

#[derive(Component)]
struct CampaignCountDown;

#[derive(Component)]
struct CampaignCountLabel;

// --- Toggle state ---

#[derive(Resource, Default)]
struct CampaignToggles {
    posters: bool,
    /// Saved poster count for when the checkbox is unchecked then rechecked.
    saved_poster_count: u32,
    ticket_price: u32,
    venue_capacity: u32,
}

fn redirect_to_poster_editor(
    origin: Option<Res<PosterEditorOrigin>>,
    mut next_hype: ResMut<NextState<HypeMode>>,
) {
    if let Some(origin) = &origin
        && **origin == PosterEditorOrigin::Menu
    {
        next_hype.set(HypeMode::PosterEditor);
    }
}

fn setup_campaign_ui(
    mut commands: Commands,
    font: Res<UiFont>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    saved: Option<Res<SavedPosterData>>,
    poster_order: Option<Res<PosterOrder>>,
    league: Option<Res<LeagueState>>,
    course: Option<Res<CourseData>>,
    location_registry: Res<LocationRegistry>,
) {
    let ui_font = font.0.clone();
    let poster_count = poster_order.map(|o| o.count).unwrap_or(POSTER_FREE_TIER);
    let posters_auto_checked = true;
    let league_state = league.as_deref().cloned().unwrap_or_default();
    let ticket_price = league_state.ticket_price;
    let total_money = league_state.money;

    let location_name = course
        .as_ref()
        .map(|c| c.location.as_str())
        .unwrap_or("Abandoned Warehouse");
    let venue_capacity = location_registry
        .get(location_name)
        .map(|l| l.capacity)
        .unwrap_or(80);

    commands.insert_resource(CampaignToggles {
        posters: posters_auto_checked,
        saved_poster_count: poster_count,
        ticket_price,
        venue_capacity,
    });
    commands.init_resource::<PosterOrder>();

    // Create poster preview image
    let preview_handle = if let Some(ref saved) = saved {
        let mut img = canvas::create_blank_canvas();
        *img.data.as_mut().unwrap() = saved.image_data.clone();
        images.add(img)
    } else {
        images.add(canvas::create_blank_canvas())
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(60.0),
                ..default()
            },
            BackgroundColor(palette::SMOKY_BLACK),
            DespawnOnExit(HypeMode::CampaignSelector),
        ))
        .with_children(|root| {
            // Left: poster preview column (preview + edit button)
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|col| {
                // Clickable poster preview with border
                col.spawn((
                    Button,
                    EditPosterButton,
                    Node {
                        width: Val::Px(PREVIEW_WIDTH + 4.0),
                        height: Val::Px(PREVIEW_HEIGHT + 4.0),
                        border: UiRect::all(Val::Px(2.0)),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor::all(palette::SIDEWALK),
                ))
                .with_children(|frame| {
                    frame.spawn((
                        ImageNode::new(preview_handle),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));

                    // Recreate text overlays on preview (scaled down)
                    if let Some(ref saved) = saved {
                        let scale = PREVIEW_WIDTH / 450.0;
                        for text_el in &saved.texts {
                            let font_handle: Handle<Font> =
                                asset_server.load(poster_editor::POSTER_FONTS[text_el.font_index].1);
                            let [r, g, b, _] = text_el.color;
                            frame.spawn((
                                Text::new(&text_el.content),
                                TextFont {
                                    font: font_handle,
                                    font_size: text_el.font_size * scale,
                                    ..default()
                                },
                                TextColor(Color::srgb_u8(r, g, b)),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(text_el.x * scale),
                                    top: Val::Px(text_el.y * scale),
                                    ..default()
                                },
                            ));
                        }
                    }
                });

                // "edit" label below preview
                col.spawn((
                    Text::new("click to edit"),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(palette::SIDEWALK),
                ));
            });

            // Right: controls column
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(30.0),
                ..default()
            })
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
                        spawn_poster_row(
                            col,
                            &ui_font_row,
                            posters_auto_checked,
                            poster_count,
                        );
                        spawn_campaign_row(
                            col,
                            &ui_font_row,
                            "Highlight Reel",
                            AdCampaign::HighlightReel,
                        );
                        spawn_campaign_row(
                            col,
                            &ui_font_row,
                            "Merch",
                            AdCampaign::Merch,
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

                // Money summary
                {
                    let marketing_cost = if posters_auto_checked { poster_cost(poster_count) } else { 0.0 };
                    let after_marketing = total_money - marketing_cost;
                    let venue_revenue = venue_capacity as f32 * ticket_price as f32;
                    let final_remaining = after_marketing + venue_revenue;

                    let ui_font_money = ui_font.clone();
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::End,
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|col| {
                            spawn_money_row(col, &ui_font_money, "Total Money:", &format!("${total_money:.0}"), MoneyTotalText, palette::VANILLA);
                            spawn_money_row(col, &ui_font_money, "Marketing:", &format!("-${marketing_cost:.0}"), MarketingCostText, palette::SALMON);
                            spawn_money_row(col, &ui_font_money, "After Marketing:", &format!("${after_marketing:.0}"), AfterMarketingText, palette::VANILLA);
                            spawn_money_row(col, &ui_font_money, "Remaining:", &format!("${final_remaining:.0}"), FinalRemainingText, palette::FROG);
                        });
                }

                // START RACE button
                ui_theme::spawn_menu_button(
                    parent,
                    "START RACE",
                    StartRaceButton,
                    240.0,
                    &ui_font,
                );
            });
        });
}

/// Poster row: label, checkbox, edit button, [-] count [+]
fn spawn_poster_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    checked: bool,
    poster_count: u32,
) {
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
                Text::new("Posters"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
                Node {
                    width: Val::Px(260.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Right),
            ));

            // Checkbox
            row.spawn((
                Button,
                CampaignCheckbox(AdCampaign::Posters),
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
                    CampaignCheckboxFill(AdCampaign::Posters),
                    Text::new(if checked { "X" } else { "" }),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));
            });

            // Count down button
            row.spawn((
                Button,
                ui_theme::ThemedButton,
                CampaignCountDown,
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
                        font: ui_font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));
            });

            // Count text
            row.spawn((
                CampaignCountLabel,
                Text::new(format!("{poster_count}")),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
                Node {
                    width: Val::Px(60.0),
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Center),
            ));

            // Count up button
            row.spawn((
                Button,
                ui_theme::ThemedButton,
                CampaignCountUp,
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
                        font: ui_font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));
            });
        });
}

/// Disabled campaign row: label + disabled checkbox + disabled edit button (no cost).
fn spawn_campaign_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    label: &str,
    _campaign: AdCampaign,
) {
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
                TextColor(palette::CHAINMAIL),
                Node {
                    width: Val::Px(260.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Right),
            ));

            // Disabled checkbox
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

        });
}

fn spawn_money_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    label: &str,
    value: &str,
    marker: impl Component,
    value_color: Color,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
                Node {
                    width: Val::Px(220.0),
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Right),
            ));
            row.spawn((
                marker,
                Text::new(value),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(value_color),
                Node {
                    width: Val::Px(120.0),
                    ..default()
                },
                TextLayout::new_with_justify(Justify::Right),
            ));
        });
}

fn handle_campaign_toggle(
    query: Query<(&Interaction, &CampaignCheckbox), Changed<Interaction>>,
    toggles: ResMut<CampaignToggles>,
    mut fills: Query<(&CampaignCheckboxFill, &mut Text), Without<CampaignCountLabel>>,
) {
    for (interaction, checkbox) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if checkbox.0 == AdCampaign::Posters {
            // Posters are always active (first 25 are free), ignore toggle
            continue;
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

fn handle_campaign_poster_count(
    down: Query<&Interaction, (Changed<Interaction>, With<CampaignCountDown>)>,
    up: Query<&Interaction, (Changed<Interaction>, With<CampaignCountUp>)>,
    mut poster_order: Option<ResMut<PosterOrder>>,
    mut toggles: ResMut<CampaignToggles>,
    mut count_text: Query<&mut Text, (With<CampaignCountLabel>, Without<CampaignCheckboxFill>)>,
    mut fills: Query<(&CampaignCheckboxFill, &mut Text), Without<CampaignCountLabel>>,
) {
    let Some(ref mut order) = poster_order else {
        return;
    };
    let mut changed = false;

    for interaction in &down {
        if *interaction == Interaction::Pressed && order.count > POSTER_FREE_TIER {
            order.count -= 25;
            changed = true;
        }
    }
    for interaction in &up {
        if *interaction == Interaction::Pressed {
            order.count += 25;
            changed = true;
        }
    }

    if changed {
        toggles.posters = true;
        toggles.saved_poster_count = order.count;

        for mut text in &mut count_text {
            text.0 = format!("{}", order.count);
        }

        // Update checkbox fill to match
        for (fill, mut text) in &mut fills {
            if fill.0 == AdCampaign::Posters {
                text.0 = if toggles.posters {
                    "X".into()
                } else {
                    String::new()
                };
            }
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
                let count = poster_order.as_ref().map(|o| o.count).unwrap_or(POSTER_FREE_TIER);
                // All posters contribute to marketing effect
                budgets.posters = count as f32 / 25.0 * 5.0;
                // But only posters beyond the free tier cost money
                total_cost += poster_cost(count);
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

#[allow(clippy::type_complexity)]
fn update_money_summary(
    toggles: Res<CampaignToggles>,
    league: Option<Res<LeagueState>>,
    poster_order: Option<Res<PosterOrder>>,
    mut marketing_text: Query<&mut Text, (With<MarketingCostText>, Without<AfterMarketingText>, Without<FinalRemainingText>)>,
    mut after_text: Query<&mut Text, (With<AfterMarketingText>, Without<MarketingCostText>, Without<FinalRemainingText>)>,
    mut final_text: Query<&mut Text, (With<FinalRemainingText>, Without<MarketingCostText>, Without<AfterMarketingText>)>,
) {
    let poster_changed = poster_order
        .as_ref()
        .is_some_and(|o| o.is_changed());
    if !toggles.is_changed() && !poster_changed {
        return;
    }

    let total_money = league.as_ref().map(|l| l.money).unwrap_or(205.0);
    let poster_count = poster_order.as_ref().map(|o| o.count).unwrap_or(POSTER_FREE_TIER);
    let marketing_cost = if toggles.posters {
        poster_cost(poster_count)
    } else {
        0.0
    };
    let after_marketing = total_money - marketing_cost;
    let venue_revenue = toggles.venue_capacity as f32 * toggles.ticket_price as f32;
    let final_remaining = after_marketing + venue_revenue;

    for mut text in &mut marketing_text {
        text.0 = format!("-${marketing_cost:.0}");
    }
    for mut text in &mut after_text {
        text.0 = format!("${after_marketing:.0}");
    }
    for mut text in &mut final_text {
        text.0 = format!("${final_remaining:.0}");
    }
}
