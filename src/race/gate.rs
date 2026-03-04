use bevy::prelude::*;

use crate::common::{FINISH_EXTENSION, POINTS_PER_GATE};
use crate::drone::components::{AIController, Drone, DroneDynamics, DroneIdentity, DronePhase};
use crate::drone::explosion::{CrashSounds, ExplosionMeshes};
use crate::drone::interpolation::PreviousTranslation;
use crate::obstacle::spawning::TriggerVolume;

use super::collision::crash_drone;
use super::collision_math::clip_opening_to_ground;

use super::progress::{DnfReason, RaceProgress};
use super::timing::RaceClock;

#[derive(Component)]
pub struct GateIndex(pub u32);

/// World-space forward direction of the gate (the expected approach direction).
#[derive(Component)]
pub struct GateForward(pub Vec3);

// --- Plane-crossing gate detection ---

/// Cached plane data for a single gate, built once at race start.
#[allow(dead_code)]
pub struct GatePlane {
    pub gate_index: u32,
    /// Gate center in world space (trigger volume center).
    pub center: Vec3,
    /// Gate forward (plane normal) — expected approach direction.
    pub normal: Vec3,
    /// Gate's local right axis (for bounded plane test).
    pub right: Vec3,
    /// Gate's local up axis.
    pub up: Vec3,
    /// Half-width of the gate opening (world-space).
    pub half_width: f32,
    /// Half-height of the gate opening (world-space).
    pub half_height: f32,
}

/// Cached gate planes for all gates, built once at race start.
#[derive(Resource)]
pub struct GatePlanes(pub Vec<GatePlane>);

#[allow(dead_code)]
const GATE_PASS_MARGIN: f32 = 1.1;

/// Pure function: test if a line segment (prev_pos → curr_pos) crosses a gate plane
/// in the correct direction (front-to-back) and within bounds.
#[allow(dead_code)]
pub fn plane_crossing_check(
    prev_pos: Vec3,
    curr_pos: Vec3,
    plane: &GatePlane,
    margin: f32,
) -> bool {
    let d_prev = (prev_pos - plane.center).dot(plane.normal);
    let d_curr = (curr_pos - plane.center).dot(plane.normal);

    // Must cross from front (positive) to back (negative or zero)
    if d_prev <= 0.0 || d_curr > 0.0 {
        return false;
    }

    // Interpolate to find crossing point on the plane
    let t = d_prev / (d_prev - d_curr);
    let crossing = prev_pos + t * (curr_pos - prev_pos);

    // Check crossing point is within gate bounds (with margin)
    let offset = crossing - plane.center;
    let x = offset.dot(plane.right).abs();
    let y = offset.dot(plane.up).abs();
    x < plane.half_width * margin && y < plane.half_height * margin
}

/// Builds the `GatePlanes` resource from spawned gate entities.
/// Runs every frame until gates are available, then inserts the resource.
pub fn build_gate_planes(
    mut commands: Commands,
    gate_query: Query<(Entity, &GateIndex, &GateForward, &GlobalTransform)>,
    trigger_query: Query<(&TriggerVolume, &GlobalTransform, &ChildOf)>,
) {
    if gate_query.is_empty() {
        return;
    }

    let mut planes = Vec::new();
    for (gate_entity, gate_index, gate_forward, gate_global) in &gate_query {
        // Find the trigger volume child of this gate
        for (trigger, trigger_global, child_of) in &trigger_query {
            if child_of.parent() != gate_entity {
                continue;
            }

            let center = trigger_global.translation();
            // Normal points TOWARD the approaching drones (opposite to gate forward,
            // which is the direction of travel). This ensures d_prev > 0 on the
            // approach side and d_curr <= 0 on the departure side.
            let normal = -(gate_forward.0.normalize_or(Vec3::NEG_Z));

            // Extract axes from the trigger's world rotation (includes parent + local rotation)
            let trigger_rotation = trigger_global.to_scale_rotation_translation().1;
            let right = trigger_rotation * Vec3::X;
            let up = trigger_rotation * Vec3::Y;

            // Scale half-extents by parent scale
            let gate_scale = gate_global.to_scale_rotation_translation().0;
            let half_width = trigger.half_extents.x * gate_scale.x;
            let raw_half_height = trigger.half_extents.y * gate_scale.y;

            // Clip gate opening to exclude below-ground portions
            let (clipped_center_y, clipped_half_height) =
                clip_opening_to_ground(center.y, raw_half_height);
            let clipped_center = Vec3::new(center.x, clipped_center_y, center.z);

            planes.push(GatePlane {
                gate_index: gate_index.0,
                center: clipped_center,
                normal,
                right,
                up,
                half_width,
                half_height: clipped_half_height,
            });
            break; // Only one trigger volume per gate
        }
    }

    if !planes.is_empty() {
        commands.insert_resource(GatePlanes(planes));
    }
}

