use bevy::prelude::*;

use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
use crate::drone::ai::{cyclic_curvature, cyclic_vel, safe_speed_for_curvature};
use crate::drone::explosion::{CrashSounds, ExplosionMeshes};
use crate::race::collision::crash_drone;
use crate::race::progress::{DnfReason, RaceProgress};
use crate::race::script::{
    RaceEventKind, RaceEventLog, RaceScript, ScriptedCrashType, TimestampedEvent,
};
use crate::race::timing::RaceClock;

use crate::pilot::roster::PilotRoster;
use crate::pilot::SelectedPilots;
use crate::race::lifecycle::CountdownTimer;

use super::components::*;
use super::wander::WanderBounds;

const SKILL_JITTER_SCALE: f32 = 3.0;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_BANK_ANGLE: f32 = 1.2; // ~69 degrees
const GROUND_HEIGHT: f32 = 0.3;
/// Lateral deflection speed (m/s) added on crash for visible veer.
const CRASH_DEFLECTION_SPEED: f32 = 3.0;

// Phase 5: Acrobatic constants
/// How far before the gate (in spline_t units) the acrobatic window starts.
const ACRO_ENTRY_OFFSET: f32 = 0.4 * POINTS_PER_GATE;
/// How far after the gate (in spline_t units) the acrobatic window ends.
const ACRO_EXIT_OFFSET: f32 = 0.4 * POINTS_PER_GATE;
/// Base altitude dip for Split-S (meters).
const SPLIT_S_DIP: f32 = 3.0;
/// Base altitude climb for Power Loop (meters).
const POWER_LOOP_CLIMB: f32 = 3.5;
/// Lateral drift during acrobatics (meters).
const ACRO_LATERAL_DRIFT: f32 = 1.0;

// Phase 6: Visual noise constants
const BASE_JITTER_AMP: f32 = 0.015;
const JITTER_FREQ_1: f32 = 7.3;
const JITTER_FREQ_2: f32 = 11.7;
const JITTER_FREQ_3: f32 = 5.1;
const DIRTY_AIR_RANGE: f32 = 8.0;
const DIRTY_AIR_MULT: f32 = 3.0;
const MICRO_DRIFT_AMP: f32 = 0.01;

// ---------------------------------------------------------------------------
// System: advance_choreography
// ---------------------------------------------------------------------------

