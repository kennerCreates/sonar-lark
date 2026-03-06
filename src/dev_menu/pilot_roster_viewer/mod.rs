mod build;

use std::collections::HashMap;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::pilot::portrait::loader::PortraitParts;
use crate::pilot::portrait::rasterize::rasterize_portrait;
use crate::pilot::roster::{PilotRoster, save_roster_to_default};
use crate::pilot::PilotId;
use crate::states::{AppState, DevMenuPage};
use crate::ui_theme::UiFont;

use super::pilot_generator::format_skill;

const PORTRAIT_THUMB_SIZE: u32 = 64;

// ── Component markers ──────────────────────────────────────────────────────

#[derive(Component)]
pub struct RosterBackButton;

#[derive(Component)]
pub struct PilotGeneratorButton;

#[derive(Component)]
pub struct PaletteEditorButton;

#[derive(Component)]
pub struct ObstacleWorkshopButton;

#[derive(Component)]
pub struct ClearRosterButton;

/// Marker on the scrollable container that holds pilot rows.
#[derive(Component)]
pub struct RosterListContainer;

/// Marker on a pilot row.
#[derive(Component)]
pub struct PilotRow;

/// Marker on the delete button inside a pilot row.
#[derive(Component)]
pub struct DeletePilotButton(pub PilotId);

/// Marker for the roster count label in the footer.
#[derive(Component)]
pub struct RosterCountLabel;

/// Tracks whether the list needs rebuilding (after a deletion).
#[derive(Resource)]
pub struct RosterViewerDirty(pub bool);

/// Portrait thumbnails rasterized for the roster viewer, keyed by PilotId.
#[derive(Resource, Default)]
pub struct RosterPortraitCache {
    pub portraits: HashMap<PilotId, Handle<Image>>,
}

// ── Setup / Cleanup ────────────────────────────────────────────────────────

pub fn setup_roster_viewer(
    mut commands: Commands,
    roster: Option<Res<PilotRoster>>,
    font: Res<UiFont>,
    parts: Option<Res<PortraitParts>>,
    mut images: ResMut<Assets<Image>>,
) {
    let roster_ref = roster.as_deref().cloned().unwrap_or_default();

    // Rasterize portrait thumbnails for all pilots
    let mut cache = RosterPortraitCache::default();
    if let Some(parts) = parts.as_deref() {
        for pilot in &roster_ref.pilots {
            let image = rasterize_portrait(
                &pilot.portrait,
                pilot.color_scheme.primary,
                PORTRAIT_THUMB_SIZE,
                parts,
            );
            let handle = images.add(image);
            cache.portraits.insert(pilot.id, handle);
        }
    }

    build::build_ui(&mut commands, &roster_ref, &font.0, &cache);
    commands.insert_resource(cache);
    commands.insert_resource(RosterViewerDirty(false));
}

pub fn cleanup_roster_viewer(mut commands: Commands) {
    commands.remove_resource::<RosterViewerDirty>();
    commands.remove_resource::<RosterPortraitCache>();
}

// ── Interaction systems ────────────────────────────────────────────────────

pub fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RosterBackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_pilot_generator_button(
    query: Query<&Interaction, (Changed<Interaction>, With<PilotGeneratorButton>)>,
    mut next_state: ResMut<NextState<DevMenuPage>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(DevMenuPage::PilotGenerator);
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

pub fn handle_delete_button(
    query: Query<(&Interaction, &DeletePilotButton), Changed<Interaction>>,
    mut roster: ResMut<PilotRoster>,
    mut dirty: ResMut<RosterViewerDirty>,
) {
    for (interaction, delete_btn) in &query {
        if *interaction == Interaction::Pressed {
            let pilot_id = delete_btn.0;
            if let Some(pos) = roster.pilots.iter().position(|p| p.id == pilot_id) {
                let removed = roster.pilots.remove(pos);
                info!("Deleted pilot #{}: {}", removed.id.0, removed.gamertag);
                save_roster_to_default(&roster);
                dirty.0 = true;
            }
        }
    }
}

pub fn handle_clear_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ClearRosterButton>)>,
    mut roster: ResMut<PilotRoster>,
    mut dirty: ResMut<RosterViewerDirty>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            let count = roster.pilots.len();
            roster.pilots.clear();
            save_roster_to_default(&roster);
            dirty.0 = true;
            info!("Cleared roster ({count} pilots removed)");
        }
    }
}

// ── Rebuild system ─────────────────────────────────────────────────────────

pub fn rebuild_roster_list(
    mut commands: Commands,
    mut dirty: ResMut<RosterViewerDirty>,
    roster: Option<Res<PilotRoster>>,
    font: Res<UiFont>,
    parts: Option<Res<PortraitParts>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<RosterPortraitCache>,
    container_q: Query<Entity, With<RosterListContainer>>,
    mut count_q: Query<&mut Text, With<RosterCountLabel>>,
) {
    if !dirty.0 {
        return;
    }
    dirty.0 = false;

    let roster_ref = roster.as_deref().cloned().unwrap_or_default();

    // Re-rasterize portraits for any new/remaining pilots
    if let Some(parts) = parts.as_deref() {
        for pilot in &roster_ref.pilots {
            if !cache.portraits.contains_key(&pilot.id) {
                let image = rasterize_portrait(
                    &pilot.portrait,
                    pilot.color_scheme.primary,
                    PORTRAIT_THUMB_SIZE,
                    parts,
                );
                let handle = images.add(image);
                cache.portraits.insert(pilot.id, handle);
            }
        }
    }

    // Rebuild the scrollable list contents
    for entity in &container_q {
        commands.entity(entity).despawn_related::<Children>();
        commands.entity(entity).with_children(|parent| {
            build::build_pilot_rows(parent, &roster_ref, &font.0, &cache);
        });
    }

    // Update count label
    for mut text in &mut count_q {
        text.0 = format!("Roster: {} pilots", roster_ref.pilots.len());
    }
}

const SCROLL_SPEED: f32 = 40.0;
const SCROLL_LINE_HEIGHT: f32 = 20.0;

pub fn mouse_wheel_scroll(
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut query: Query<(&Node, &mut ScrollPosition), With<RosterListContainer>>,
) {
    let mut dy = 0.0;
    for event in mouse_wheel.read() {
        dy += match event.unit {
            MouseScrollUnit::Line => -event.y * SCROLL_LINE_HEIGHT,
            MouseScrollUnit::Pixel => -event.y,
        };
    }
    if dy == 0.0 {
        return;
    }

    for (_node, mut scroll) in &mut query {
        scroll.0.y = (scroll.0.y + dy * SCROLL_SPEED / SCROLL_LINE_HEIGHT).max(0.0);
    }
}
