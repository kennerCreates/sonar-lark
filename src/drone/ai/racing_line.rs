use bevy::prelude::*;

use super::super::components::*;
use super::super::maneuver::ManeuverTrajectory;
use super::{
    FINISH_EPSILON, VELOCITY_LOOK_AHEAD_T,
    cyclic_curvature, cyclic_pos, cyclic_vel, max_curvature_ahead,
    safe_speed_for_curvature_with,
};
use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};

/// How far ahead (in spline parameter units) gate correction begins.
/// 2.0 = starts from the previous gate's midleg waypoint.
const GATE_CORRECTION_RANGE: f32 = 2.0;

/// Maximum blend factor toward the gate position (0.0–1.0).
/// 0.7 = at the gate midpoint, 70% gate target / 30% spline look-ahead.
const GATE_CORRECTION_STRENGTH: f32 = 0.7;

pub fn compute_racing_line(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    mut query: Query<(
        &Transform,
        &AIController,
        &DroneConfig,
        &DronePhase,
        &mut DesiredPosition,
        Option<&ManeuverTrajectory>,
    )>,
) {
    let elapsed = time.elapsed_secs();

    for (transform, ai, config, phase, mut desired, maneuver_traj) in &mut query {
        // When a maneuver trajectory is active, sample from it instead of the racing spline
        if let Some(traj) = maneuver_traj {
            let progress = ((elapsed - traj.start_time) / traj.duration).clamp(0.0, 1.0);
            let curve_t = progress * traj.curve_len;

            // Small look-ahead for smoother tracking
            let look_ahead_t = ((progress + 0.05) * traj.curve_len).min(traj.curve_len);
            desired.position = traj.curve.position(look_ahead_t);
            desired.velocity_hint = traj.curve.velocity(curve_t).normalize_or(Vec3::NEG_Z);
            desired.max_speed = traj.entry_speed;
            continue;
        }

        match *phase {
            DronePhase::Idle | DronePhase::Crashed | DronePhase::Wandering => continue,
            DronePhase::Racing => {
                let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
                let finish_t = cycle_t + FINISH_EXTENSION;

                if ai.spline_t >= finish_t + FINISH_EPSILON {
                    desired.position = transform.translation;
                    desired.velocity_hint = Vec3::ZERO;
                    desired.max_speed = tuning.max_speed;
                    continue;
                }

                // Curvature at the current spline position for adaptive look-ahead.
                let cur_curvature = cyclic_curvature(&ai.spline, ai.spline_t, cycle_t);
                let curvature_factor =
                    1.0 / (1.0 + cur_curvature * tuning.curvature_look_ahead_scale);
                let look_ahead_clamp =
                    curvature_factor.clamp(tuning.min_look_ahead_fraction, 1.0);
                let adaptive_look_ahead = tuning.look_ahead_t * look_ahead_clamp;
                let adaptive_vel_look_ahead = VELOCITY_LOOK_AHEAD_T * look_ahead_clamp;

                // Sample ahead on the cyclic spline, clamped to the finish line.
                let target_t =
                    (ai.spline_t + adaptive_look_ahead * POINTS_PER_GATE).min(finish_t);
                let target_pos = cyclic_pos(&ai.spline, target_t, cycle_t);

                let vel_t =
                    (ai.spline_t + adaptive_vel_look_ahead * POINTS_PER_GATE).min(finish_t);
                let tangent = cyclic_vel(&ai.spline, vel_t, cycle_t).normalize_or(Vec3::NEG_Z);

                // Gate correction: blend target toward the gate when approaching.
                // Simulates a pilot actively aiming for the gate opening.
                let next_gate_idx =
                    ((ai.spline_t - 0.5) / POINTS_PER_GATE + 1.0).floor() as usize;
                let gate_blend = if next_gate_idx < ai.gate_positions.len() {
                    let gate_t = next_gate_idx as f32 * POINTS_PER_GATE + 0.5;
                    let dist_t = gate_t - ai.spline_t;
                    if dist_t > 0.0 && dist_t < GATE_CORRECTION_RANGE {
                        let t = 1.0 - dist_t / GATE_CORRECTION_RANGE;
                        t * t * GATE_CORRECTION_STRENGTH
                    } else {
                        0.0
                    }
                } else if next_gate_idx == ai.gate_positions.len() {
                    // Finish gate = gate 0 again
                    let gate_t = cycle_t + 0.5;
                    let dist_t = gate_t - ai.spline_t;
                    if dist_t > 0.0 && dist_t < GATE_CORRECTION_RANGE {
                        let t = 1.0 - dist_t / GATE_CORRECTION_RANGE;
                        t * t * GATE_CORRECTION_STRENGTH
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let gate_world_idx = if next_gate_idx >= ai.gate_positions.len() {
                    0
                } else {
                    next_gate_idx
                };
                let corrected_target = if gate_blend > 0.0 {
                    let gate_pos = ai.gate_positions[gate_world_idx];
                    target_pos.lerp(gate_pos, gate_blend)
                } else {
                    target_pos
                };

                // Blend velocity hint toward gate direction so the PID steers toward it
                let corrected_tangent = if gate_blend > 0.0 {
                    let gate_pos = ai.gate_positions[gate_world_idx];
                    let to_gate = (gate_pos - transform.translation).normalize_or(tangent);
                    tangent.lerp(to_gate, gate_blend)
                } else {
                    tangent
                };

                // Small organic wobble, suppressed near gates
                let lateral = Vec3::Y.cross(corrected_tangent).normalize_or(Vec3::X);
                let wobble_suppress = 1.0 - gate_blend / GATE_CORRECTION_STRENGTH;
                let noise = (elapsed * config.noise_frequency + config.line_offset * std::f32::consts::PI).sin()
                    * config.noise_amplitude * 0.3 * wobble_suppress;
                let offset = lateral * noise;

                desired.position = corrected_target + offset;
                desired.velocity_hint = corrected_tangent;

                // Curvature-aware speed limit: scan ahead for the tightest upcoming turn.
                // Per-drone variation: aggressive drones carry more speed, cautious drones brake earlier.
                let per_drone_range = tuning.speed_curvature_range * config.braking_distance;
                let max_k = max_curvature_ahead(
                    &ai.spline,
                    ai.spline_t,
                    per_drone_range,
                    cycle_t,
                );
                let per_drone_accel = tuning.safe_lateral_accel * config.cornering_aggression;
                desired.max_speed = safe_speed_for_curvature_with(max_k, per_drone_accel, &tuning);
            }
            DronePhase::VictoryLap => {
                let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;

                let cur_curvature = cyclic_curvature(&ai.spline, ai.spline_t, cycle_t);
                let curvature_factor =
                    1.0 / (1.0 + cur_curvature * tuning.curvature_look_ahead_scale);
                let look_ahead_clamp =
                    curvature_factor.clamp(tuning.min_look_ahead_fraction, 1.0);
                let adaptive_look_ahead = tuning.look_ahead_t * look_ahead_clamp;
                let adaptive_vel_look_ahead = VELOCITY_LOOK_AHEAD_T * look_ahead_clamp;

                // Pure cyclic sampling — no finish_t clamp
                let target_t = ai.spline_t + adaptive_look_ahead * POINTS_PER_GATE;
                let target_pos = cyclic_pos(&ai.spline, target_t, cycle_t);

                let vel_t = ai.spline_t + adaptive_vel_look_ahead * POINTS_PER_GATE;
                let tangent = cyclic_vel(&ai.spline, vel_t, cycle_t).normalize_or(Vec3::NEG_Z);

                let lateral = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
                let noise = (elapsed * config.noise_frequency + config.line_offset * std::f32::consts::PI).sin()
                    * config.noise_amplitude * 0.3;
                let offset = lateral * noise;

                desired.position = target_pos + offset;
                desired.velocity_hint = tangent;

                let per_drone_range = tuning.speed_curvature_range * config.braking_distance;
                let max_k = max_curvature_ahead(
                    &ai.spline,
                    ai.spline_t,
                    per_drone_range,
                    cycle_t,
                );
                let per_drone_accel = tuning.safe_lateral_accel * config.cornering_aggression;
                desired.max_speed = safe_speed_for_curvature_with(max_k, per_drone_accel, &tuning);
            }
        }
    }
}
