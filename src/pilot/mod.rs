pub mod gamertag;
pub mod personality;
pub mod portrait;
pub mod roster;
pub mod skill;

use bevy::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::drone::components::DroneConfig;
use crate::race::progress::RaceResults;
use crate::states::AppState;

pub use portrait::PortraitDescriptor;

pub struct PilotPlugin;

impl Plugin for PilotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, roster::load_or_generate_roster)
            .add_systems(
                OnEnter(AppState::Race),
                (
                    select_pilots_for_race,
                    portrait::cache::setup_portrait_cache,
                )
                    .chain(),
            )
            .add_systems(OnEnter(AppState::Results), update_pilot_stats_after_race)
            .add_systems(OnExit(AppState::Results), cleanup_race_pilot_resources);
    }
}

/// Unique identifier for a pilot. Monotonically increasing u64 counter.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PilotId(pub u64);

/// Color scheme for a pilot's drone. Stored as sRGB [r, g, b] for clean RON serialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorScheme {
    pub primary: [f32; 3],
}

impl ColorScheme {
    pub fn to_color(&self) -> Color {
        Color::srgb(self.primary[0], self.primary[1], self.primary[2])
    }

    #[cfg(test)]
    pub fn from_color(color: Color) -> Self {
        let lin = color.to_linear();
        // Convert linear → sRGB for storage
        let srgba = Color::LinearRgba(lin).to_srgba();
        Self {
            primary: [srgba.red, srgba.green, srgba.blue],
        }
    }
}

/// Placeholder for Phase 3: drone build configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DroneBuildDescriptor {}

/// Accumulated stats across races.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PilotStats {
    pub races_entered: u32,
    pub finishes: u32,
    pub crashes: u32,
    pub wins: u32,
    pub best_time: Option<f32>,
}

/// A procedurally generated pilot identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pilot {
    pub id: PilotId,
    pub gamertag: String,
    pub personality: Vec<personality::PersonalityTrait>,
    pub skill: skill::SkillProfile,
    pub color_scheme: ColorScheme,
    #[serde(default)]
    pub drone_build: DroneBuildDescriptor,
    #[serde(default)]
    pub portrait: PortraitDescriptor,
    #[serde(default)]
    pub stats: PilotStats,
}

impl Pilot {
    pub fn generate_drone_config(&self, rng: &mut impl rand::Rng) -> DroneConfig {
        skill::generate_drone_config(&self.skill, &self.personality, rng)
    }
}

/// The 12 pilots selected for the current race, indexed by drone slot (0..12).
#[derive(Resource)]
pub struct SelectedPilots {
    pub pilots: Vec<SelectedPilot>,
}

/// Per-slot data for a pilot in the current race.
pub struct SelectedPilot {
    pub pilot_id: PilotId,
    pub gamertag: String,
    pub color: Color,
}

/// Pre-computed DroneConfigs from selected pilots, indexed by drone slot.
#[derive(Resource)]
pub struct PilotConfigs {
    pub configs: Vec<DroneConfig>,
}

fn select_pilots_for_race(mut commands: Commands, roster: Option<Res<roster::PilotRoster>>) {
    let Some(roster) = roster else {
        warn!("No PilotRoster resource — pilots will not be selected");
        return;
    };

    if roster.pilots.is_empty() {
        warn!("PilotRoster is empty — pilots will not be selected");
        return;
    }

    let mut rng = rand::thread_rng();

    // Shuffle indices and pick up to 12
    let mut indices: Vec<usize> = (0..roster.pilots.len()).collect();
    indices.shuffle(&mut rng);
    indices.truncate(12);

    // If roster has fewer than 12, cycle to fill slots
    while indices.len() < 12 {
        let extra = indices[indices.len() % roster.pilots.len()];
        indices.push(extra);
    }

    let mut selected = Vec::with_capacity(12);
    let mut configs = Vec::with_capacity(12);

    for &idx in &indices {
        let pilot = &roster.pilots[idx];
        selected.push(SelectedPilot {
            pilot_id: pilot.id,
            gamertag: pilot.gamertag.clone(),
            color: pilot.color_scheme.to_color(),
        });
        configs.push(pilot.generate_drone_config(&mut rng));
    }

    commands.insert_resource(SelectedPilots { pilots: selected });
    commands.insert_resource(PilotConfigs { configs });
    info!("Selected {} pilots for race", indices.len());
}

fn update_pilot_stats_after_race(
    results: Option<Res<RaceResults>>,
    selected: Option<Res<SelectedPilots>>,
    mut roster: Option<ResMut<roster::PilotRoster>>,
) {
    let (Some(results), Some(selected), Some(ref mut roster)) =
        (results, selected, roster.as_mut())
    else {
        return;
    };

    for entry in &results.standings {
        let drone_idx = entry.drone_index;
        let Some(sel) = selected.pilots.get(drone_idx) else {
            continue;
        };
        let Some(pilot) = roster.get_mut(sel.pilot_id) else {
            continue;
        };

        pilot.stats.races_entered += 1;
        if entry.finished {
            pilot.stats.finishes += 1;
            if let Some(time) = entry.finish_time {
                pilot.stats.best_time = Some(
                    pilot
                        .stats
                        .best_time
                        .map_or(time, |best| best.min(time)),
                );
            }
        }
        if entry.crashed {
            pilot.stats.crashes += 1;
        }
    }

    // Check if first finisher is a win
    if let Some(first) = results.standings.first()
        && first.finished
        && let Some(sel) = selected.pilots.get(first.drone_index)
        && let Some(pilot) = roster.get_mut(sel.pilot_id)
    {
        pilot.stats.wins += 1;
    }

    roster::save_roster_to_default(roster);
    info!("Updated pilot stats after race");
}

fn cleanup_race_pilot_resources(mut commands: Commands) {
    commands.remove_resource::<SelectedPilots>();
    commands.remove_resource::<PilotConfigs>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_scheme_roundtrip() {
        let original = Color::srgb(0.8, 0.2, 0.5);
        let scheme = ColorScheme::from_color(original);
        let restored = scheme.to_color();
        let orig_srgba = original.to_srgba();
        let rest_srgba = restored.to_srgba();
        assert!((orig_srgba.red - rest_srgba.red).abs() < 0.01);
        assert!((orig_srgba.green - rest_srgba.green).abs() < 0.01);
        assert!((orig_srgba.blue - rest_srgba.blue).abs() < 0.01);
    }
}
