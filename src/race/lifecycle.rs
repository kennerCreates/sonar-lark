use bevy::prelude::*;

use crate::camera::switching::{CameraMode, CameraState, CourseCameras};
use crate::course::loader::SelectedCourse;
use crate::drone::components::{AIController, Drone, DronePhase};
use crate::states::AppState;

use super::progress::{DroneRaceState, RaceProgress};
use super::timing::RaceClock;

#[derive(Resource)]
pub struct RaceStartSound(pub Handle<bevy::audio::AudioSource>);

#[derive(Resource)]
pub struct RaceEndSound(pub Handle<bevy::audio::AudioSource>);

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum RacePhase {
    #[default]
    WaitingToStart,
    Countdown,
    Racing,
    Finished,
}

#[derive(Resource)]
pub struct CountdownTimer {
    pub remaining: f32,
}

impl Default for CountdownTimer {
    fn default() -> Self {
        Self { remaining: 3.0 }
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

/// Run condition: returns true when any drone is actively racing, victory-lapping, or returning.
/// Used to keep AI systems running during and after the race.
pub fn drones_are_active(
    phase: Option<Res<RacePhase>>,
    drones: Query<&DronePhase, With<Drone>>,
) -> bool {
    if phase.is_some_and(|p| *p == RacePhase::Racing) {
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
) {
    if *phase != RacePhase::Countdown {
        return;
    }
    let Some(ref mut timer) = timer else { return };

    timer.remaining -= time.delta_secs();

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

/// Transitions from Racing → Finished when every drone has finished or crashed.
/// Also starts the auto-transition timer to the Results screen.
pub fn check_race_finished(
    mut phase: ResMut<RacePhase>,
    progress: Option<Res<RaceProgress>>,
    mut clock: Option<ResMut<RaceClock>>,
    mut commands: Commands,
    mut camera_state: ResMut<CameraState>,
    course_cameras: Option<Res<CourseCameras>>,
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
        commands.insert_resource(ResultsTransitionTimer { remaining: 3.0 });
        // Switch to primary course camera if available
        if course_cameras.is_some_and(|cc| !cc.cameras.is_empty()) {
            camera_state.mode = CameraMode::CourseCamera(0);
        }
        info!("Race finished! All drones completed or crashed.");
    }
}

/// Ticks the results transition timer. When it expires, snapshots race results
/// and transitions to the Results state.
pub fn tick_results_transition(
    time: Res<Time>,
    mut timer: Option<ResMut<ResultsTransitionTimer>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    progress: Option<Res<RaceProgress>>,
    clock: Option<Res<RaceClock>>,
    selected_course: Option<Res<SelectedCourse>>,
) {
    let Some(ref mut timer) = timer else { return };

    timer.remaining -= time.delta_secs();
    if timer.remaining > 0.0 {
        return;
    }

    // Build RaceResults snapshot before leaving Race state
    if let Some(progress) = progress {
        let total_time = clock.map(|c| c.elapsed).unwrap_or(0.0);
        let course_name = selected_course
            .map(|s| {
                std::path::Path::new(&s.path)
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .trim_end_matches(".course")
                    .to_string()
            })
            .unwrap_or_else(|| "Unknown".to_string());
        commands.insert_resource(progress.to_race_results(total_time, course_name));
    }

    commands.remove_resource::<ResultsTransitionTimer>();
    next_state.set(AppState::Results);
    info!("Transitioning to Results screen.");
}
