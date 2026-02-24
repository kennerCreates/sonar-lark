use bevy::prelude::*;

use super::components::*;
use super::spawning::generate_return_path;

const WAYPOINT_REACH_DISTANCE: f32 = 5.0;
const LOOK_AHEAD_T: f32 = 0.3;
const VELOCITY_LOOK_AHEAD_T: f32 = 0.5;
const MAX_ADVANCE_PER_TICK: f32 = 0.15;

/// Smoothstep deceleration: 1.0 at start of return → 0.0 at arrival.
fn return_speed_fraction(spline_t: f32, total_t: f32) -> f32 {
    let progress = (spline_t / total_t).clamp(0.0, 1.0);
    let inv = 1.0 - progress;
    inv * inv * (3.0 - 2.0 * inv)
}

pub fn update_ai_targets(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Transform,
        &mut AIController,
        &mut DronePhase,
        &DroneDynamics,
        &DroneStartPosition,
        &DroneConfig,
        &Drone,
        Option<&mut ReturnPath>,
    )>,
) {
    for (entity, transform, mut ai, mut phase, dynamics, start_pos, config, drone, return_path) in
        &mut query
    {
        match *phase {
            DronePhase::Idle => continue,
            DronePhase::Racing => {
                let total_t = ai.gate_count as f32 * POINTS_PER_GATE;
                if ai.spline_t >= total_t {
                    // Transition: Racing → Returning
                    *phase = DronePhase::Returning;
                    if let Some(spline) = generate_return_path(
                        transform.translation,
                        dynamics.velocity,
                        start_pos.translation,
                        config,
                        drone.index,
                    ) {
                        let seg_count = spline.segments().len() as f32;
                        commands.entity(entity).insert(ReturnPath {
                            spline,
                            spline_t: 0.0,
                            total_t: seg_count,
                        });
                    } else {
                        // Fallback: skip directly to Idle if path generation fails
                        *phase = DronePhase::Idle;
                    }
                    continue;
                }

                // Local projection: advance spline_t based on how far the drone has
                // moved along the spline tangent direction.
                let curve_pos = ai.spline.position(ai.spline_t);
                let tangent = ai.spline.velocity(ai.spline_t);
                let tangent_len = tangent.length();
                if tangent_len > 0.001 {
                    let tangent_dir = tangent / tangent_len;
                    let displacement = transform.translation - curve_pos;
                    let forward_proj = displacement.dot(tangent_dir);
                    let advance = (forward_proj / tangent_len)
                        .clamp(0.0, MAX_ADVANCE_PER_TICK * POINTS_PER_GATE);
                    ai.spline_t += advance;
                }

                // Fallback: snap forward if drone is close to the next gate center.
                let next_gate_idx =
                    ((ai.spline_t - 0.5) / POINTS_PER_GATE + 1.0).floor() as usize;
                if next_gate_idx < ai.gate_positions.len() {
                    let next_center_t = next_gate_idx as f32 * POINTS_PER_GATE + 0.5;
                    let dist_to_next =
                        (transform.translation - ai.gate_positions[next_gate_idx]).length();
                    if dist_to_next < WAYPOINT_REACH_DISTANCE {
                        ai.spline_t = ai.spline_t.max(next_center_t);
                    }
                }

                ai.target_gate_index = (ai.spline_t / POINTS_PER_GATE).floor() as u32;
                ai.target_gate_index = ai.target_gate_index.min(ai.gate_count - 1);
            }
            DronePhase::Returning => {
                let Some(mut rp) = return_path else {
                    *phase = DronePhase::Idle;
                    continue;
                };
                if rp.spline_t >= rp.total_t {
                    *phase = DronePhase::Idle;
                    commands.entity(entity).remove::<ReturnPath>();
                    continue;
                }

                let speed_frac = return_speed_fraction(rp.spline_t, rp.total_t);
                let curve_pos = rp.spline.position(rp.spline_t);
                let tangent = rp.spline.velocity(rp.spline_t);
                let tangent_len = tangent.length();
                if tangent_len > 0.001 {
                    let tangent_dir = tangent / tangent_len;
                    let displacement = transform.translation - curve_pos;
                    let forward_proj = displacement.dot(tangent_dir);
                    let max_advance = MAX_ADVANCE_PER_TICK * speed_frac.max(0.05);
                    let advance = (forward_proj / tangent_len).clamp(0.0, max_advance);
                    rp.spline_t += advance;
                }
                rp.spline_t = rp.spline_t.min(rp.total_t);
            }
        }
    }
}

pub fn compute_racing_line(
    time: Res<Time>,
    mut query: Query<(
        &Transform,
        &AIController,
        &DroneConfig,
        &DronePhase,
        Option<&ReturnPath>,
        &mut DesiredPosition,
    )>,
) {
    let elapsed = time.elapsed_secs();

    for (transform, ai, config, phase, return_path, mut desired) in &mut query {
        match *phase {
            DronePhase::Idle => continue,
            DronePhase::Racing => {
                let total_t = ai.gate_count as f32 * POINTS_PER_GATE;
                if ai.spline_t >= total_t {
                    desired.position = transform.translation;
                    desired.velocity_hint = Vec3::ZERO;
                    continue;
                }

                // Sample position and tangent ahead on the spline
                let target_t = (ai.spline_t + LOOK_AHEAD_T * POINTS_PER_GATE).min(total_t);
                let target_pos = ai.spline.position(target_t);

                let vel_t = (ai.spline_t + VELOCITY_LOOK_AHEAD_T * POINTS_PER_GATE).min(total_t);
                let tangent = ai.spline.velocity(vel_t).normalize_or(Vec3::NEG_Z);

                // Per-drone lateral offset and sine noise
                let lateral = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
                let noise = (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
                    * config.noise_amplitude;
                let offset = lateral * (config.line_offset + noise);

                desired.position = target_pos + offset;
                desired.velocity_hint = tangent;
            }
            DronePhase::Returning => {
                let Some(rp) = return_path else {
                    desired.position = transform.translation;
                    desired.velocity_hint = Vec3::ZERO;
                    continue;
                };
                if rp.spline_t >= rp.total_t {
                    desired.position = transform.translation;
                    desired.velocity_hint = Vec3::ZERO;
                    continue;
                }

                let speed_frac = return_speed_fraction(rp.spline_t, rp.total_t);

                let look_ahead = LOOK_AHEAD_T * speed_frac.max(0.1);
                let target_t = (rp.spline_t + look_ahead).min(rp.total_t);
                let target_pos = rp.spline.position(target_t);

                let vel_look_ahead = VELOCITY_LOOK_AHEAD_T * speed_frac.max(0.1);
                let vel_t = (rp.spline_t + vel_look_ahead).min(rp.total_t);
                let tangent = rp.spline.velocity(vel_t).normalize_or(Vec3::NEG_Z);

                // Per-drone noise, fading as drone slows
                let lateral = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
                let noise = (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
                    * config.noise_amplitude
                    * speed_frac;
                let offset = lateral * (config.line_offset * 0.5 + noise);

                desired.position = target_pos + offset;
                desired.velocity_hint = tangent * speed_frac;
            }
        }
    }
}
