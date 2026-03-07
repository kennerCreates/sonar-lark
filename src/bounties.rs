use bevy::prelude::*;

use crate::league::bounties::{is_bounty_met, BountyId, BOUNTIES};
use crate::league::fan_network::RaceAttractionResult;
use crate::league::LeagueState;
use crate::menu::ui::SkipToLocationSelect;
use crate::palette;
use crate::states::AppState;
use crate::ui_theme::{self, UiFont};

#[derive(Component)]
struct ClaimBountyButton(BountyId);

#[derive(Component)]
struct SkipBountiesButton;

/// Tracks whether a bounty has already been claimed this round.
#[derive(Resource, Default)]
struct BountyClaimedThisRound(bool);

fn setup_bounties_ui(
    mut commands: Commands,
    font: Res<UiFont>,
    league: Option<Res<LeagueState>>,
    attraction: Option<Res<RaceAttractionResult>>,
) {
    commands.insert_resource(BountyClaimedThisRound(false));

    let league = league.as_deref().cloned().unwrap_or_default();
    let ui_font = font.0.clone();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.95)),
            DespawnOnExit(AppState::Bounties),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("BOUNTIES"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 40.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));

            parent.spawn((
                Text::new("One-time rewards — claim 1 per race"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(palette::STONE),
            ));

            // Money display
            parent.spawn((
                Text::new(format!("Money: ${:.0}", league.money)),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(palette::LIMON),
            ));

            // Bounty cards
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    margin: UiRect::vertical(Val::Px(12.0)),
                    ..default()
                })
                .with_children(|col| {
                    for def in BOUNTIES {
                        let claimed = league.claimed_bounties.contains(&def.id);
                        let met = attraction
                            .as_ref()
                            .map(|att| is_bounty_met(def.id, att, &league.fan_network))
                            .unwrap_or(false);
                        let claimable = !claimed && met;

                        spawn_bounty_row(col, def, claimed, claimable, &ui_font);
                    }
                });

            // Skip button
            ui_theme::spawn_menu_button(
                parent,
                "CONTINUE",
                SkipBountiesButton,
                200.0,
                &ui_font,
            );
        });
}

fn spawn_bounty_row(
    parent: &mut ChildSpawnerCommands,
    def: &crate::league::bounties::BountyDef,
    claimed: bool,
    claimable: bool,
    ui_font: &Handle<Font>,
) {
    let (bg_color, label_color, reward_color) = if claimed {
        (
            Color::srgba(0.1, 0.15, 0.1, 0.5),
            palette::STONE,
            palette::STONE,
        )
    } else if claimable {
        (
            Color::srgba(0.05, 0.15, 0.05, 0.8),
            palette::SEA_FOAM,
            palette::LIMON,
        )
    } else {
        (
            Color::srgba(0.08, 0.08, 0.12, 0.6),
            palette::SIDEWALK,
            palette::SIDEWALK,
        )
    };

    let mut row = parent.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            width: Val::Px(500.0),
            ..default()
        },
        BackgroundColor(bg_color),
    ));

    row.with_children(|row_children| {
        // Status indicator
        let status_text = if claimed {
            "CLAIMED"
        } else if claimable {
            "READY"
        } else {
            "LOCKED"
        };
        let status_color = if claimed {
            palette::STONE
        } else if claimable {
            palette::SEA_FOAM
        } else {
            palette::STEEL
        };

        row_children.spawn((
            Text::new(status_text),
            TextFont {
                font: ui_font.clone(),
                font_size: 11.0,
                ..default()
            },
            TextColor(status_color),
            Node {
                width: Val::Px(60.0),
                ..default()
            },
        ));

        // Label + description column
        row_children
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|info| {
                info.spawn((
                    Text::new(def.label),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(label_color),
                ));
                info.spawn((
                    Text::new(def.description),
                    TextFont {
                        font: ui_font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(palette::STONE),
                ));
            });

        // Reward
        row_children.spawn((
            Text::new(format!("${:.0}", def.reward)),
            TextFont {
                font: ui_font.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(reward_color),
            Node {
                width: Val::Px(50.0),
                ..default()
            },
        ));

        // Claim button (only if claimable)
        if claimable {
            ui_theme::spawn_menu_button(
                row_children,
                "CLAIM",
                ClaimBountyButton(def.id),
                90.0,
                ui_font,
            );
        }
    });
}

fn handle_claim_button(
    mut commands: Commands,
    query: Query<(&Interaction, &ClaimBountyButton), Changed<Interaction>>,
    mut league: ResMut<LeagueState>,
    mut claimed_this_round: ResMut<BountyClaimedThisRound>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, claim) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if claimed_this_round.0 {
            continue;
        }
        if league.claimed_bounties.contains(&claim.0) {
            continue;
        }

        let reward = BOUNTIES
            .iter()
            .find(|b| b.id == claim.0)
            .map(|b| b.reward)
            .unwrap_or(0.0);

        league.claimed_bounties.push(claim.0);
        league.money += reward;
        claimed_this_round.0 = true;

        info!("Claimed bounty {:?}, +${:.0}", claim.0, reward);

        // Immediately proceed after claiming
        commands.insert_resource(SkipToLocationSelect);
        next_state.set(AppState::Menu);
    }
}

fn handle_skip_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SkipBountiesButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(SkipToLocationSelect);
            next_state.set(AppState::Menu);
        }
    }
}

fn cleanup_bounties(mut commands: Commands) {
    commands.remove_resource::<BountyClaimedThisRound>();
    commands.remove_resource::<RaceAttractionResult>();
}

pub struct BountiesPlugin;

impl Plugin for BountiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Bounties), setup_bounties_ui)
            .add_systems(
                Update,
                (handle_claim_button, handle_skip_button)
                    .run_if(in_state(AppState::Bounties)),
            )
            .add_systems(OnExit(AppState::Bounties), cleanup_bounties);
    }
}