/// Advances Racing drones along their splines using the pre-computed pace profiles.
/// Also handles ballistic arcs for crashed drones and acrobatic position offsets.
/// Writes Transform.translation, DroneDynamics.velocity, AIController.spline_t.
#[allow(clippy::too_many_arguments)]
pub fn advance_choreography(
    mut commands: Commands,
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    script: Option<Res<RaceScript>>,
    mut progress: Option<ResMut<RaceProgress>>,
    _event_log: Option<ResMut<RaceEventLog>>,
    explosion_meshes: Option<Res<ExplosionMeshes>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    crash_sounds: Option<Res<CrashSounds>>,
    mut racing_query: Query<(
        Entity,
        &Drone,
        &DroneIdentity,
        &mut DronePhase,
        &mut AIController,
        &mut DroneDynamics,
        &mut Transform,
        &mut Visibility,
        &mut ChoreographyState,
    ), Without<BallisticState>>,
    mut ballistic_query: Query<(
        Entity,
        &Drone,
        &DroneIdentity,
        &mut DronePhase,
        &mut DroneDynamics,
        &mut Transform,
        &mut Visibility,
        &mut BallisticState,
    ), Without<ChoreographyState>>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    // --- Ballistic arc update (Phase 4): crashed drones falling under gravity ---
    // Runs independently of RaceScript so drones mid-crash-arc continue falling
    // even after the script is cleaned up (e.g., during Results transition).
    for (entity, drone, identity, mut phase, mut dynamics, mut transform, mut visibility, mut ballistic) in &mut ballistic_query {
        if *phase != DronePhase::Racing {
            continue;
        }
        ballistic.velocity.y -= GRAVITY * dt;
        transform.translation += ballistic.velocity * dt;
        dynamics.velocity = ballistic.velocity;

        if transform.translation.y <= GROUND_HEIGHT {
            transform.translation.y = GROUND_HEIGHT;
            if let Some(ref meshes) = explosion_meshes {
                crash_drone(
                    &mut commands,
                    &mut phase,
                    &mut dynamics,
                    &mut visibility,
                    drone.index as usize,
                    transform.translation,
                    ballistic.velocity,
                    progress.as_deref_mut(),
                    meshes,
                    &mut materials,
                    crash_sounds.as_deref(),
                    identity.color,
                    DnfReason::ObstacleCollision,
                );
            }
            commands.entity(entity).remove::<BallisticState>();
        }
    }

    // --- Racing spline advancement (requires RaceScript) ---
    let Some(script) = script else { return };

    for (_entity, drone, _identity, phase, mut ai, mut dynamics, mut transform, _visibility, mut choreo) in &mut racing_query {
        if *phase != DronePhase::Racing {
            continue;
        }

        let d_idx = drone.index as usize;
        let Some(ds) = script.drone_scripts.get(d_idx) else {
            continue;
        };

        let gate_count = ai.gate_count;
        let cycle_t = gate_count as f32 * POINTS_PER_GATE;

        // Save previous spline_t for crossing detection
        choreo.previous_spline_t = ai.spline_t;

        // Phase 4: Check if this drone has reached its crash point
        if let Some(ref crash) = ds.crash {
            let crash_gate_t = ds.gate_pass_t.get(crash.gate_index as usize).copied().unwrap_or(0.0);
            let crash_t = crash_gate_t + crash.progress_past_gate * POINTS_PER_GATE;
            if ai.spline_t >= crash_t {
                // Already past crash point — insert BallisticState if not already present
                // (fire_scripted_events handles the actual crash initiation on crossing)
                continue;
            }
        }

        // Determine current segment and pace
        let seg_idx = ((ai.spline_t / POINTS_PER_GATE).floor() as usize)
            .min(ds.segment_pace.len().saturating_sub(1));
        let pace = ds.segment_pace[seg_idx];

        // Compute speed from curvature
        let curvature = cyclic_curvature(&ai.spline, ai.spline_t, cycle_t);
        let base_speed = safe_speed_for_curvature(curvature, &tuning);
        let speed = base_speed * pace;

        // Advance spline_t
        let tangent = ai.spline.velocity(ai.spline_t.rem_euclid(cycle_t));
        let tangent_len = tangent.length().max(0.01);
        let dt_spline = speed * dt / tangent_len;
        ai.spline_t += dt_spline;

        // Update base position from spline
        let mut new_pos = ai.spline.position(ai.spline_t.rem_euclid(cycle_t));
        let tangent_dir = tangent / tangent_len;
        let mut vel = tangent_dir * speed;

        // Phase 5: Acrobatic position offset
        if let Some((acro_type, t_local, _gate_idx)) = active_acrobatic(ai.spline_t, ds, &ai) {
            let sin_t = (t_local * std::f32::consts::PI).sin();
            let cos_t = (t_local * std::f32::consts::PI).cos();
            let speed_scale = speed / tuning.max_speed;

            let entry_t = acro_entry_exit(ds, &ai).map(|(e, _)| e).unwrap_or(0.0);
            let exit_t = acro_entry_exit(ds, &ai).map(|(_, x)| x).unwrap_or(1.0);
            let window_len = (exit_t - entry_t).max(0.01);
            let spline_t_per_sec = speed / tangent_len;
            let dt_local_per_sec = spline_t_per_sec / window_len;

            match acro_type {
                AcroType::SplitS => {
                    let dip = SPLIT_S_DIP * speed_scale * sin_t;
                    new_pos.y -= dip;
                    vel.y -= SPLIT_S_DIP * speed_scale * std::f32::consts::PI * cos_t * dt_local_per_sec;
                }
                AcroType::PowerLoop => {
                    let climb = POWER_LOOP_CLIMB * speed_scale * sin_t;
                    new_pos.y += climb;
                    vel.y += POWER_LOOP_CLIMB * speed_scale * std::f32::consts::PI * cos_t * dt_local_per_sec;
                }
            }

            // Lateral drift
            let lateral_dir = Vec3::Y.cross(tangent_dir).normalize_or(Vec3::X);
            let roll_sign = if drone.index % 2 == 0 { 1.0 } else { -1.0 };
            let drift = ACRO_LATERAL_DRIFT * sin_t * roll_sign;
            new_pos += lateral_dir * drift;
            vel += lateral_dir * (ACRO_LATERAL_DRIFT * std::f32::consts::PI * cos_t * dt_local_per_sec * roll_sign);
        }

        transform.translation = new_pos;
        dynamics.velocity = vel;

        // Phase 6: Position micro-drift
        let phase_offset = drone.index as f32 * 1.618;
        let t_secs = time.elapsed_secs();
        transform.translation.x += (t_secs * 3.1 + phase_offset).sin() * MICRO_DRIFT_AMP;
        transform.translation.z += (t_secs * 4.7 + phase_offset).sin() * MICRO_DRIFT_AMP;

        // Update target_gate_index for leaderboard
        ai.target_gate_index = (ai.spline_t / POINTS_PER_GATE).floor() as u32;
        ai.target_gate_index = ai.target_gate_index.min(gate_count.saturating_sub(1));
    }
}

