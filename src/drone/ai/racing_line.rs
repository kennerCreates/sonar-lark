use bevy::prelude::*;

use super::super::components::*;
use super::{
    VELOCITY_LOOK_AHEAD_T,
    cyclic_curvature, cyclic_pos, cyclic_vel, max_curvature_ahead,
    safe_speed_for_curvature_with,
};
use crate::common::POINTS_PER_GATE;

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

    for (_transform, ai, config, phase, mut desired) in &mut query {
        match *phase {
            // Racing drones are handled by the choreography chain.
            DronePhase::Idle | DronePhase::Crashed | DronePhase::Wandering | DronePhase::Racing => continue,
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
