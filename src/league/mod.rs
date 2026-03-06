pub mod fan_network;
pub mod marketing;
#[allow(dead_code)] // Used starting in Step 6 (recruitment integration)
pub mod recruitment;

use std::path::PathBuf;

use bevy::prelude::*;

use crate::course::data::CourseData;
use crate::course::location::LocationRegistry;
use crate::race::track_quality::TrackQuality;
use crate::states::AppState;

use fan_network::{FanNetwork, RaceAttractionInputs};
use marketing::{CampaignBudgets, compute_marketing_effects};

/// Persistent league state: fan network + finances + campaign budgets.
/// Loaded from RON at startup, saved after each race.
#[derive(Resource, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LeagueState {
    pub fan_network: FanNetwork,
    pub money: f32,
    pub campaign_budgets: CampaignBudgets,
}

impl Default for LeagueState {
    fn default() -> Self {
        Self {
            fan_network: FanNetwork::new_seeded(42),
            money: 0.0,
            campaign_budgets: CampaignBudgets::default(),
        }
    }
}

fn league_save_path() -> PathBuf {
    PathBuf::from("assets/league/league_state.ron")
}

fn load_league_state(mut commands: Commands) {
    let state: LeagueState =
        crate::persistence::load_ron_or_default(&league_save_path());
    info!(
        "Loaded league state: {} fans, ${:.0}",
        state.fan_network.network_size(),
        state.money
    );
    commands.insert_resource(state);
    commands.insert_resource(LocationRegistry::new());
}

/// Runs on entering Results: simulates fan attraction from the last race.
fn simulate_fans_on_results(
    mut league: ResMut<LeagueState>,
    track_quality: Option<Res<TrackQuality>>,
    course: Option<Res<CourseData>>,
    location_registry: Res<LocationRegistry>,
    mut commands: Commands,
) {
    let tq_overall = track_quality.as_ref().map_or(0.5, |tq| tq.overall);

    let location_name = course
        .as_ref()
        .map(|c| c.location.as_str())
        .unwrap_or("Abandoned Warehouse");
    let location = location_registry.get(location_name);
    let (attractiveness, capacity) = location
        .map(|l| (l.base_attractiveness, l.capacity))
        .unwrap_or((0.2, 40));

    let marketing_effects = compute_marketing_effects(&league.campaign_budgets);

    let seed = league.fan_network.network_size()
        .wrapping_mul(31)
        .wrapping_add((tq_overall * 1000.0) as u32);

    let inputs = RaceAttractionInputs {
        track_quality: tq_overall,
        location_attractiveness: attractiveness,
        capacity,
        marketing: marketing_effects,
        seed,
    };

    let result = fan_network::simulate_race(&mut league.fan_network, &inputs);

    // Ticket revenue: $2 per attendee
    league.money += result.actual_attendance as f32 * 2.0;

    info!(
        "Fan simulation: {} attended ({} demand), network={}, fans={}, +${:.0}",
        result.actual_attendance,
        result.demand,
        result.network_size,
        result.fan_count,
        result.actual_attendance as f32 * 2.0,
    );

    commands.insert_resource(result);

    // Save updated league state
    if let Err(e) = crate::persistence::save_ron(&*league, &league_save_path()) {
        error!("Failed to save league state: {e}");
    }
}

pub struct LeaguePlugin;

impl Plugin for LeaguePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_league_state)
            .add_systems(OnEnter(AppState::Results), simulate_fans_on_results);
    }
}