/// Checks each racing drone against gate planes using line-segment crossing detection.
/// Uses PreviousTranslation → current Transform to detect plane crossings, eliminating
/// tunneling at any speed. Only counts front-to-back crossings (directional validation).
#[allow(dead_code)]
pub fn gate_trigger_check(
    mut progress: Option<ResMut<RaceProgress>>,
    clock: Option<Res<RaceClock>>,
    gate_planes: Option<Res<GatePlanes>>,
    drone_query: Query<(&Drone, &Transform, &PreviousTranslation, &DronePhase)>,
) {
    let Some(ref mut progress) = progress else {
        return;
    };
    let Some(ref clock) = clock else { return };
    if !clock.running {
        return;
    }
    let Some(ref gate_planes) = gate_planes else {
        return;
    };

    for (drone, drone_transform, prev_translation, drone_phase) in &drone_query {
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

        let Some(plane) = gate_planes.0.iter().find(|p| p.gate_index == target_gate_index) else {
            continue;
        };

        if plane_crossing_check(
            prev_translation.0,
            drone_transform.translation,
            plane,
            GATE_PASS_MARGIN,
        ) {
            if is_finish_pass {
                progress.record_finish(drone_idx, clock.elapsed);
                info!("Drone {} FINISHED at {:.2}s", drone.index, clock.elapsed);
            } else {
                progress.record_gate_pass(drone_idx, expected_gate);
                info!("Drone {} passed gate {}", drone.index, expected_gate);
            }
        }
    }
}

