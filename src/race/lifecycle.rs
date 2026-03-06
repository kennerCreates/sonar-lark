use bevy::prelude::*;

use crate::camera::switching::{CameraMode, CameraState};
use crate::drone::components::{AIController, Drone, DroneConfig, DronePhase, RaceSeed};
use crate::pilot::roster::PilotRoster;
use crate::pilot::SelectedPilots;
use crate::states::AppState;

use super::progress::{DroneRaceState, RaceProgress};
use super::script::{self, DroneScriptInput, RaceEventLog, RaceScript};
use super::timing::RaceClock;

#[derive(Resource)]
pub struct RaceStartSound(pub Handle<bevy::audio::AudioSource>);

#[derive(Resource)]
pub struct RaceEndSound(pub Handle<bevy::audio::AudioSource>);

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum RacePhase {
    #[default]
    WaitingToStart,
    Converging,
    Countdown,
    Racing,
    Finished,
}

#[derive(Resource)]
pub struct CountdownTimer {
    pub remaining: f32,
    pub sound_played: bool,
}

impl Default for CountdownTimer {
    fn default() -> Self {
        // 3s visible 3-2-1 countdown (convergence is handled by RacePhase::Converging).
        Self {
            remaining: 3.0,
            sound_played: false,
        }
    }
}

/// Timer for auto-transitioning from Race → Results after finish.
#[derive(Resource)]
pub struct ResultsTransitionTimer {
    pub remaining: f32,
}

pub fn load_race_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RaceStartSound(asset_server.load("sounds/race_start.wav")));
    commands.insert_resource(RaceEndSound(asset_server.load("sounds/race_end.wav")));
}

/// Run condition: returns true when any drone is actively racing, converging, or wandering.
/// Used to keep AI/wander systems running instead of hover_target.
pub fn drones_are_active(
    phase: Option<Res<RacePhase>>,
    drones: Query<&DronePhase, With<Drone>>,
) -> bool {
    if phase.is_some_and(|p| matches!(*p, RacePhase::Racing | RacePhase::Converging)) {
        return true;
    }
    drones
        .iter()
        .any(|dp| matches!(*dp, DronePhase::VictoryLap | DronePhase::Wandering))
}