// ---------------------------------------------------------------------------
// System: compute_choreographed_rotation
// ---------------------------------------------------------------------------

/// Derives bank angle from spline curvature for Racing drones.
/// Applies acrobatic rotation keyframes (Phase 5) during acrobatic windows.
pub fn compute_choreographed_rotation(
    _tuning: Res<AiTuningParams>,
    script: Option<Res<RaceScript>>,
    mut query: Query<(
        &Drone,
        &DronePhase,
        &AIController,
        &DroneDynamics,
        &mut Transform,
    )>,
) {
    let Some(script) = script else { return };

    for (drone, phase, ai, dynamics, mut transform) in &mut query {
        if *phase != DronePhase::Racing {
            continue;
        }

        let d_idx = drone.index as usize;
        let Some(ds) = script.drone_scripts.get(d_idx) else {
            continue;
        };

        let gate_count = ai.gate_count;
        let cycle_t = gate_count as f32 * POINTS_PER_GATE;
        let t = ai.spline_t;

        // Sample tangent and acceleration for curvature-based banking
        let vel = cyclic_vel(&ai.spline, t, cycle_t);
        let vel_len = vel.length();
        if vel_len < 0.001 {
            continue;
        }
        let tangent = vel / vel_len;

        let accel = ai.spline.acceleration(t.rem_euclid(cycle_t));
        let centripetal = accel - tangent * tangent.dot(accel);

        let speed = dynamics.velocity.length();
        let kappa = centripetal.length() / (vel_len * vel_len);
        let bank_angle = (speed * speed * kappa / GRAVITY)
            .atan()
            .clamp(0.0, MAX_BANK_ANGLE);

        // Bank direction: which way the turn curves
        let left = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
        let bank_sign = centripetal.dot(left).signum();

        // Build normal banking rotation
        let banked_up =
            Quat::from_axis_angle(tangent, -bank_sign * bank_angle) * Vec3::Y;

        let normal_rot = match look_toward(tangent, banked_up) {
            Some(r) => r,
            None => continue,
        };

        // Phase 5: Acrobatic rotation keyframes
        if let Some((acro_type, t_local, _gate_idx)) = active_acrobatic(ai.spline_t, ds, ai) {
            let roll_sign = if drone.index % 2 == 0 { 1.0 } else { -1.0 };

            // Compute exit rotation (normal bank at window exit)
            let exit_rot = normal_rot;
            let entry_rot = normal_rot;

            let acro_rot = acrobatic_rotation(acro_type, t_local, entry_rot, exit_rot, tangent, roll_sign);

            // Blend at entry/exit edges for smooth transition
            let blend = acrobatic_blend(t_local);
            transform.rotation = normal_rot.slerp(acro_rot, blend);
        } else {
            transform.rotation = normal_rot;
        }
    }
}