/// Detects when a drone has advanced past its expected gate without triggering it.
/// On miss: crash with explosion effect (hidden + particles + sound).
#[allow(dead_code)]
pub fn miss_detection(
    mut commands: Commands,
    mut progress: Option<ResMut<RaceProgress>>,
    explosion_meshes: Option<Res<ExplosionMeshes>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    crash_sounds: Option<Res<CrashSounds>>,
    mut drone_query: Query<(
        &Drone,
        &Transform,
        &AIController,
        &DroneIdentity,
        &mut DronePhase,
        &mut DroneDynamics,
        &mut Visibility,
    )>,
) {
    let Some(ref mut progress) = progress else {
        return;
    };

    for (drone, transform, ai, identity, mut phase, mut dynamics, mut visibility) in &mut drone_query {
        if *phase != DronePhase::Racing {
            continue;
        }

        let drone_idx = drone.index as usize;
        if !progress.is_active(drone_idx) {
            continue;
        }

        let expected = progress.drone_states[drone_idx].next_gate;
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;

        // Threshold: departure point + 0.5 spline units of margin.
        // The gate midpoint is at expected * PPG + 0.5; departure at +1.0.
        // Detecting at +1.5 catches the miss promptly (near the gate) rather
        // than waiting until the drone reaches the next gate.
        let miss_threshold = if expected < progress.total_gates {
            expected as f32 * POINTS_PER_GATE + 1.5
        } else {
            cycle_t + FINISH_EXTENSION
        };

        if ai.spline_t > miss_threshold {
            let crash_velocity = dynamics.velocity;

            if let Some(ref meshes) = explosion_meshes {
                crash_drone(
                    &mut commands,
                    &mut phase,
                    &mut dynamics,
                    &mut visibility,
                    drone_idx,
                    transform.translation,
                    crash_velocity,
                    Some(&mut *progress),
                    meshes,
                    &mut materials,
                    crash_sounds.as_deref(),
                    identity.color,
                    DnfReason::MissedGate(expected),
                );
            }

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

    fn default_gate_plane() -> GatePlane {
        // Gate at origin, facing +Z (normal = +Z means approach from +Z side).
        // Right = +X, Up = +Y. 4m wide, 3m tall.
        GatePlane {
            gate_index: 0,
            center: Vec3::ZERO,
            normal: Vec3::Z,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 2.0,
            half_height: 1.5,
        }
    }

    #[test]
    fn front_to_back_within_bounds() {
        let plane = default_gate_plane();
        // Fly from +Z to -Z (front to back), through center
        assert!(plane_crossing_check(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
            &plane,
            1.0,
        ));
    }

    #[test]
    fn back_to_front_rejected() {
        let plane = default_gate_plane();
        // Fly from -Z to +Z (back to front) — wrong direction
        assert!(!plane_crossing_check(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 5.0),
            &plane,
            1.0,
        ));
    }

    #[test]
    fn outside_bounds_horizontally() {
        let plane = default_gate_plane();
        // Cross the plane but 3m to the right (half_width = 2.0)
        assert!(!plane_crossing_check(
            Vec3::new(3.0, 0.0, 5.0),
            Vec3::new(3.0, 0.0, -5.0),
            &plane,
            1.0,
        ));
    }

    #[test]
    fn outside_bounds_vertically() {
        let plane = default_gate_plane();
        // Cross the plane but 2m above (half_height = 1.5)
        assert!(!plane_crossing_check(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::new(0.0, 2.0, -5.0),
            &plane,
            1.0,
        ));
    }

    #[test]
    fn both_positions_same_side_no_crossing() {
        let plane = default_gate_plane();
        // Both positions on the +Z (front) side — no crossing
        assert!(!plane_crossing_check(
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.0, 0.0, 5.0),
            &plane,
            1.0,
        ));
    }

    #[test]
    fn edge_graze_rejected_without_margin() {
        let plane = default_gate_plane();
        // Cross at x=1.99, just inside half_width=2.0 → passes with margin=1.0
        assert!(plane_crossing_check(
            Vec3::new(1.99, 0.0, 5.0),
            Vec3::new(1.99, 0.0, -5.0),
            &plane,
            1.0,
        ));
        // Cross at x=2.05, just outside half_width=2.0 → fails with margin=1.0
        assert!(!plane_crossing_check(
            Vec3::new(2.05, 0.0, 5.0),
            Vec3::new(2.05, 0.0, -5.0),
            &plane,
            1.0,
        ));
        // But passes with margin=1.1 (effective half_width = 2.2)
        assert!(plane_crossing_check(
            Vec3::new(2.05, 0.0, 5.0),
            Vec3::new(2.05, 0.0, -5.0),
            &plane,
            1.1,
        ));
    }

    #[test]
    fn rotated_gate_90_degrees() {
        // Gate rotated 90° around Y: now faces +X (normal=+X, right=−Z, up=+Y)
        let plane = GatePlane {
            gate_index: 0,
            center: Vec3::ZERO,
            normal: Vec3::X,
            right: Vec3::NEG_Z,
            up: Vec3::Y,
            half_width: 2.0,
            half_height: 1.5,
        };
        // Approach from +X, depart toward −X
        assert!(plane_crossing_check(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(-5.0, 0.0, 0.0),
            &plane,
            1.0,
        ));
        // Wrong direction (−X to +X)
        assert!(!plane_crossing_check(
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            &plane,
            1.0,
        ));
    }

    #[test]
    fn flipped_gate_forward() {
        // Simulates gate_forward_flipped: normal is −Z instead of +Z.
        // Drone must now approach from −Z side.
        let plane = GatePlane {
            gate_index: 0,
            center: Vec3::ZERO,
            normal: Vec3::NEG_Z,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 2.0,
            half_height: 1.5,
        };
        // Approach from −Z (front) to +Z (back)
        assert!(plane_crossing_check(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 5.0),
            &plane,
            1.0,
        ));
        // Original +Z to −Z direction should now be rejected
        assert!(!plane_crossing_check(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
            &plane,
            1.0,
        ));
    }
}