/// Ticks the countdown timer each frame, then transitions to Racing when it expires.
pub fn tick_countdown(
    time: Res<Time>,
    mut phase: ResMut<RacePhase>,
    mut timer: Option<ResMut<CountdownTimer>>,
    mut commands: Commands,
    mut drones: Query<(&mut DronePhase, &AIController), With<Drone>>,
    drone_count: Query<(), With<Drone>>,
    race_start_sound: Option<Res<RaceStartSound>>,
) {
    if *phase != RacePhase::Countdown {
        return;
    }
    let Some(ref mut timer) = timer else { return };

    timer.remaining -= time.delta_secs();

    // Play the countdown sound when the visible 3-2-1 begins
    if timer.remaining <= 3.0 && !timer.sound_played {
        timer.sound_played = true;
        if let Some(ref sound) = race_start_sound {
            commands.spawn((
                AudioPlayer::new(sound.0.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }

    if timer.remaining <= 0.0 {
        *phase = RacePhase::Racing;
        commands.remove_resource::<CountdownTimer>();

        // Start all drones racing
        let mut total_gates = 0u32;
        for (mut drone_phase, ai) in &mut drones {
            *drone_phase = DronePhase::Racing;
            total_gates = ai.gate_count;
        }

        // Start race clock
        commands.insert_resource(RaceClock {
            elapsed: 0.0,
            running: true,
        });

        // Initialize RaceProgress
        let drone_count = drone_count.iter().count();
        let drone_states = (0..drone_count)
            .map(|_| DroneRaceState::default())
            .collect();
        commands.insert_resource(RaceProgress {
            drone_states,
            total_gates,
        });

        info!(
            "GO! Race started with {} drones on {}-gate course",
            drone_count, total_gates
        );
    }
}

/// Generates the RaceScript resource once when RacePhase transitions to Racing.
/// Runs as a one-shot system in the race logic chain, after tick_countdown.
pub fn generate_race_script_system(
    mut commands: Commands,
    phase: Res<RacePhase>,
    existing_script: Option<Res<RaceScript>>,
    race_seed: Option<Res<RaceSeed>>,
    tuning: Res<crate::drone::components::AiTuningParams>,
    drones: Query<(&Drone, &AIController, &DroneConfig), With<Drone>>,
    selected_pilots: Option<Res<SelectedPilots>>,
    roster: Option<Res<PilotRoster>>,
) {
    if *phase != RacePhase::Racing || existing_script.is_some() {
        return;
    }

    let Some(seed_res) = race_seed else { return };
    if drones.is_empty() {
        return;
    }

    // Collect drone data sorted by index
    let mut drone_data: Vec<(&Drone, &AIController, &DroneConfig)> = drones.iter().collect();
    drone_data.sort_by_key(|(d, _, _)| d.index);

    let gate_count = drone_data[0].1.gate_count;
    if gate_count == 0 {
        return;
    }
    let gate_positions = &drone_data[0].1.gate_positions;

    // Build script inputs
    let inputs: Vec<DroneScriptInput> = drone_data
        .iter()
        .map(|(drone, ai, config)| {
            // Look up pilot skill/personality from roster
            let (skill, traits) = if let (Some(selected), Some(roster)) =
                (&selected_pilots, &roster)
            {
                if let Some(sel) = selected.pilots.get(drone.index as usize)
                    && let Some(pilot) = roster.get(sel.pilot_id)
                {
                    (pilot.skill.clone(), pilot.personality.clone())
                } else {
                    default_pilot_data()
                }
            } else {
                default_pilot_data()
            };

            DroneScriptInput {
                spline: &ai.spline,
                config,
                skill,
                traits,
            }
        })
        .collect();

    let race_script = script::generate_race_script(
        gate_count,
        gate_positions,
        &inputs,
        seed_res.0,
        &tuning,
    );

    info!(
        "Generated race script: {} drones, {} overtakes, {} crashes",
        race_script.drone_scripts.len(),
        race_script.overtakes.len(),
        race_script
            .drone_scripts
            .iter()
            .filter(|ds| ds.crash.is_some())
            .count(),
    );

    commands.insert_resource(race_script);
    commands.insert_resource(RaceEventLog::default());
}

fn default_pilot_data() -> (crate::pilot::skill::SkillProfile, Vec<crate::pilot::personality::PersonalityTrait>) {
    (
        crate::pilot::skill::SkillProfile {
            level: 0.5,
            speed: 0.5,
            cornering: 0.5,
            consistency: 0.5,
        },
        Vec::new(),
    )
}

/// When the first drone (winner) finishes, starts the results transition timer
/// and locks the camera in place so it doesn't jump around.
pub fn check_winner_finished(
    phase: Res<RacePhase>,
    progress: Option<Res<RaceProgress>>,
    existing_timer: Option<Res<ResultsTransitionTimer>>,
    mut commands: Commands,
    mut camera_state: ResMut<CameraState>,
) {
    if *phase != RacePhase::Racing || existing_timer.is_some() {
        return;
    }
    let Some(progress) = progress else { return };
    if progress.any_finished() {
        commands.insert_resource(ResultsTransitionTimer { remaining: 3.0 });
        camera_state.locked = true;
        info!("Winner crossed the finish line! Results in 3 seconds.");
    }
}

/// Transitions from Racing → Finished when every drone has finished or crashed.
/// Unlocks the camera and switches to chase mode to follow the winner.
pub fn check_race_finished(
    mut phase: ResMut<RacePhase>,
    progress: Option<Res<RaceProgress>>,
    mut clock: Option<ResMut<RaceClock>>,
    mut camera_state: ResMut<CameraState>,
) {
    if *phase != RacePhase::Racing {
        return;
    }
    let Some(progress) = progress else { return };
    if progress.drone_states.is_empty() {
        return;
    }

    let all_done = progress
        .drone_states
        .iter()
        .all(|s| s.finished || s.crashed);
    if all_done {
        *phase = RacePhase::Finished;
        if let Some(ref mut clock) = clock {
            clock.running = false;
        }
        // Unlock camera and follow the winner
        camera_state.locked = false;
        camera_state.mode = CameraMode::Chase;
        info!("Race finished! All drones completed or crashed.");
    }
}

/// Ticks the results transition timer. When it expires, transitions to Results.
/// Race resources (RaceProgress, RaceClock, RaceScript) stay alive so drones
/// can keep finishing and the results UI updates live.
pub fn tick_results_transition(
    time: Res<Time>,
    mut timer: Option<ResMut<ResultsTransitionTimer>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(ref mut timer) = timer else { return };

    timer.remaining -= time.delta_secs();
    if timer.remaining > 0.0 {
        return;
    }

    commands.remove_resource::<ResultsTransitionTimer>();
    next_state.set(AppState::Results);
    info!("Transitioning to Results screen.");
}
