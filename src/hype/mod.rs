pub(crate) mod poster_editor;

use bevy::prelude::*;

use crate::league::marketing::{compute_marketing_effects, CampaignBudgets};
use crate::league::LeagueState;
use crate::menu::ui::SkipToLocationSelect;
use crate::palette;
use crate::states::{AdCampaign, AppState, HypeMode, SelectedAdCampaign};
use crate::ui_theme::{self, UiFont};

pub struct HypePlugin;

impl Plugin for HypePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(HypeMode::CampaignSelector), setup_campaign_ui)
            .add_systems(
                Update,
                (
                    handle_budget_button,
                    handle_run_campaigns,
                    handle_done_button,
                    handle_edit_poster_button,
                )
                    .run_if(in_state(HypeMode::CampaignSelector)),
            )
            .add_plugins(poster_editor::PosterEditorPlugin);
    }
}

// --- Components ---

#[derive(Component)]
struct BudgetUpButton(BudgetKind);

#[derive(Component)]
struct BudgetDownButton(BudgetKind);

#[derive(Component)]
struct BudgetValueText(BudgetKind);

#[derive(Component)]
struct EffectPreviewText;

#[derive(Component)]
struct RunCampaignsButton;

#[derive(Component)]
struct CampaignsDoneButton;

#[derive(Component)]
struct EditPosterButton;

#[derive(Component)]
struct MoneyText;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BudgetKind {
    Posters,
    HighlightReel,
    Merch,
}

#[derive(Resource)]
struct PendingBudgets {
    posters: f32,
    highlight_reel: f32,
    merch: f32,
    applied: bool,
}

impl Default for PendingBudgets {
    fn default() -> Self {
        Self {
            posters: 0.0,
            highlight_reel: 0.0,
            merch: 0.0,
            applied: false,
        }
    }
}

const BUDGET_STEP: f32 = 5.0;
const BUDGET_MAX: f32 = 100.0;

fn setup_campaign_ui(mut commands: Commands, font: Res<UiFont>, league: Option<Res<LeagueState>>) {
    let ui_font = font.0.clone();
    let money = league.as_ref().map(|l| l.money).unwrap_or(0.0);

    commands.insert_resource(PendingBudgets::default());

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(palette::SMOKY_BLACK),
            DespawnOnExit(HypeMode::CampaignSelector),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("CAMPAIGN SETUP"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 40.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            // Money display
            parent.spawn((
                MoneyText,
                Text::new(format!("Budget: ${money:.0}")),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(palette::SUNSHINE),
            ));

            // Budget rows
            let ui_font_row = ui_font.clone();
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|col| {
                    spawn_budget_row(col, &ui_font_row, "Posters", BudgetKind::Posters);
                    spawn_budget_row(col, &ui_font_row, "Highlight Reel", BudgetKind::HighlightReel);
                    spawn_budget_row(col, &ui_font_row, "Merch", BudgetKind::Merch);
                });

            // Effect preview
            parent.spawn((
                EffectPreviewText,
                Text::new("Set budgets to see expected effects"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
                Node {
                    max_width: Val::Px(500.0),
                    ..default()
                },
            ));

            // Button row
            let ui_font_btn = ui_font.clone();
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                })
                .with_children(|row| {
                    ui_theme::spawn_menu_button(
                        row,
                        "EDIT POSTER",
                        EditPosterButton,
                        200.0,
                        &ui_font_btn,
                    );
                    ui_theme::spawn_menu_button(
                        row,
                        "RUN CAMPAIGNS",
                        RunCampaignsButton,
                        240.0,
                        &ui_font_btn,
                    );
                    ui_theme::spawn_menu_button(
                        row,
                        "SKIP",
                        CampaignsDoneButton,
                        140.0,
                        &ui_font_btn,
                    );
                });
        });
}

