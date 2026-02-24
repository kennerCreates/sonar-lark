use bevy::prelude::*;

use super::components::*;
use super::spawning::generate_return_path;

const WAYPOINT_REACH_DISTANCE: f32 = 5.0;
const VELOCITY_LOOK_AHEAD_T: f32 = 0.5;
const MAX_ADVANCE_PER_TICK: f32 = 0.15;

/// How far past a full cycle the race extends. Drones must fly through
/// the start/finish gate again (completing a full lap) before transitioning.
/// 1.5 puts the finish well past gate 0's departure (at cycle + 1.0).
const FINISH_EXTENSION: f32 = 1.5;

/// How many samples ahead to scan for upcoming curvature (for speed limiting).
const SPEED_CURVATURE_SAMPLES: usize = 5;

/// Sample position from the cyclic race spline, wrapping t into [0, cycle_t).
fn cyclic_pos(spline: &bevy::math::cubic_splines::CubicCurve<Vec3>, t: f32, cycle_t: f32) -> Vec3 {
    spline.position(t.rem_euclid(cycle_t))
}

/// Sample velocity/tangent from the cyclic race spline, wrapping t into [0, cycle_t).
fn cyclic_vel(spline: &bevy::math::cubic_splines::CubicCurve<Vec3>, t: f32, cycle_t: f32) -> Vec3 {
    spline.velocity(t.rem_euclid(cycle_t))
}

/// Sample acceleration from the cyclic race spline, wrapping t into [0, cycle_t).
fn cyclic_accel(spline: &bevy::math::cubic_splines::CubicCurve<Vec3>, t: f32, cycle_t: f32) -> Vec3 {
    spline.acceleration(t.rem_euclid(cycle_t))
}

/// Compute curvature κ = |v × a| / |v|³ at parameter t on the cyclic spline.
pub fn cyclic_curvature(spline: &bevy::math::cubic_splines::CubicCurve<Vec3>, t: f32, cycle_t: f32) -> f32 {
    let vel = cyclic_vel(spline, t, cycle_t);
    let acc = cyclic_accel(spline, t, cycle_t);
    let vel_mag = vel.length();
    if vel_mag < 0.001 {
        return 0.0;
    }
    vel.cross(acc).length() / (vel_mag * vel_mag * vel_mag)
}

/// Scan curvature over a range ahead and return the maximum (tightest upcoming turn).
fn max_curvature_ahead(
    spline: &bevy::math::cubic_splines::CubicCurve<Vec3>,
    current_t: f32,
    range: f32,
    cycle_t: f32,
) -> f32 {
    let mut max_k = 0.0f32;
    for i in 0..SPEED_CURVATURE_SAMPLES {
        let sample_t = current_t + (i as f32 / (SPEED_CURVATURE_SAMPLES - 1).max(1) as f32) * range;
        max_k = max_k.max(cyclic_curvature(spline, sample_t, cycle_t));
    }
    max_k
}

/// Convert curvature to a safe speed: v = sqrt(a_lateral / κ).
/// `lateral_accel` allows per-drone override of the global safe_lateral_accel.
pub fn safe_speed_for_curvature_with(curvature: f32, lateral_accel: f32, tuning: &AiTuningParams) -> f32 {
    if curvature > 0.001 {
        (lateral_accel / curvature)
            .sqrt()
            .clamp(tuning.min_curvature_speed, tuning.max_speed)
    } else {
        tuning.max_speed
    }
}

/// Convert curvature to a safe speed using the global safe_lateral_accel.
pub fn safe_speed_for_curvature(curvature: f32, tuning: &AiTuningParams) -> f32 {
    safe_speed_for_curvature_with(curvature, tuning.safe_lateral_accel, tuning)
}

/// Smoothstep deceleration: 1.0 at start of return → 0.0 at arrival.
fn return_speed_fraction(spline_t: f32, total_t: f32) -> f32 {
    let progress = (spline_t / total_t).clamp(0.0, 1.0);
    let inv = 1.0 - progress;
    inv * inv * (3.0 - 2.0 * inv)
}

pub fn update_ai_targets(
    mut commands: Commands,
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
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
    let dt = time.delta_secs();

    for (entity, transform, mut ai, mut phase, dynamics, start_pos, config, drone, return_path) in
        &mut query
    {
        match *phase {
            DronePhase::Idle => continue,
            DronePhase::Racing => {
                let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
                let finish_t = cycle_t + FINISH_EXTENSION;

                if ai.spline_t >= finish_t {
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
                        *phase = DronePhase::Idle;
                    }
                    continue;
                }

                // Local projection: advance spline_t along the cyclic spline tangent.
                let curve_pos = cyclic_pos(&ai.spline, ai.spline_t, cycle_t);
                let tangent = cyclic_vel(&ai.spline, ai.spline_t, cycle_t);
                let tangent_len = tangent.length();
                if tangent_len > 0.001 {
                    let tangent_dir = tangent / tangent_len;
                    let displacement = transform.translation - curve_pos;
                    let forward_proj = displacement.dot(tangent_dir);
                    let projection_advance = forward_proj / tangent_len;

                    // Minimum advancement based on drone speed prevents deadlock when
                    // the drone overshoots a turn and forward_proj drops to zero.
                    let speed = dynamics.velocity.length();
                    let min_advance =
                        speed * dt * tuning.min_advance_speed_fraction / tangent_len;

                    let advance = projection_advance
                        .max(min_advance)
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
                } else if next_gate_idx == ai.gate_positions.len() {
                    // Wrap-around: snap past gate 0 when completing the lap
                    let finish_gate_t = cycle_t + 0.5;
                    let dist_to_finish =
                        (transform.translation - ai.gate_positions[0]).length();
                    if dist_to_finish < WAYPOINT_REACH_DISTANCE {
                        ai.spline_t = ai.spline_t.max(finish_gate_t);
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

                    // Minimum advancement for return path too
                    let speed = dynamics.velocity.length();
                    let min_advance =
                        speed * dt * tuning.min_advance_speed_fraction / tangent_len;

                    let advance = (forward_proj / tangent_len)
                        .max(min_advance)
                        .clamp(0.0, max_advance);
                    rp.spline_t += advance;
                }
                rp.spline_t = rp.spline_t.min(rp.total_t);
            }
        }
    }
}

pub fn compute_racing_line(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
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
                let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
                let finish_t = cycle_t + FINISH_EXTENSION;

                if ai.spline_t >= finish_t {
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

                // Per-drone lateral offset and sine noise
                let lateral = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
                let noise = (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
                    * config.noise_amplitude;
                let offset = lateral * (config.line_offset + noise);

                desired.position = target_pos + offset;
                desired.velocity_hint = tangent;

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
            DronePhase::Returning => {
                let Some(rp) = return_path else {
                    desired.position = transform.translation;
                    desired.velocity_hint = Vec3::ZERO;
                    desired.max_speed = tuning.max_speed;
                    continue;
                };
                if rp.spline_t >= rp.total_t {
                    desired.position = transform.translation;
                    desired.velocity_hint = Vec3::ZERO;
                    desired.max_speed = tuning.max_speed;
                    continue;
                }

                let speed_frac = return_speed_fraction(rp.spline_t, rp.total_t);

                let look_ahead = tuning.look_ahead_t * speed_frac.max(0.1);
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
                desired.max_speed = tuning.max_speed * speed_frac.max(0.2);
            }
        }
    }
}
