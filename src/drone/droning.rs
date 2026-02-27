use bevy::audio::{AudioSink, AudioSinkPlayback, Volume};
use bevy::prelude::*;
use rand::Rng;

use crate::drone::components::{Drone, DronePhase};
use crate::race::lifecycle::RacePhase;
use crate::states::AppState;

const DRONING_TRACK_COUNT: usize = 4;
const TOTAL_DRONES: f32 = 12.0;

/// Duration of each droning track in seconds (all 4 tracks are identical length).
const TRACK_DURATION: f32 = 10.305;

/// Overlap: next track starts this many seconds before the current one ends,
/// so the outgoing fade-out blends with the incoming fade-in.
const OVERLAP_SECS: f32 = 2.0;

/// Seconds between successive track starts.
const ADVANCE_INTERVAL: f32 = TRACK_DURATION - OVERLAP_SECS;

/// Base volume when all 12 drones are active. Scaled down proportionally as drones crash.
const BASE_VOLUME: f32 = 0.3;

/// Minimum volume floor (so the sound doesn't vanish entirely with few drones left).
const MIN_VOLUME: f32 = 0.05;

/// Marker component for droning audio entities so we can adjust their volume at runtime.
#[derive(Component)]
pub struct DroningTrack;

#[derive(Resource)]
pub struct DroningSounds(pub Vec<Handle<bevy::audio::AudioSource>>);

#[derive(Resource)]
pub struct DroningState {
    /// Seconds remaining until the next track should start.
    time_until_next: f32,
    /// Current position in the shuffle order.
    shuffle_cursor: usize,
    /// Shuffled track indices for variety without immediate repeats.
    shuffle_order: Vec<usize>,
    /// Whether droning has started (waits for countdown to finish).
    started: bool,
}

pub fn load_droning_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handles: Vec<Handle<bevy::audio::AudioSource>> = (1..=DRONING_TRACK_COUNT)
        .map(|i| asset_server.load(format!("sounds/droning/drone_droning_{i}.wav")))
        .collect();
    commands.insert_resource(DroningSounds(handles));

    let mut order: Vec<usize> = (0..DRONING_TRACK_COUNT).collect();
    let mut rng = rand::thread_rng();
    for i in (1..order.len()).rev() {
        let j = rng.gen_range(0..=i);
        order.swap(i, j);
    }

    commands.insert_resource(DroningState {
        time_until_next: 0.0,
        shuffle_cursor: 0,
        shuffle_order: order,
        started: false,
    });
}

/// Manages overlapping ambient droning tracks. Each track has a baked-in
/// fade-in/fade-out, so we start the next track `OVERLAP_SECS` before the
/// current one ends, producing a smooth crossfade.
///
/// Waits for countdown to finish before starting. Volume scales with the
/// number of active (non-crashed) drones.
pub fn update_droning(
    mut commands: Commands,
    time: Res<Time>,
    sounds: Option<Res<DroningSounds>>,
    mut state: Option<ResMut<DroningState>>,
    phase: Option<Res<RacePhase>>,
    drones: Query<&DronePhase, With<Drone>>,
    mut droning_sinks: Query<&mut AudioSink, With<DroningTrack>>,
) {
    let (Some(sounds), Some(ref mut state)) = (sounds, state.as_mut()) else {
        return;
    };

    // Don't start until the countdown finishes and racing begins.
    if !state.started {
        let dominated = phase.is_some_and(|p| matches!(*p, RacePhase::Racing | RacePhase::Finished));
        if !dominated {
            return;
        }
        state.started = true;
    }

    // Count active (non-crashed) drones for volume scaling.
    let active = drones.iter().filter(|p| **p != DronePhase::Crashed).count() as f32;
    let target_volume = (BASE_VOLUME * (active / TOTAL_DRONES)).max(MIN_VOLUME);

    // Update volume on all currently-playing droning tracks.
    for mut sink in &mut droning_sinks {
        sink.set_volume(Volume::Linear(target_volume));
    }

    // Advance timer and spawn next track when ready.
    state.time_until_next -= time.delta_secs();
    if state.time_until_next > 0.0 {
        return;
    }

    let track_idx = state.shuffle_order[state.shuffle_cursor % DRONING_TRACK_COUNT];

    commands.spawn((
        AudioPlayer::new(sounds.0[track_idx].clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(target_volume)),
        DroningTrack,
        DespawnOnExit(AppState::Results),
    ));

    state.shuffle_cursor += 1;

    // Reshuffle when we've cycled through all tracks.
    if state.shuffle_cursor % DRONING_TRACK_COUNT == 0 {
        let last_played = track_idx;
        let mut rng = rand::thread_rng();
        for i in (1..state.shuffle_order.len()).rev() {
            let j = rng.gen_range(0..=i);
            state.shuffle_order.swap(i, j);
        }
        if state.shuffle_order[0] == last_played && DRONING_TRACK_COUNT > 1 {
            state.shuffle_order.swap(0, 1);
        }
    }

    state.time_until_next = ADVANCE_INTERVAL;
}

pub fn cleanup_droning(mut commands: Commands) {
    commands.remove_resource::<DroningSounds>();
    commands.remove_resource::<DroningState>();
}
