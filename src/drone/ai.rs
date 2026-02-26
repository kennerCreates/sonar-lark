use bevy::prelude::*;

use crate::race::progress::RaceProgress;

use super::components::*;

const WAYPOINT_REACH_DISTANCE: f32 = 5.0;
const VELOCITY_LOOK_AHEAD_T: f32 = 0.5;
const MAX_ADVANCE_PER_TICK: f32 = 0.15;

/// How far past a full cycle the race extends. Drones must fly through
/// the start/finish gate again (completing a full lap) before transitioning.
/// 1.5 puts the finish well past gate 0's departure (at cycle + 1.0).
pub const FINISH_EXTENSION: f32 = 1.5;

/// Small epsilon added to finish_t guards so that spline_t can advance slightly
/// past the finish, allowing miss_detection's strict `>` check to fire on drones
/// that miss the finish gate.
const FINISH_EPSILON: f32 = 0.01;

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
            .clamp(tuning.min_curvature_speed.min(tuning.max_speed), tuning.max_speed)
    } else {
        tuning.max_speed
    }
}

/// Convert curvature to a safe speed using the global safe_lateral_accel.
pub fn safe_speed_for_curvature(curvature: f32, tuning: &AiTuningParams) -> f32 {
    safe_speed_for_curvature_with(curvature, tuning.safe_lateral_accel, tuning)
}

pub fn update_ai_targets(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    race_progress: Option<Res<RaceProgress>>,
    mut query: Query<(
        &Transform,
        &mut AIController,
        &mut DronePhase,
        &DroneDynamics,
        &Drone,
    )>,
) {
    let dt = time.delta_secs();

    for (transform, mut ai, mut phase, dynamics, drone) in &mut query {
        match *phase {
            DronePhase::Idle | DronePhase::Crashed | DronePhase::Returning => continue,
            DronePhase::Racing => {
                let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
                let finish_t = cycle_t + FINISH_EXTENSION;

                if ai.spline_t >= finish_t + FINISH_EPSILON {
                    // Only transition to VictoryLap if RaceProgress confirms the drone finished.
                    // If not finished, stay Racing so miss_detection (in Update) can crash it.
                    let confirmed = race_progress.as_ref().map_or(true, |p| {
                        p.drone_states
                            .get(drone.index as usize)
                            .is_some_and(|s| s.finished || s.crashed)
                    });

                    if !confirmed {
                        continue;
                    }

                    // Transition: Racing → VictoryLap, wrap spline_t for continued lapping
                    *phase = DronePhase::VictoryLap;
                    ai.spline_t -= cycle_t;
                    continue;
                }

                advance_racing_spline(transform, &mut ai, dynamics, &tuning, dt);
            }
            DronePhase::VictoryLap => {
                let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;
                advance_cyclic_spline(transform, &mut ai, dynamics, &tuning, dt, cycle_t);
            }
        }
    }
}

/// Advance spline_t for a racing drone (with gate snap and finish-line logic).
fn advance_racing_spline(
    transform: &Transform,
    ai: &mut AIController,
    dynamics: &DroneDynamics,
    tuning: &AiTuningParams,
    dt: f32,
) {
    let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;

    // Local projection: advance spline_t along the cyclic spline tangent.
    let curve_pos = cyclic_pos(&ai.spline, ai.spline_t, cycle_t);
    let tangent = cyclic_vel(&ai.spline, ai.spline_t, cycle_t);
    let tangent_len = tangent.length();
    if tangent_len > 0.001 {
        let tangent_dir = tangent / tangent_len;
        let displacement = transform.translation - curve_pos;
        let forward_proj = displacement.dot(tangent_dir);
        let projection_advance = forward_proj / tangent_len;

        let speed = dynamics.velocity.length();
        let min_advance = speed * dt * tuning.min_advance_speed_fraction / tangent_len;

        let advance = projection_advance
            .max(min_advance)
            .clamp(0.0, MAX_ADVANCE_PER_TICK * POINTS_PER_GATE);
        ai.spline_t += advance;
    }

    // Fallback: snap forward if drone is close to the next gate center.
    let next_gate_idx = ((ai.spline_t - 0.5) / POINTS_PER_GATE + 1.0).floor() as usize;
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
        let dist_to_finish = (transform.translation - ai.gate_positions[0]).length();
        if dist_to_finish < WAYPOINT_REACH_DISTANCE {
            ai.spline_t = ai.spline_t.max(finish_gate_t);
        }
    }

    ai.target_gate_index = (ai.spline_t / POINTS_PER_GATE).floor() as u32;
    ai.target_gate_index = ai.target_gate_index.min(ai.gate_count - 1);
}

