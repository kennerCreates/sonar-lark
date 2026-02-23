use bevy::prelude::*;

use super::components::*;

const WAYPOINT_REACH_DISTANCE: f32 = 5.0;
const LOOK_AHEAD_T: f32 = 0.3;
const VELOCITY_LOOK_AHEAD_T: f32 = 0.5;
const MAX_ADVANCE_PER_TICK: f32 = 0.15;

pub fn update_ai_targets(mut query: Query<(&Transform, &mut AIController), With<Drone>>) {
    for (transform, mut ai) in &mut query {
        let total_t = ai.gate_count as f32;
        if ai.spline_t >= total_t {
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
            let advance = (forward_proj / tangent_len).clamp(0.0, MAX_ADVANCE_PER_TICK);
            ai.spline_t += advance;
        }

        // Fallback: snap forward if drone is close to the next gate center.
        // Handles edge cases where projection stalls (e.g., after an overshoot).
        let next_gate_t = (ai.spline_t.floor() + 1.0).min(total_t);
        let next_gate_idx = (next_gate_t as usize) % ai.gate_positions.len();
        let dist_to_next =
            (transform.translation - ai.gate_positions[next_gate_idx]).length();
        if dist_to_next < WAYPOINT_REACH_DISTANCE {
            ai.spline_t = ai.spline_t.max(next_gate_t);
        }

        ai.target_gate_index = (ai.spline_t.floor() as u32).min(ai.gate_count - 1);
    }
}

pub fn compute_racing_line(
    time: Res<Time>,
    mut query: Query<(&Transform, &AIController, &DroneConfig, &mut DesiredPosition)>,
) {
    let elapsed = time.elapsed_secs();

    for (transform, ai, config, mut desired) in &mut query {
        let total_t = ai.gate_count as f32;
        if ai.spline_t >= total_t {
            desired.position = transform.translation;
            desired.velocity_hint = Vec3::ZERO;
            continue;
        }

        // Sample position and tangent ahead on the spline
        let target_t = (ai.spline_t + LOOK_AHEAD_T).min(total_t);
        let target_pos = ai.spline.position(target_t);

        let vel_t = (ai.spline_t + VELOCITY_LOOK_AHEAD_T).min(total_t);
        let tangent = ai.spline.velocity(vel_t).normalize_or(Vec3::NEG_Z);

        // Per-drone lateral offset and sine noise
        let lateral = Vec3::Y.cross(tangent).normalize_or(Vec3::X);
        let noise =
            (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
                * config.noise_amplitude;
        let offset = lateral * (config.line_offset + noise);

        desired.position = target_pos + offset;
        desired.velocity_hint = tangent;
    }
}
