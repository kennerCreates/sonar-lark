use bevy::prelude::*;

use super::components::*;

const WAYPOINT_REACH_DISTANCE: f32 = 5.0;
const LOOK_AHEAD_BLEND: f32 = 0.3;

pub fn update_ai_targets(mut query: Query<(&Transform, &mut AIController), With<Drone>>) {
    for (transform, mut ai) in &mut query {
        if ai.current_waypoint >= ai.waypoints.len() {
            continue;
        }

        let target = ai.waypoints[ai.current_waypoint];
        let distance = (transform.translation - target).length();

        if distance < WAYPOINT_REACH_DISTANCE {
            ai.current_waypoint += 1;
            if ai.current_waypoint < ai.waypoints.len() {
                ai.target_gate_index = ai.current_waypoint as u32;
            }
        }
    }
}

pub fn compute_racing_line(
    time: Res<Time>,
    mut query: Query<(&Transform, &AIController, &DroneConfig, &mut DesiredPosition)>,
) {
    let elapsed = time.elapsed_secs();

    for (transform, ai, config, mut desired) in &mut query {
        if ai.current_waypoint >= ai.waypoints.len() {
            // Finished: hold current position
            desired.position = transform.translation;
            desired.velocity_hint = Vec3::ZERO;
            continue;
        }

        let target = ai.waypoints[ai.current_waypoint];
        let to_target = target - transform.translation;
        let forward = to_target.normalize_or(Vec3::NEG_Z);

        // Lateral direction perpendicular to forward on XZ plane
        let lateral = Vec3::Y.cross(forward).normalize_or(Vec3::X);

        // Per-drone sine noise for racing line variation
        let noise =
            (elapsed * config.noise_frequency + config.line_offset * 3.14).sin()
                * config.noise_amplitude;

        let offset = lateral * (config.line_offset + noise);

        // Look-ahead: blend toward next waypoint for smoother cornering
        let look_ahead = if ai.current_waypoint + 1 < ai.waypoints.len() {
            let next = ai.waypoints[ai.current_waypoint + 1];
            let blend =
                1.0 - (to_target.length() / WAYPOINT_REACH_DISTANCE).min(1.0);
            target.lerp(next, blend * LOOK_AHEAD_BLEND)
        } else {
            target
        };

        desired.position = look_ahead + offset;
        desired.velocity_hint = forward;
    }
}