/// Compute a look rotation toward a direction with an up hint.
/// Bevy convention: local -Z is forward. The Z column of the rotation
/// matrix is set to -forward so that local -Z aligns with `forward`.
/// Returns None if direction is zero-length or parallel to up.
fn look_toward(forward: Vec3, up: Vec3) -> Option<Quat> {
    // Negate: we want local -Z = forward, so Z column = -forward.
    let f = (-forward).normalize_or_zero();
    if f.length_squared() < 0.001 {
        return None;
    }
    let right = up.cross(f).normalize_or_zero();
    if right.length_squared() < 0.001 {
        return None;
    }
    let corrected_up = f.cross(right);
    Some(Quat::from_mat3(&Mat3::from_cols(right, corrected_up, f)))
}

// ---------------------------------------------------------------------------
// Phase 5: Acrobatic helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum AcroType {
    SplitS,
    PowerLoop,
}

/// Determine the acrobatic entry/exit t for the first active acrobatic window.
fn acro_entry_exit(
    ds: &crate::race::script::DroneScript,
    ai: &AIController,
) -> Option<(f32, f32)> {
    for &gate_idx in &ds.acrobatic_gates {
        let gate_t = ds.gate_pass_t.get(gate_idx as usize).copied().unwrap_or(0.0);
        let entry_t = gate_t - ACRO_ENTRY_OFFSET;
        let exit_t = gate_t + ACRO_EXIT_OFFSET;
        if ai.spline_t >= entry_t && ai.spline_t <= exit_t {
            return Some((entry_t, exit_t));
        }
    }
    None
}

/// Check if a spline_t falls within an acrobatic window. Returns the acro type,
/// local progress (0..1), and gate index.
fn active_acrobatic(
    spline_t: f32,
    ds: &crate::race::script::DroneScript,
    ai: &AIController,
) -> Option<(AcroType, f32, u32)> {
    for &gate_idx in &ds.acrobatic_gates {
        let gate_t = ds.gate_pass_t.get(gate_idx as usize).copied().unwrap_or(0.0);
        let entry_t = gate_t - ACRO_ENTRY_OFFSET;
        let exit_t = gate_t + ACRO_EXIT_OFFSET;
        if spline_t >= entry_t && spline_t <= exit_t {
            let t_local = ((spline_t - entry_t) / (exit_t - entry_t)).clamp(0.0, 1.0);
            // Choose maneuver type based on altitude of next gate vs current
            let next_gate = (gate_idx as usize + 1) % ai.gate_positions.len().max(1);
            let current_y = ai.gate_positions.get(gate_idx as usize).map(|p| p.y).unwrap_or(0.0);
            let next_y = ai.gate_positions.get(next_gate).map(|p| p.y).unwrap_or(0.0);
            let acro_type = if next_y > current_y + 2.0 {
                AcroType::PowerLoop
            } else {
                AcroType::SplitS
            };
            return Some((acro_type, t_local, gate_idx));
        }
    }
    None
}

