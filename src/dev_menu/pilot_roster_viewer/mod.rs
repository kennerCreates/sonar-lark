mod build;

use bevy::prelude::*;

use crate::pilot::roster::{PilotRoster, save_roster_to_default};
use crate::pilot::PilotId;
use crate::states::{AppState, DevMenuPage};
use crate::ui_theme::UiFont;

use super::pilot_generator::format_skill;

// ── Component markers ──────────────────────────────────────────────────────

#[derive(Component)]
pub struct RosterBackButton;

#[derive(Component)]
pub struct PilotGeneratorButton;

#[derive(Component)]
pub struct PaletteEditorButton;

#[derive(Component)]
pub struct ObstacleWorkshopButton;

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

// ── Setup / Cleanup ────────────────────────────────────────────────────────

pub fn setup_roster_viewer(
    mut commands: Commands,
    roster: Option<Res<PilotRoster>>,
    font: Res<UiFont>,
) {
    let roster_ref = roster.as_deref().cloned().unwrap_or_default();
    build::build_ui(&mut commands, &roster_ref, &font.0);
    commands.insert_resource(RosterViewerDirty(false));
}

pub fn cleanup_roster_viewer(mut commands: Commands) {
    commands.remove_resource::<RosterViewerDirty>();
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

// ── Rebuild system ─────────────────────────────────────────────────────────

pub fn rebuild_roster_list(
    mut commands: Commands,
    mut dirty: ResMut<RosterViewerDirty>,
    roster: Option<Res<PilotRoster>>,
    font: Res<UiFont>,
    container_q: Query<Entity, With<RosterListContainer>>,
    mut count_q: Query<&mut Text, With<RosterCountLabel>>,
) {
    if !dirty.0 {
        return;
    }
    dirty.0 = false;

    let roster_ref = roster.as_deref().cloned().unwrap_or_default();

    // Rebuild the scrollable list contents
    for entity in &container_q {
        commands.entity(entity).despawn_related::<Children>();
        commands.entity(entity).with_children(|parent| {
            build::build_pilot_rows(parent, &roster_ref, &font.0);
        });
    }

    // Update count label
    for mut text in &mut count_q {
        text.0 = format!("Roster: {} pilots", roster_ref.pilots.len());
    }
}