/// Advance spline_t cyclically for a victory-lapping drone (no finish check, wraps around).
fn advance_cyclic_spline(
    transform: &Transform,
    ai: &mut AIController,
    dynamics: &DroneDynamics,
    tuning: &AiTuningParams,
    dt: f32,
    cycle_t: f32,
) {
    let curve_pos = cyclic_pos(&ai.spline, ai.spline_t, cycle_t);
    let tangent = cyclic_vel(&ai.spline, ai.spline_t, cycle_t);
    let tangent_len = tangent.length();
    if tangent_len > 0.001 {
        let tangent_dir = tangent / tangent_len;
        let displacement = transform.translation - curve_pos;
        let forward_proj = displacement.dot(tangent_dir);
        let projection_advance = forward_proj / tangent_len;

        let speed = dynamics.velocity.length();
        let min_advance = speed * dt * tuning.min_advance_speed_fraction / tangent_len;

        let advance = projection_advance
            .max(min_advance)
            .clamp(0.0, MAX_ADVANCE_PER_TICK * POINTS_PER_GATE);
        ai.spline_t += advance;
    }

    // Wrap cyclically
    if ai.spline_t >= cycle_t {
        ai.spline_t -= cycle_t;
    }
}

/// Maximum lateral offset a drone can be pushed by avoidance (meters).
const MAX_AVOIDANCE_OFFSET: f32 = 3.0;

/// Distance to next gate at which proximity avoidance starts to fade.
const GATE_SUPPRESS_RADIUS: f32 = 10.0;
/// Minimum avoidance multiplier at the gate itself (20% of full avoidance).
const GATE_SUPPRESS_MIN: f32 = 0.2;