fn spawn_budget_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &Handle<Font>,
    label: &str,
    kind: BudgetKind,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            // Label
            row.spawn((
                Text::new(format!("{label:>15}")),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::SAND),
                Node {
                    width: Val::Px(140.0),
                    ..default()
                },
            ));

            // Down button
            row.spawn((
                Button,
                ui_theme::ThemedButton,
                BudgetDownButton(kind),
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

            // Value text
            row.spawn((
                BudgetValueText(kind),
                Text::new("$0"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
                Node {
                    width: Val::Px(60.0),
                    ..default()
                },
            ));

            // Up button
            row.spawn((
                Button,
                ui_theme::ThemedButton,
                BudgetUpButton(kind),
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

fn handle_budget_button(
    up_query: Query<(&Interaction, &BudgetUpButton), Changed<Interaction>>,
    down_query: Query<(&Interaction, &BudgetDownButton), Changed<Interaction>>,
    mut budgets: ResMut<PendingBudgets>,
    league: Option<Res<LeagueState>>,
    mut value_texts: Query<(&BudgetValueText, &mut Text)>,
    mut preview_text: Query<
        &mut Text,
        (
            With<EffectPreviewText>,
            Without<BudgetValueText>,
            Without<MoneyText>,
        ),
    >,
) {
    let money = league.as_ref().map(|l| l.money).unwrap_or(0.0);
    let mut changed = false;

    for (interaction, btn) in &up_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let current_total = total_budget(&budgets);
        let val = budget_mut(&mut budgets, btn.0);
        let room = (money - current_total).max(0.0);
        let step = BUDGET_STEP.min(room);
        if step > 0.0 {
            *val = (*val + step).min(BUDGET_MAX);
        }
        changed = true;
    }

    for (interaction, btn) in &down_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let val = budget_mut(&mut budgets, btn.0);
        *val = (*val - BUDGET_STEP).max(0.0);
        changed = true;
    }

    if changed {
        for (vt, mut text) in &mut value_texts {
            let val = budget_val(&budgets, vt.0);
            text.0 = format!("${val:.0}");
        }

        update_preview(&budgets, &mut preview_text);
    }
}

fn update_preview(
    budgets: &PendingBudgets,
    preview_text: &mut Query<
        &mut Text,
        (
            With<EffectPreviewText>,
            Without<BudgetValueText>,
            Without<MoneyText>,
        ),
    >,
) {
    let cb = CampaignBudgets {
        posters: budgets.posters,
        highlight_reel: budgets.highlight_reel,
        merch: budgets.merch,
    };
    let effects = compute_marketing_effects(&cb);
    let total = total_budget(budgets);

    if let Ok(mut text) = preview_text.single_mut() {
        if total < 0.01 {
            text.0 = "Set budgets to see expected effects".into();
        } else {
            let mut parts = Vec::new();
            if effects.new_aware_count > 0 {
                parts.push(format!(
                    "~{} new people hear about you",
                    effects.new_aware_count
                ));
            }
            if effects.aware_attendance_nudge > 0.01 {
                parts.push(format!(
                    "+{:.0}% attendance nudge",
                    effects.aware_attendance_nudge * 100.0
                ));
            }
            if effects.spread_potency_mult > 1.01 {
                parts.push(format!(
                    "{:.0}% spread boost",
                    (effects.spread_potency_mult - 1.0) * 100.0
                ));
            }
            if effects.spread_volume_bonus > 0 {
                parts.push(format!("+{} spread rolls", effects.spread_volume_bonus));
            }
            if effects.decay_slowdown {
                parts.push("Slower decay".into());
            }
            text.0 = format!("Cost: ${total:.0} | {}", parts.join(" | "));
        }
    }
}

fn handle_run_campaigns(
    query: Query<&Interaction, (Changed<Interaction>, With<RunCampaignsButton>)>,
    mut budgets: ResMut<PendingBudgets>,
    mut league: Option<ResMut<LeagueState>>,
    mut money_text: Query<&mut Text, With<MoneyText>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if budgets.applied {
            continue;
        }

        let total = total_budget(&budgets);
        if let Some(ref mut league) = league {
            if total > league.money {
                warn!("Not enough money for campaigns (need ${total:.0}, have ${:.0})", league.money);
                continue;
            }
            league.money -= total;
            league.campaign_budgets = CampaignBudgets {
                posters: budgets.posters,
                highlight_reel: budgets.highlight_reel,
                merch: budgets.merch,
            };
            budgets.applied = true;
            info!(
                "Campaigns applied: posters={}, reel={}, merch={}, remaining=${}",
                budgets.posters, budgets.highlight_reel, budgets.merch, league.money
            );

            // Update money display
            if let Ok(mut text) = money_text.single_mut() {
                text.0 = format!("Budget: ${:.0} (campaigns applied!)", league.money);
            }

            let save_path = std::path::PathBuf::from("assets/league/league_state.ron");
            if let Err(e) = crate::persistence::save_ron(&**league, &save_path) {
                error!("Failed to save league state: {e}");
            }
        }
    }
}

fn handle_done_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<CampaignsDoneButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    budgets: Option<Res<PendingBudgets>>,
    mut league: Option<ResMut<LeagueState>>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // If campaigns weren't run, zero out budgets
        if let (Some(budgets), Some(league)) = (&budgets, &mut league)
            && !budgets.applied
        {
            league.campaign_budgets = CampaignBudgets::default();
        }

        commands.remove_resource::<PendingBudgets>();
        commands.insert_resource(SkipToLocationSelect);
        next_state.set(AppState::Menu);
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

fn budget_mut(budgets: &mut PendingBudgets, kind: BudgetKind) -> &mut f32 {
    match kind {
        BudgetKind::Posters => &mut budgets.posters,
        BudgetKind::HighlightReel => &mut budgets.highlight_reel,
        BudgetKind::Merch => &mut budgets.merch,
    }
}

fn budget_val(budgets: &PendingBudgets, kind: BudgetKind) -> f32 {
    match kind {
        BudgetKind::Posters => budgets.posters,
        BudgetKind::HighlightReel => budgets.highlight_reel,
        BudgetKind::Merch => budgets.merch,
    }
}

fn total_budget(budgets: &PendingBudgets) -> f32 {
    budgets.posters + budgets.highlight_reel + budgets.merch
}
