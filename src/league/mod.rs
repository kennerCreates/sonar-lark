pub mod fan_network;
pub mod marketing;
#[allow(dead_code)] // Used starting in Step 6 (recruitment integration)
pub mod recruitment;

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
    /// Ticket price in whole dollars. 0 = FREE.
    #[serde(default)]
    pub ticket_price: u32,
}

impl Default for LeagueState {
    fn default() -> Self {
        Self {
            fan_network: FanNetwork::new_seeded(42),
            money: 205.0,
            campaign_budgets: CampaignBudgets::default(),
            ticket_price: 0,
        }
    }
}

fn init_league_state(mut commands: Commands) {
    commands.insert_resource(LeagueState::default());
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
        ticket_price: league.ticket_price,
        seed,
    };

    let result = fan_network::simulate_race(&mut league.fan_network, &inputs);

    // Ticket revenue
    let ticket_revenue = result.actual_attendance as f32 * league.ticket_price as f32;
    league.money += ticket_revenue;

    info!(
        "Fan simulation: {} attended ({} demand), network={}, fans={}, +${:.0}",
        result.actual_attendance,
        result.demand,
        result.network_size,
        result.fan_count,
        ticket_revenue,
    );

    commands.insert_resource(result);
}

pub struct LeaguePlugin;

impl Plugin for LeaguePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_league_state)
            .add_systems(OnEnter(AppState::Results), simulate_fans_on_results);
    }
}
