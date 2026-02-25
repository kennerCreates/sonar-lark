use bevy::prelude::*;

use crate::drone::ai::FINISH_EXTENSION;
use crate::drone::components::{AIController, Drone, DroneDynamics, DronePhase, POINTS_PER_GATE};
use crate::drone::explosion::{self, ExplosionSounds};
use crate::drone::spawning::DRONE_COLORS;
use crate::obstacle::spawning::TriggerVolume;

use super::progress::{DnfReason, RaceProgress};
use super::timing::RaceClock;

#[derive(Component)]
pub struct GateIndex(pub u32);

/// World-space forward direction of the gate (the expected approach direction).
#[derive(Component)]
pub struct GateForward(pub Vec3);

/// Pure function: test if a point is inside a trigger volume defined by its
/// GlobalTransform and half_extents. Handles rotation and non-uniform scale.
pub fn point_in_trigger_volume(
    point: Vec3,
    trigger_global: &GlobalTransform,
    half_extents: Vec3,
) -> bool {
    let inv = trigger_global.affine().inverse();
    let local = inv.transform_point3(point);
    local.x.abs() < half_extents.x
        && local.y.abs() < half_extents.y
        && local.z.abs() < half_extents.z
}

/// Checks each racing drone against gate trigger volumes and records gate passes.
pub fn gate_trigger_check(
    mut progress: Option<ResMut<RaceProgress>>,
    clock: Option<Res<RaceClock>>,
    trigger_query: Query<(&TriggerVolume, &GlobalTransform, &ChildOf)>,
    gate_query: Query<&GateIndex>,
    drone_query: Query<(&Drone, &Transform, &DronePhase)>,
) {
    let Some(ref mut progress) = progress else {
        return;
    };
    let Some(ref clock) = clock else { return };
    if !clock.running {
        return;
    }

    for (drone, drone_transform, drone_phase) in &drone_query {
        if *drone_phase != DronePhase::Racing {
            continue;
        }

        let drone_idx = drone.index as usize;
        if !progress.is_active(drone_idx) {
            continue;
        }

        let expected_gate = progress.drone_states[drone_idx].next_gate;

        // After passing all gates, the drone must pass through gate 0 again (finish line)
        let is_finish_pass = expected_gate == progress.total_gates;
        let target_gate_index = if is_finish_pass { 0 } else { expected_gate };

        for (trigger, trigger_global, child_of) in &trigger_query {
            let Ok(gate_index) = gate_query.get(child_of.parent()) else {
                continue;
            };

            if gate_index.0 != target_gate_index {
                continue;
            }

            if point_in_trigger_volume(
                drone_transform.translation,
                trigger_global,
                trigger.half_extents,
            ) {
                if is_finish_pass {
                    progress.record_finish(drone_idx, clock.elapsed);
                    info!("Drone {} FINISHED at {:.2}s", drone.index, clock.elapsed);
                } else {
                    progress.record_gate_pass(drone_idx, expected_gate);
                    info!("Drone {} passed gate {}", drone.index, expected_gate);
                }
                break;
            }
        }
    }
}

/// Detects when a drone has advanced past its expected gate without triggering it.
/// On miss: crash with explosion effect (hidden + particles + sound).
pub fn miss_detection(
    mut commands: Commands,
    mut progress: Option<ResMut<RaceProgress>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    explosion_sounds: Option<Res<ExplosionSounds>>,
    mut drone_query: Query<(
        &Drone,
        &Transform,
        &AIController,
        &mut DronePhase,
        &mut DroneDynamics,
        &mut Visibility,
    )>,
) {
    let Some(ref mut progress) = progress else {
        return;
    };

    for (drone, transform, ai, mut phase, mut dynamics, mut visibility) in &mut drone_query {
        if *phase != DronePhase::Racing {
            continue;
        }

        let drone_idx = drone.index as usize;
        if !progress.is_active(drone_idx) {
            continue;
        }

        let expected = progress.drone_states[drone_idx].next_gate;
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;

        let miss_threshold = if expected < progress.total_gates {
            (expected as f32 + 1.0) * POINTS_PER_GATE
        } else {
            cycle_t + FINISH_EXTENSION
        };

        if ai.spline_t > miss_threshold {
            progress.record_crash(drone_idx, DnfReason::MissedGate(expected));
            let crash_velocity = dynamics.velocity;
            *phase = DronePhase::Crashed;
            dynamics.velocity = Vec3::ZERO;
            dynamics.angular_velocity = Vec3::ZERO;
            *visibility = Visibility::Hidden;

            let [r, g, b] = DRONE_COLORS[drone_idx];
            explosion::spawn_explosion(
                &mut commands,
                &mut meshes,
                &mut materials,
                transform.translation,
                crash_velocity,
                Color::srgb(r, g, b),
                explosion_sounds.as_deref(),
            );

            warn!(
                "Drone {} CRASHED — missed gate {} (spline_t={:.1}, threshold={:.1})",
                drone.index, expected, ai.spline_t, miss_threshold
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_inside_identity_trigger() {
        let gt = GlobalTransform::from(Transform::IDENTITY);
        assert!(point_in_trigger_volume(
            Vec3::new(0.5, 0.5, 0.5),
            &gt,
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_outside_identity_trigger() {
        let gt = GlobalTransform::from(Transform::IDENTITY);
        assert!(!point_in_trigger_volume(
            Vec3::new(2.0, 0.0, 0.0),
            &gt,
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_inside_translated_trigger() {
        let gt = GlobalTransform::from(Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)));
        assert!(point_in_trigger_volume(
            Vec3::new(10.5, 0.0, 0.0),
            &gt,
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_outside_translated_trigger() {
        let gt = GlobalTransform::from(Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)));
        assert!(!point_in_trigger_volume(
            Vec3::new(0.5, 0.0, 0.0),
            &gt,
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_inside_rotated_trigger() {
        // Rotate 90 degrees around Y: local X becomes world Z
        let gt = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(
            std::f32::consts::FRAC_PI_2,
        )));
        // half_extents (2, 1, 1): local X extends ±2.
        // After 90° Y rotation, world Z maps to local X.
        // So a point at world (0, 0, 1.5) should be inside (local X = 1.5 < 2.0)
        assert!(point_in_trigger_volume(
            Vec3::new(0.0, 0.0, 1.5),
            &gt,
            Vec3::new(2.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_outside_rotated_trigger() {
        let gt = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(
            std::f32::consts::FRAC_PI_2,
        )));
        // After 90° Y rotation, world X maps to local -Z.
        // half_extents Z = 1.0, so world X=1.5 → local Z=1.5 > 1.0 → outside
        assert!(!point_in_trigger_volume(
            Vec3::new(1.5, 0.0, 0.0),
            &gt,
            Vec3::new(2.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_inside_scaled_trigger() {
        // Parent with scale 2.0. Trigger half_extents are (1, 1, 1) in local space.
        // After parent scale, effective world-space extent is 2.0 in each axis.
        let gt = GlobalTransform::from(Transform::from_scale(Vec3::splat(2.0)));
        // World position 1.5 → local position 1.5/2.0 = 0.75 < 1.0 → inside
        assert!(point_in_trigger_volume(
            Vec3::new(1.5, 0.0, 0.0),
            &gt,
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }

    #[test]
    fn point_outside_scaled_trigger() {
        let gt = GlobalTransform::from(Transform::from_scale(Vec3::splat(2.0)));
        // World position 2.5 → local position 2.5/2.0 = 1.25 > 1.0 → outside
        assert!(!point_in_trigger_volume(
            Vec3::new(2.5, 0.0, 0.0),
            &gt,
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }
}