/// Smoothstep blend factor for acrobatic rotation entry/exit.
fn acrobatic_blend(t_local: f32) -> f32 {
    let blend_zone = 0.08;
    if t_local < blend_zone {
        smoothstep(t_local / blend_zone)
    } else if t_local > 1.0 - blend_zone {
        smoothstep((1.0 - t_local) / blend_zone)
    } else {
        1.0
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Compute acrobatic rotation from keyframes using slerp.
fn acrobatic_rotation(
    acro_type: AcroType,
    t_local: f32,
    entry_rot: Quat,
    exit_rot: Quat,
    tangent: Vec3,
    roll_sign: f32,
) -> Quat {
    match acro_type {
        AcroType::SplitS => split_s_rotation(t_local, entry_rot, exit_rot, tangent, roll_sign),
        AcroType::PowerLoop => power_loop_rotation(t_local, entry_rot, exit_rot, tangent, roll_sign),
    }
}

/// Split-S: roll inverted, pull through the bottom.
fn split_s_rotation(t_local: f32, entry: Quat, exit: Quat, tangent: Vec3, roll_sign: f32) -> Quat {
    let half_roll = Quat::from_axis_angle(tangent, roll_sign * std::f32::consts::PI * 0.5);
    let full_roll = Quat::from_axis_angle(tangent, roll_sign * std::f32::consts::PI);
    let right = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
    let nose_down = Quat::from_axis_angle(right, -0.5);

    // Keyframe table: (t, rotation)
    let keyframes: [(f32, Quat); 5] = [
        (0.0, entry),
        (0.15, entry * half_roll),
        (0.35, entry * full_roll),
        (0.55, entry * full_roll * nose_down),
        (1.0, exit),
    ];
    slerp_keyframes(&keyframes, t_local)
}

/// Power loop: pitch up, go over the top inverted, pull through.
fn power_loop_rotation(t_local: f32, entry: Quat, exit: Quat, tangent: Vec3, _roll_sign: f32) -> Quat {
    let right = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
    let pitch_up_45 = Quat::from_axis_angle(right, 0.785);
    let pitch_up_90 = Quat::from_axis_angle(right, 1.57);
    let inverted = Quat::from_axis_angle(tangent, std::f32::consts::PI);

    let keyframes: [(f32, Quat); 5] = [
        (0.0, entry),
        (0.20, entry * pitch_up_45),
        (0.40, entry * pitch_up_90),
        (0.60, entry * inverted),
        (1.0, exit),
    ];
    slerp_keyframes(&keyframes, t_local)
}

/// Slerp between keyframes with smoothstep easing.
fn slerp_keyframes(keyframes: &[(f32, Quat)], t: f32) -> Quat {
    if keyframes.is_empty() {
        return Quat::IDENTITY;
    }
    if t <= keyframes[0].0 {
        return keyframes[0].1;
    }
    for window in keyframes.windows(2) {
        let (t0, q0) = window[0];
        let (t1, q1) = window[1];
        if t >= t0 && t <= t1 {
            let seg_t = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
            let eased = smoothstep(seg_t);
            return q0.slerp(q1, eased);
        }
    }
    keyframes.last().unwrap().1
}

// ---------------------------------------------------------------------------
// System: apply_visual_noise (Phase 6)
// ---------------------------------------------------------------------------

/// Adds attitude jitter, dirty air wobble, and position micro-drift to Racing drones.
/// Jitter amplitude scales with (1 - consistency) so less-skilled pilots wobble more.
pub fn apply_visual_noise(
    time: Res<Time>,
    script: Option<Res<RaceScript>>,
    mut query: Query<(
        &Drone,
        &DronePhase,
        &ChoreographyState,
        &mut Transform,
    ), Without<BallisticState>>,
) {
    let Some(_script) = script else { return };
    let t_secs = time.elapsed_secs();

    // Collect Racing drone positions for dirty air proximity check
    let racing_positions: Vec<(u8, Vec3)> = query
        .iter()
        .filter(|(_, phase, _, _)| **phase == DronePhase::Racing)
        .map(|(drone, _, _, transform)| (drone.index, transform.translation))
        .collect();

    for (drone, phase, choreo, mut transform) in &mut query {
        if *phase != DronePhase::Racing {
            continue;
        }

        let phase_offset = drone.index as f32 * 2.39996;
        let base_amp = BASE_JITTER_AMP * (1.0 + (1.0 - choreo.consistency) * SKILL_JITTER_SCALE);

        // Dirty air: increase jitter when near another drone
        let mut dirty_air_factor: f32 = 1.0;
        for &(other_idx, other_pos) in &racing_positions {
            if other_idx == drone.index {
                continue;
            }
            let dist = (transform.translation - other_pos).length();
            if dist < DIRTY_AIR_RANGE {
                let proximity = 1.0 - (dist / DIRTY_AIR_RANGE);
                dirty_air_factor = dirty_air_factor.max(1.0 + proximity * DIRTY_AIR_MULT);
            }
        }

        let amp = base_amp * dirty_air_factor;
        let jitter = Vec3::new(
            (t_secs * JITTER_FREQ_1 + phase_offset).sin() * amp,
            (t_secs * JITTER_FREQ_2 + phase_offset).sin() * amp * 0.3,
            (t_secs * JITTER_FREQ_3 + phase_offset).sin() * amp,
        );
        transform.rotation *= Quat::from_euler(EulerRot::XYZ, jitter.x, jitter.y, jitter.z);
    }
}

// ---------------------------------------------------------------------------
// System: set_convergence_targets
// ---------------------------------------------------------------------------

/// When countdown starts, sets each drone's DesiredPosition to its spline start position.
/// The existing wander/physics chain naturally flies them there.
/// Also checks convergence: if all drones are within 2m of start and remaining > 3.0,
/// reduces the countdown to 3.0 to start the visible 3-2-1.
pub fn set_convergence_targets(
    phase: Res<crate::race::lifecycle::RacePhase>,
    mut timer: Option<ResMut<CountdownTimer>>,
    mut query: Query<(&AIController, &Transform, &mut DesiredPosition, &DronePhase), With<Drone>>,
) {
    if *phase != crate::race::lifecycle::RacePhase::Countdown {
        return;
    }

    let mut all_converged = true;
    let mut any_idle = false;

    for (ai, transform, mut desired, drone_phase) in &mut query {
        if *drone_phase != DronePhase::Idle {
            continue;
        }
        any_idle = true;
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
        let start_pos = ai.spline.position(0.0_f32.rem_euclid(cycle_t.max(0.01)));
        desired.position = start_pos;
        desired.velocity_hint = Vec3::ZERO;
        desired.max_speed = 15.0;

        if (transform.translation - start_pos).length() > 2.0 {
            all_converged = false;
        }
    }

    // Once all drones are within 2m of start, skip ahead to the 3-2-1 countdown
    if any_idle
        && all_converged
        && let Some(ref mut timer) = timer
        && timer.remaining > 3.0
    {
        timer.remaining = 3.0;
    }
}

// ---------------------------------------------------------------------------
// System: snap_to_start_positions
// ---------------------------------------------------------------------------

/// On the frame when Racing begins, snap drones to exact spline start positions
/// and insert ChoreographyState with pilot consistency cached for jitter scaling.
pub fn snap_to_start_positions(
    mut commands: Commands,
    phase: Res<crate::race::lifecycle::RacePhase>,
    selected_pilots: Option<Res<SelectedPilots>>,
    roster: Option<Res<PilotRoster>>,
    mut query: Query<
        (Entity, &Drone, &AIController, &mut Transform, &DronePhase),
        (With<Drone>, Without<ChoreographyState>),
    >,
) {
    if *phase != crate::race::lifecycle::RacePhase::Racing {
        return;
    }

    for (entity, drone, ai, mut transform, drone_phase) in &mut query {
        if *drone_phase != DronePhase::Racing {
            continue;
        }
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
        let start_pos = ai.spline.position(0.0_f32.rem_euclid(cycle_t.max(0.01)));
        transform.translation = start_pos;

        let consistency = selected_pilots
            .as_ref()
            .and_then(|sel| sel.pilots.get(drone.index as usize))
            .and_then(|sel| roster.as_ref().and_then(|r| r.get(sel.pilot_id)))
            .map(|pilot| pilot.skill.consistency)
            .unwrap_or(0.5);

        commands.entity(entity).insert(ChoreographyState {
            previous_spline_t: 0.0,
            consistency,
        });
    }
}

// ---------------------------------------------------------------------------
// System: fire_scripted_events
// ---------------------------------------------------------------------------

/// Detects gate passes, overtakes, crashes, and finishes from spline_t thresholds.
/// Replaces gate_trigger_check + miss_detection for choreographed drones.
#[allow(clippy::too_many_arguments)]
pub fn fire_scripted_events(
    mut commands: Commands,
    script: Option<Res<RaceScript>>,
    clock: Option<Res<RaceClock>>,
    mut progress: Option<ResMut<RaceProgress>>,
    mut event_log: Option<ResMut<RaceEventLog>>,
    bounds: Option<Res<WanderBounds>>,
    mut query: Query<(
        Entity,
        &Drone,
        &mut DronePhase,
        &AIController,
        &ChoreographyState,
        &Transform,
        &mut DroneDynamics,
        &mut PositionPid,
        &mut DesiredPosition,
        &mut DesiredAcceleration,
        &mut DesiredAttitude,
    )>,
) {
    let Some(script) = script else { return };
    let race_time = clock.as_ref().map(|c| c.elapsed).unwrap_or(0.0);

    for (
        entity,
        drone,
        mut phase,
        ai,
        choreo,
        transform,
        mut dynamics,
        mut pid,
        mut desired_pos,
        mut desired_accel,
        mut desired_att,
    ) in &mut query
    {
        if *phase != DronePhase::Racing {
            continue;
        }

        let d_idx = drone.index as usize;
        let Some(ds) = script.drone_scripts.get(d_idx) else {
            continue;
        };

        let gate_count = ai.gate_count;
        let cycle_t = gate_count as f32 * POINTS_PER_GATE;
        let finish_t = cycle_t + FINISH_EXTENSION + 0.01; // FINISH_EPSILON

        let prev_t = choreo.previous_spline_t;
        let curr_t = ai.spline_t;

        // Gate pass detection: check each gate's per-drone threshold
        for (g_idx, &threshold) in ds.gate_pass_t.iter().enumerate() {
            if prev_t < threshold && curr_t >= threshold {
                if let Some(ref mut progress) = progress {
                    progress.record_gate_pass(d_idx, g_idx as u32);
                }
                if let Some(ref mut log) = event_log {
                    log.events.push(TimestampedEvent {
                        race_time,
                        kind: RaceEventKind::GatePass {
                            drone_idx: drone.index,
                            gate_index: g_idx as u32,
                        },
                    });
                }

                // Phase 5: Acrobatic event on gate entry
                if ds.acrobatic_gates.contains(&(g_idx as u32))
                    && let Some(ref mut log) = event_log
                {
                    log.events.push(TimestampedEvent {
                        race_time,
                        kind: RaceEventKind::Acrobatic {
                            drone_idx: drone.index,
                            gate_index: g_idx as u32,
                        },
                    });
                }
            }
        }

        // Overtake detection: check script's overtakes
        for overtake in &script.overtakes {
            if overtake.overtaker_idx == drone.index {
                let gate_t = ds
                    .gate_pass_t
                    .get(overtake.gate_index as usize)
                    .copied()
                    .unwrap_or(0.0);
                if prev_t < gate_t
                    && curr_t >= gate_t
                    && let Some(ref mut log) = event_log
                {
                    log.events.push(TimestampedEvent {
                        race_time,
                        kind: RaceEventKind::Overtake {
                            overtaker_idx: overtake.overtaker_idx,
                            overtaken_idx: overtake.overtaken_idx,
                            gate_index: overtake.gate_index,
                        },
                    });
                }
            }
        }

        // Phase 4: Crash detection — when spline_t crosses the crash point
        if let Some(ref crash) = ds.crash {
            let crash_gate_t = ds.gate_pass_t.get(crash.gate_index as usize).copied().unwrap_or(0.0);
            let crash_t = crash_gate_t + crash.progress_past_gate * POINTS_PER_GATE;
            if prev_t < crash_t && curr_t >= crash_t {
                // Compute crash velocity: current velocity + lateral deflection
                let tangent = ai.spline.velocity(curr_t.rem_euclid(cycle_t));
                let tangent_len = tangent.length().max(0.01);
                let tangent_dir = tangent / tangent_len;
                let speed = dynamics.velocity.length();
                let lateral = Vec3::Y.cross(tangent_dir).normalize_or(Vec3::X);
                let deflection_sign = if drone.index % 2 == 0 { 1.0 } else { -1.0 };
                let crash_velocity = tangent_dir * speed
                    + lateral * (CRASH_DEFLECTION_SPEED * deflection_sign)
                    + Vec3::Y * 2.0; // slight upward kick for visible arc

                commands
                    .entity(entity)
                    .insert(BallisticState { velocity: crash_velocity })
                    .remove::<ChoreographyState>();

                let dnf_reason = match crash.crash_type {
                    ScriptedCrashType::ObstacleCollision => DnfReason::ObstacleCollision,
                    ScriptedCrashType::DroneCollision { .. } => DnfReason::DroneCollision,
                };

                if let Some(ref mut log) = event_log {
                    log.events.push(TimestampedEvent {
                        race_time,
                        kind: RaceEventKind::Crash {
                            drone_idx: drone.index,
                            crash_type: crash.crash_type,
                        },
                    });
                }

                // Record the DNF in progress (crash_drone will also call this on
                // ground impact, but record_crash is idempotent)
                if let Some(ref mut progress) = progress {
                    progress.record_crash(d_idx, dnf_reason);
                }

                continue; // Skip finish detection for crashing drones
            }
        }

        // Finish detection
        if prev_t < finish_t && curr_t >= finish_t {
            if let Some(ref mut progress) = progress {
                progress.record_finish(d_idx, race_time);
            }
            if let Some(ref mut log) = event_log {
                log.events.push(TimestampedEvent {
                    race_time,
                    kind: RaceEventKind::Finish {
                        drone_idx: drone.index,
                        time: race_time,
                    },
                });
            }

            // Transition Racing → Wandering
            *phase = DronePhase::Wandering;
            commands.entity(entity).remove::<ChoreographyState>();
            reset_physics_for_wandering(
                &mut commands,
                entity,
                drone,
                transform,
                &mut dynamics,
                &mut pid,
                &mut desired_pos,
                &mut desired_accel,
                &mut desired_att,
                bounds.as_deref(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// reset_physics_for_wandering
// ---------------------------------------------------------------------------

/// Resets stale physics state when a drone transitions from Racing to Wandering.
/// Called per-entity at the moment of phase transition.
pub fn reset_physics_for_wandering(
    commands: &mut Commands,
    entity: Entity,
    drone: &Drone,
    transform: &Transform,
    dynamics: &mut DroneDynamics,
    pid: &mut PositionPid,
    desired_pos: &mut DesiredPosition,
    desired_accel: &mut DesiredAcceleration,
    desired_att: &mut DesiredAttitude,
    bounds: Option<&WanderBounds>,
) {
    // Velocity: keep as-is from choreography for smooth handoff
    // Angular velocity: zero (stale from choreography)
    dynamics.angular_velocity = Vec3::ZERO;
    // Thrust: set to hover thrust
    let hover_thrust = dynamics.mass * GRAVITY;
    dynamics.thrust = hover_thrust;
    dynamics.commanded_thrust = hover_thrust;
    // PID integral: zero to prevent windup
    pid.integral = Vec3::ZERO;
    // DesiredPosition: current position
    desired_pos.position = transform.translation;
    desired_pos.velocity_hint = Vec3::ZERO;
    desired_pos.max_speed = 8.0; // Wander speed
    // DesiredAcceleration: zero
    desired_accel.acceleration = Vec3::ZERO;
    // DesiredAttitude: hover defaults
    desired_att.orientation = Quat::IDENTITY;
    desired_att.thrust_magnitude = hover_thrust;

    // Insert WanderState
    let target = bounds.map_or(
        transform.translation + Vec3::Y * 5.0,
        |b| {
            // Simple deterministic waypoint
            let hash = (drone.index as u32).wrapping_mul(2654435769);
            let fx = ((hash & 0xFFFF) as f32) / 65535.0;
            let fz = (((hash >> 16) & 0xFFFF) as f32) / 65535.0;
            let fy = ((hash.wrapping_mul(7) & 0xFFFF) as f32) / 65535.0;
            Vec3::new(
                b.min.x + fx * (b.max.x - b.min.x),
                b.min.y + fy * (b.max.y - b.min.y),
                b.min.z + fz * (b.max.z - b.min.z),
            )
        },
    );

    commands.entity(entity).insert(WanderState {
        target,
        dwell_timer: 2.0 + (drone.index % 3) as f32,
        step: 0,
    });
}