pub fn compute_racing_line(
    time: Res<Time>,
    tuning: Res<AiTuningParams>,
    mut query: Query<(
        &Transform,
        &AIController,
        &DroneConfig,
        &DronePhase,
        &mut DesiredPosition,
    )>,
) {
    let elapsed = time.elapsed_secs();

    for (transform, ai, config, phase, mut desired) in &mut query {
        match *phase {
            DronePhase::Idle | DronePhase::Crashed | DronePhase::Returning => continue,
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

                // Small organic wobble — main per-drone variation comes from the unique spline
                let lateral = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
                let noise = (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
                    * config.noise_amplitude * 0.3;
                let offset = lateral * noise;

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
                let noise = (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
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

/// Proximity avoidance: offsets DesiredPosition laterally when drones are nearby.
/// Drones veer around each other at full speed instead of slowing down.
/// Runs after compute_racing_line, before position_pid.
pub fn proximity_avoidance(
    tuning: Res<AiTuningParams>,
    mut query: Query<(
        &Transform,
        &Drone,
        &DroneDynamics,
        &DronePhase,
        &AIController,
        &mut DesiredPosition,
    )>,
) {
    if tuning.avoidance_strength == 0.0 {
        return;
    }

    let radius = tuning.avoidance_radius;
    let radius_sq = radius * radius;

    // Snapshot positions, velocities, and indices (stack-allocated, 12 drones max)
    let mut drone_data = [(0u8, Vec3::ZERO, Vec3::ZERO); 12];
    let mut drone_count = 0;
    for (tr, drone, dyn_, phase, _, _) in query.iter() {
        if matches!(*phase, DronePhase::Idle | DronePhase::Crashed) { continue; }
        if drone_count < 12 {
            drone_data[drone_count] = (drone.index, tr.translation, dyn_.velocity);
            drone_count += 1;
        }
    }
    let drone_data = &drone_data[..drone_count];

    for (transform, drone, dynamics, phase, ai, mut desired) in &mut query {
        if matches!(*phase, DronePhase::Idle | DronePhase::Crashed) {
            continue;
        }

        let my_pos = transform.translation;
        let my_idx = drone.index;
        let my_vel = dynamics.velocity;
        let my_speed = my_vel.length();

        let mut total_offset = Vec3::ZERO;

        for &(other_idx, other_pos, _) in drone_data {
            if other_idx == my_idx {
                continue;
            }

            let separation = my_pos - other_pos;
            let dist_sq = separation.length_squared();
            if dist_sq > radius_sq || dist_sq < 0.01 {
                continue;
            }

            let dist = dist_sq.sqrt();

            // Compute lateral dodge direction (perpendicular to own velocity)
            let lateral = if my_speed > 1.0 {
                let vel_dir = my_vel / my_speed;
                let proj = separation - vel_dir * separation.dot(vel_dir);

                // Head-on tiebreaker: if lateral separation is tiny, use deterministic perpendicular
                if proj.length_squared() < 0.25 {
                    let perp = Vec3::new(vel_dir.z, 0.0, -vel_dir.x);
                    let sign = if my_idx > other_idx { 1.0 } else { -1.0 };
                    perp * sign
                } else {
                    proj
                }
            } else {
                separation
            };

            let lateral_dir = lateral.normalize_or(Vec3::X);

            // Smooth quadratic falloff: strongest when closest
            let t = 1.0 - dist / radius;
            let weight = t * t;

            total_offset += lateral_dir * weight;
        }

        // Suppress avoidance near the next gate so drones don't get pushed out of the opening
        let gate_idx = (ai.target_gate_index as usize).min(ai.gate_positions.len().saturating_sub(1));
        let gate_dist = (my_pos - ai.gate_positions[gate_idx]).length();
        let suppress = if gate_dist < GATE_SUPPRESS_RADIUS {
            GATE_SUPPRESS_MIN
                + (1.0 - GATE_SUPPRESS_MIN) * (gate_dist / GATE_SUPPRESS_RADIUS)
        } else {
            1.0
        };

        let offset = total_offset * tuning.avoidance_strength * suppress;
        let mag = offset.length();
        if mag > MAX_AVOIDANCE_OFFSET {
            desired.position += offset * (MAX_AVOIDANCE_OFFSET / mag);
        } else {
            desired.position += offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::cubic_splines::CubicCardinalSpline;

    fn default_tuning() -> AiTuningParams {
        AiTuningParams::default()
    }

    // --- safe_speed_for_curvature_with ---

    #[test]
    fn zero_curvature_returns_max_speed() {
        let tuning = default_tuning();
        let speed = safe_speed_for_curvature_with(0.0, tuning.safe_lateral_accel, &tuning);
        assert_eq!(speed, tuning.max_speed);
    }

    #[test]
    fn tiny_curvature_returns_max_speed() {
        let tuning = default_tuning();
        let speed = safe_speed_for_curvature_with(0.0005, tuning.safe_lateral_accel, &tuning);
        assert_eq!(speed, tuning.max_speed);
    }

    #[test]
    fn high_curvature_returns_min_speed() {
        let tuning = default_tuning();
        // κ = 100 → v = sqrt(50/100) = 0.707, clamped to min_curvature_speed
        let speed = safe_speed_for_curvature_with(100.0, tuning.safe_lateral_accel, &tuning);
        assert_eq!(speed, tuning.min_curvature_speed);
    }

    #[test]
    fn moderate_curvature_between_limits() {
        let tuning = default_tuning();
        // κ = 0.05 → v = sqrt(50/0.05) = sqrt(1000) ≈ 31.6
        let speed = safe_speed_for_curvature_with(0.05, tuning.safe_lateral_accel, &tuning);
        assert!(speed > tuning.min_curvature_speed);
        assert!(speed < tuning.max_speed);
        assert!((speed - (50.0f32 / 0.05).sqrt()).abs() < 0.01);
    }

    #[test]
    fn higher_lateral_accel_gives_faster_speed() {
        let tuning = default_tuning();
        let k = 0.1;
        let slow = safe_speed_for_curvature_with(k, 30.0, &tuning);
        let fast = safe_speed_for_curvature_with(k, 80.0, &tuning);
        assert!(fast > slow);
    }

    // --- safe_speed_for_curvature (delegates to _with) ---

    #[test]
    fn safe_speed_uses_global_lateral_accel() {
        let tuning = default_tuning();
        let k = 0.05;
        let a = safe_speed_for_curvature(k, &tuning);
        let b = safe_speed_for_curvature_with(k, tuning.safe_lateral_accel, &tuning);
        assert_eq!(a, b);
    }

    // --- cyclic_curvature ---

    fn make_circle_spline() -> (bevy::math::cubic_splines::CubicCurve<Vec3>, f32) {
        // Approximate a circle with Catmull-Rom through 8 evenly spaced points
        let n = 8;
        let r = 10.0;
        let points: Vec<Vec3> = (0..n)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / n as f32;
                Vec3::new(r * angle.cos(), 0.0, r * angle.sin())
            })
            .collect();
        let spline = CubicCardinalSpline::new_catmull_rom(points)
            .to_curve_cyclic()
            .expect("spline creation failed");
        let cycle_t = n as f32;
        (spline, cycle_t)
    }

    #[test]
    fn circle_curvature_is_roughly_constant() {
        let (spline, cycle_t) = make_circle_spline();
        let mut curvatures = Vec::new();
        for i in 0..16 {
            let t = i as f32 / 16.0 * cycle_t;
            curvatures.push(cyclic_curvature(&spline, t, cycle_t));
        }
        let mean = curvatures.iter().sum::<f32>() / curvatures.len() as f32;
        // All samples should be within 50% of mean (Catmull-Rom isn't a perfect circle)
        for (i, &k) in curvatures.iter().enumerate() {
            assert!(
                (k - mean).abs() / mean < 0.5,
                "Curvature at sample {i} ({k:.4}) too far from mean ({mean:.4})"
            );
        }
        // Expected curvature for radius 10: κ = 1/r = 0.1
        assert!(mean > 0.05 && mean < 0.2, "Mean curvature {mean} not near 1/r=0.1");
    }

    #[test]
    fn straight_line_has_low_curvature() {
        // Nearly straight: 4 collinear-ish points with slight offset to avoid degenerate tangents
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.001),
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::new(30.0, 0.0, 0.001),
        ];
        let spline = CubicCardinalSpline::new_catmull_rom(points)
            .to_curve_cyclic()
            .expect("spline creation failed");
        let cycle_t = 4.0;
        // Sample near the middle of the first segment
        let k = cyclic_curvature(&spline, 0.5, cycle_t);
        assert!(k < 0.05, "Straight-ish spline curvature {k} should be near zero");
    }
}
