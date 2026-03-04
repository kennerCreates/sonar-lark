mod build;

use std::collections::HashSet;

use bevy::prelude::*;
use rand::Rng;

use crate::pilot::gamertag::generate_gamertag;
use crate::pilot::portrait::loader::PortraitParts;
use crate::pilot::portrait::rasterize::rasterize_portrait;
use crate::pilot::portrait::PortraitDescriptor;
use crate::pilot::roster::{PilotRoster, pick_personality_traits, save_roster_to_default};
use crate::pilot::skill::SkillProfile;
use crate::pilot::{ColorScheme, DroneBuildDescriptor, Pilot, PilotId, PilotStats};
use crate::states::{AppState, DevMenuPage};

use super::portrait_config::{
    PortraitColorSlot, PortraitPaletteConfig, PALETTE_COLORS, load_config,
};

// ── State resource ─────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct PilotGeneratorState {
    pub candidate: Pilot,
    pub preview_handle: Option<Handle<Image>>,
    pub preview_dirty: bool,
}

// ── Component markers ──────────────────────────────────────────────────────

#[derive(Component)]
pub struct GenBackButton;

#[derive(Component)]
pub struct PaletteEditorButton;

#[derive(Component)]
pub struct ObstacleWorkshopButton;

#[derive(Component)]
pub struct RerollPortraitButton;

#[derive(Component)]
pub struct RerollGamertagButton;

#[derive(Component)]
pub struct RerollPersonalityButton;

#[derive(Component)]
pub struct AcceptButton;

#[derive(Component)]
pub struct PreviewImage;

#[derive(Component)]
pub struct GamertagLabel;

#[derive(Component)]
pub struct PersonalityLabel;

#[derive(Component)]
pub struct DroneColorSwatch;

#[derive(Component)]
pub struct RosterCountLabel;

// ── Helpers ────────────────────────────────────────────────────────────────

fn generate_random_pilot(roster: &PilotRoster, config: &PortraitPaletteConfig) -> Pilot {
    let mut rng = rand::thread_rng();

    let used_tags: HashSet<String> = roster.pilots.iter().map(|p| p.gamertag.clone()).collect();
    let gamertag = generate_gamertag(&mut rng, &used_tags);
    let traits = pick_personality_traits(&mut rng);

    let skill = SkillProfile {
        level: rng.gen_range(0.2..=0.95),
        speed: rng.gen_range(0.2..=1.0),
        cornering: rng.gen_range(0.2..=1.0),
        consistency: rng.gen_range(0.2..=1.0),
    };

    let allowed_drone_colors = config.allowed_indices(PortraitColorSlot::Drone);
    let idx = allowed_drone_colors[rng.gen_range(0..allowed_drone_colors.len())];
    let drone_primary = PALETTE_COLORS[idx].1;
    let color = ColorScheme {
        primary: drone_primary,
    };

    let has_config = !config.vetoed.is_empty() || !config.complementary.is_empty();
    let portrait = if has_config {
        PortraitDescriptor::generate_with_config(&mut rng, color.primary, &PALETTE_COLORS, config)
    } else {
        PortraitDescriptor::generate(&mut rng, color.primary)
    };

    Pilot {
        id: PilotId(roster.next_id),
        gamertag,
        personality: traits,
        skill,
        color_scheme: color,
        drone_build: DroneBuildDescriptor::default(),
        portrait,
        stats: PilotStats::default(),
    }
}

fn format_personality(
    traits: &[crate::pilot::personality::PersonalityTrait],
) -> String {
    if traits.is_empty() {
        "None".to_string()
    } else {
        traits
            .iter()
            .map(|t| format!("{t:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

// ── Setup / Cleanup ────────────────────────────────────────────────────────

pub fn setup_pilot_generator(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    portrait_parts: Option<Res<PortraitParts>>,
    roster: Option<Res<PilotRoster>>,
) {
    let config = load_config();
    let roster_snapshot = roster
        .as_ref()
        .map(|r| (*r).clone())
        .unwrap_or_default();
    let candidate = generate_random_pilot(&roster_snapshot, &config);

    let mut state = PilotGeneratorState {
        candidate,
        preview_handle: None,
        preview_dirty: false,
    };

    let preview_handle = if let Some(ref parts) = portrait_parts {
        let image = rasterize_portrait(
            &state.candidate.portrait,
            state.candidate.color_scheme.primary,
            512,
            parts,
        );
        let handle = images.add(image);
        state.preview_handle = Some(handle.clone());
        Some(handle)
    } else {
        None
    };

    let roster_count = roster_snapshot.pilots.len();
    build::build_ui(&mut commands, &state, roster_count, preview_handle);
    commands.insert_resource(state);
    commands.insert_resource(config);
}

pub fn cleanup_pilot_generator(mut commands: Commands) {
    commands.remove_resource::<PilotGeneratorState>();
    commands.remove_resource::<PortraitPaletteConfig>();
}

// ── Interaction systems ────────────────────────────────────────────────────

pub fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<GenBackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_palette_editor_button(
    query: Query<&Interaction, (Changed<Interaction>, With<PaletteEditorButton>)>,
    mut next_state: ResMut<NextState<DevMenuPage>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(DevMenuPage::PaletteEditor);
        }
    }
}

pub fn handle_obstacle_workshop_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ObstacleWorkshopButton>)>,
    mut next_state: ResMut<NextState<DevMenuPage>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(DevMenuPage::ObstacleWorkshop);
        }
    }
}

