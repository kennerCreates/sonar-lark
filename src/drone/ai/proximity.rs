use bevy::prelude::*;

use super::super::components::*;

/// Maximum lateral offset a drone can be pushed by avoidance (meters).
const MAX_AVOIDANCE_OFFSET: f32 = 3.0;

/// Distance to next gate at which proximity avoidance starts to fade.
const GATE_SUPPRESS_RADIUS: f32 = 10.0;
/// Minimum avoidance multiplier at the gate itself (20% of full avoidance).
const GATE_SUPPRESS_MIN: f32 = 0.2;

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
        // Racing drones are handled by the choreography chain.
        if matches!(*phase, DronePhase::Idle | DronePhase::Crashed | DronePhase::Racing) { continue; }
        if drone_count < 12 {
            drone_data[drone_count] = (drone.index, tr.translation, dyn_.velocity);
            drone_count += 1;
        }
    }
    let drone_data = &drone_data[..drone_count];

    for (transform, drone, dynamics, phase, ai, mut desired) in &mut query {
        if matches!(*phase, DronePhase::Idle | DronePhase::Crashed | DronePhase::Racing) {
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