pub fn handle_reroll_portrait_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RerollPortraitButton>)>,
    mut state: ResMut<PilotGeneratorState>,
    config: Res<PortraitPaletteConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            let mut rng = rand::thread_rng();

            let allowed = config.allowed_indices(PortraitColorSlot::Drone);
            let idx = allowed[rng.gen_range(0..allowed.len())];
            let drone_primary = PALETTE_COLORS[idx].1;
            state.candidate.color_scheme = ColorScheme {
                primary: drone_primary,
            };

            let has_config = !config.vetoed.is_empty() || !config.complementary.is_empty();
            state.candidate.portrait = if has_config {
                PortraitDescriptor::generate_with_config(
                    &mut rng,
                    state.candidate.color_scheme.primary,
                    &PALETTE_COLORS,
                    &config,
                )
            } else {
                PortraitDescriptor::generate(&mut rng, state.candidate.color_scheme.primary)
            };
            state.preview_dirty = true;
        }
    }
}

pub fn handle_reroll_gamertag_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RerollGamertagButton>)>,
    mut state: ResMut<PilotGeneratorState>,
    roster: Option<Res<PilotRoster>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            let mut rng = rand::thread_rng();
            let used_tags: HashSet<String> = roster
                .as_ref()
                .map(|r| r.pilots.iter().map(|p| p.gamertag.clone()).collect())
                .unwrap_or_default();
            state.candidate.gamertag = generate_gamertag(&mut rng, &used_tags);
        }
    }
}

pub fn handle_reroll_personality_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RerollPersonalityButton>)>,
    mut state: ResMut<PilotGeneratorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            let mut rng = rand::thread_rng();
            state.candidate.personality = pick_personality_traits(&mut rng);
            state.candidate.skill = SkillProfile {
                level: rng.gen_range(0.2..=0.95),
                speed: rng.gen_range(0.2..=1.0),
                cornering: rng.gen_range(0.2..=1.0),
                consistency: rng.gen_range(0.2..=1.0),
            };
        }
    }
}

pub fn handle_accept_button(
    query: Query<&Interaction, (Changed<Interaction>, With<AcceptButton>)>,
    mut state: ResMut<PilotGeneratorState>,
    mut roster: ResMut<PilotRoster>,
    config: Res<PortraitPaletteConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            // Assign the real ID and push to roster
            let mut pilot = state.candidate.clone();
            pilot.id = PilotId(roster.next_id);
            roster.next_id += 1;

            info!(
                "Accepted pilot #{}: {} ({:?})",
                pilot.id.0, pilot.gamertag, pilot.personality
            );
            roster.pilots.push(pilot);
            save_roster_to_default(&roster);

            // Generate the next candidate
            state.candidate = generate_random_pilot(&roster, &config);
            state.preview_dirty = true;
        }
    }
}

// ── Display systems ────────────────────────────────────────────────────────

pub fn update_preview(
    mut state: ResMut<PilotGeneratorState>,
    portrait_parts: Option<Res<PortraitParts>>,
    mut images: ResMut<Assets<Image>>,
    mut preview_query: Query<(Entity, Option<&mut ImageNode>), With<PreviewImage>>,
    mut commands: Commands,
) {
    if !state.preview_dirty {
        return;
    }
    state.preview_dirty = false;

    let Some(parts) = portrait_parts else {
        return;
    };

    let image = rasterize_portrait(
        &state.candidate.portrait,
        state.candidate.color_scheme.primary,
        512,
        &parts,
    );
    let handle = images.add(image);
    state.preview_handle = Some(handle.clone());

    for (entity, existing_image) in &mut preview_query {
        if let Some(mut img) = existing_image {
            img.image = handle.clone();
        } else {
            commands
                .entity(entity)
                .insert(ImageNode::new(handle.clone()));
        }
    }
}

pub fn update_pilot_info(
    state: Res<PilotGeneratorState>,
    roster: Option<Res<PilotRoster>>,
    mut gamertag_q: Query<
        &mut Text,
        (
            With<GamertagLabel>,
            Without<PersonalityLabel>,
            Without<RosterCountLabel>,
        ),
    >,
    mut personality_q: Query<
        &mut Text,
        (
            With<PersonalityLabel>,
            Without<GamertagLabel>,
            Without<RosterCountLabel>,
        ),
    >,
    mut swatch_q: Query<&mut BackgroundColor, With<DroneColorSwatch>>,
    mut count_q: Query<
        &mut Text,
        (
            With<RosterCountLabel>,
            Without<GamertagLabel>,
            Without<PersonalityLabel>,
        ),
    >,
) {
    if !state.is_changed() {
        return;
    }

    for mut text in &mut gamertag_q {
        text.0.clone_from(&state.candidate.gamertag);
    }
    for mut text in &mut personality_q {
        text.0 = format_personality(&state.candidate.personality);
    }
    for mut bg in &mut swatch_q {
        let [r, g, b] = state.candidate.color_scheme.primary;
        *bg = BackgroundColor(Color::srgb(r, g, b));
    }
    if let Some(roster) = roster {
        for mut text in &mut count_q {
            let msg = format!("Roster: {} pilots", roster.pilots.len());
            if text.0 != msg {
                text.0 = msg;
            }
        }
    }
}
