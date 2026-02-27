use bevy::prelude::*;

use crate::drone::components::{Drone, DroneDynamics, DronePhase};
use crate::drone::explosion::{self, ExplosionMeshes, ExplosionSounds};
use crate::drone::interpolation::PreviousTranslation;
use crate::drone::spawning::DRONE_COLORS;
use crate::obstacle::spawning::{ObstacleCollisionVolume, TriggerVolume};
use super::progress::{DnfReason, RaceProgress};

// ---------------------------------------------------------------------------
// Collision cache
// ---------------------------------------------------------------------------

pub struct GateOpening {
    pub center: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub half_width: f32,
    pub half_height: f32,
}

pub struct ObstacleObb {
    pub center: Vec3,
    pub axes: [Vec3; 3],
    pub half_extents: Vec3,
    pub gate_opening: Option<GateOpening>,
}

#[derive(Resource)]
pub struct ObstacleCollisionCache(pub Vec<ObstacleObb>);

/// Builds the `ObstacleCollisionCache` resource from spawned obstacle entities.
/// Runs every frame until obstacle collision volumes are available, then inserts
/// the resource. Follows the same polling pattern as `build_gate_planes`.
pub fn build_obstacle_collision_cache(
    mut commands: Commands,
    obstacle_query: Query<(Entity, &ObstacleCollisionVolume, &GlobalTransform)>,
    trigger_query: Query<(&TriggerVolume, &GlobalTransform, &ChildOf)>,
) {
    if obstacle_query.is_empty() {
        return;
    }

    let mut obbs = Vec::new();

    for (entity, collision_vol, global_transform) in &obstacle_query {
        let (scale, rotation, translation) = global_transform.to_scale_rotation_translation();

        // World-space center: apply instance transform to the local offset
        let center = translation + rotation * (collision_vol.offset * scale);

        // World-space axes from the rotation matrix
        let axes = [
            rotation * Vec3::X,
            rotation * Vec3::Y,
            rotation * Vec3::Z,
        ];

        // Half-extents scaled by instance scale
        let half_extents = collision_vol.half_extents * scale;

        // For gates, find the child TriggerVolume to build the gate opening
        let gate_opening = if collision_vol.is_gate {
            let mut opening = None;

            for (trigger, trigger_global, child_of) in &trigger_query {
                if child_of.parent() != entity {
                    continue;
                }

                let trigger_center = trigger_global.translation();

                // Extract axes from parent rotation (trigger child has no rotation)
                let right = rotation * Vec3::X;
                let up = rotation * Vec3::Y;

                let half_width = trigger.half_extents.x * scale.x;
                let half_height = trigger.half_extents.y * scale.y;

                // Also need the gate forward for directionality (already stored as component)
                opening = Some(GateOpening {
                    center: trigger_center,
                    right,
                    up,
                    half_width,
                    half_height,
                });
                break; // Only one trigger volume per gate
            }

            opening
        } else {
            None
        };

        obbs.push(ObstacleObb {
            center,
            axes,
            half_extents,
            gate_opening,
        });
    }

    if !obbs.is_empty() {
        commands.insert_resource(ObstacleCollisionCache(obbs));
    }
}

// ---------------------------------------------------------------------------
// Pure collision functions
// ---------------------------------------------------------------------------

/// Swept segment vs OBB intersection using slab method.
/// `expansion` (drone radius) is added to half_extents at test time.
/// Returns the first hit point on the (expanded) OBB surface, or `None`.
pub fn segment_obb_intersection(
    p0: Vec3,
    p1: Vec3,
    obb: &ObstacleObb,
    expansion: f32,
) -> Option<Vec3> {
    let dir = p1 - p0;
    let delta = p0 - obb.center;

    let mut t_min = 0.0_f32;
    let mut t_max = 1.0_f32;

    for i in 0..3 {
        let axis = obb.axes[i];
        let half = obb.half_extents[i] + expansion;

        let e = axis.dot(delta);
        let f = axis.dot(dir);

        if f.abs() > 1e-8 {
            let inv_f = 1.0 / f;
            let mut t1 = (-half - e) * inv_f;
            let mut t2 = (half - e) * inv_f;

            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }

            t_min = t_min.max(t1);
            t_max = t_max.min(t2);

            if t_min > t_max {
                return None;
            }
        } else {
            // Ray is parallel to this slab — check if origin is within
            if (-half - e) > 0.0 || (half - e) < 0.0 {
                // Equivalent: |e| > half
                return None;
            }
        }
    }

    // t_min is in [0, 1] — segment intersects the OBB
    Some(p0 + t_min * dir)
}

/// Returns true if `point` is within the gate opening (infinite depth tube).
/// Projects the offset onto the opening's right and up axes; ignores the
/// depth axis entirely so any approach angle works.
pub fn point_in_gate_opening(point: Vec3, opening: &GateOpening) -> bool {
    let offset = point - opening.center;
    let x = offset.dot(opening.right).abs();
    let y = offset.dot(opening.up).abs();
    x < opening.half_width && y < opening.half_height
}

// ---------------------------------------------------------------------------
// Crash helper (shared by miss_detection and obstacle_collision_check)
// ---------------------------------------------------------------------------

/// Crashes a drone: sets phase to Crashed, zeros dynamics, hides the entity,
/// records the DNF in progress (if provided), and spawns an explosion.
///
/// Used by both `miss_detection` (gate.rs) and `obstacle_collision_check`.
pub fn crash_drone(
    commands: &mut Commands,
    phase: &mut DronePhase,
    dynamics: &mut DroneDynamics,
    visibility: &mut Visibility,
    drone_index: usize,
    position: Vec3,
    crash_velocity: Vec3,
    progress: Option<&mut RaceProgress>,
    explosion_meshes: &ExplosionMeshes,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    explosion_sounds: Option<&ExplosionSounds>,
    reason: DnfReason,
) {
    *phase = DronePhase::Crashed;
    dynamics.velocity = Vec3::ZERO;
    dynamics.angular_velocity = Vec3::ZERO;
    *visibility = Visibility::Hidden;

    if let Some(progress) = progress {
        progress.record_crash(drone_index, reason);
    }

    explosion::spawn_explosion(
        commands,
        explosion_meshes,
        materials,
        position,
        crash_velocity,
        DRONE_COLORS[drone_index],
        explosion_sounds,
    );
}

// ---------------------------------------------------------------------------
// Collision detection system
// ---------------------------------------------------------------------------

const DRONE_COLLISION_RADIUS: f32 = 0.35;

/// Tests each racing/victory-lap drone's swept segment against obstacle OBBs.
/// Gate openings are exempted — a hit inside the opening is a safe pass.
pub fn obstacle_collision_check(
    mut commands: Commands,
    mut progress: Option<ResMut<RaceProgress>>,
    cache: Option<Res<ObstacleCollisionCache>>,
    explosion_meshes: Option<Res<ExplosionMeshes>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    explosion_sounds: Option<Res<ExplosionSounds>>,
    mut drone_query: Query<(
        &Drone,
        &Transform,
        &PreviousTranslation,
        &mut DronePhase,
        &mut DroneDynamics,
        &mut Visibility,
    )>,
) {
    let Some(ref cache) = cache else { return };
    let Some(ref meshes) = explosion_meshes else {
        return;
    };

    for (drone, transform, prev_translation, mut phase, mut dynamics, mut visibility) in
        &mut drone_query
    {
        if *phase != DronePhase::Racing && *phase != DronePhase::VictoryLap {
            continue;
        }

        let p0 = prev_translation.0;
        let p1 = transform.translation;

        for obb in &cache.0 {
            let Some(hit_point) = segment_obb_intersection(p0, p1, obb, DRONE_COLLISION_RADIUS)
            else {
                continue;
            };

            // If this is a gate OBB and the hit is inside the opening, it's a safe pass
            if let Some(ref opening) = obb.gate_opening {
                if point_in_gate_opening(hit_point, opening) {
                    continue;
                }
            }

            let drone_idx = drone.index as usize;
            let crash_velocity = dynamics.velocity;

            crash_drone(
                &mut commands,
                &mut phase,
                &mut dynamics,
                &mut visibility,
                drone_idx,
                transform.translation,
                crash_velocity,
                progress.as_deref_mut(),
                meshes,
                &mut materials,
                explosion_sounds.as_deref(),
                DnfReason::ObstacleCollision,
            );

            warn!(
                "Drone {} CRASHED — obstacle collision at ({:.1}, {:.1}, {:.1})",
                drone.index, hit_point.x, hit_point.y, hit_point.z
            );

            break; // One collision per drone per frame
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helpers ---

    fn axis_aligned_obb(center: Vec3, half_extents: Vec3) -> ObstacleObb {
        ObstacleObb {
            center,
            axes: [Vec3::X, Vec3::Y, Vec3::Z],
            half_extents,
            gate_opening: None,
        }
    }

    fn rotated_obb_y90(center: Vec3, half_extents: Vec3) -> ObstacleObb {
        // 90° rotation around Y: X→Z, Y→Y, Z→−X
        ObstacleObb {
            center,
            axes: [Vec3::Z, Vec3::Y, Vec3::NEG_X],
            half_extents,
            gate_opening: None,
        }
    }

    fn default_opening() -> GateOpening {
        GateOpening {
            center: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 2.0,
            half_height: 1.5,
        }
    }

    // ===================================================================
    // segment_obb_intersection tests
    // ===================================================================

    #[test]
    fn segment_through_center() {
        let obb = axis_aligned_obb(Vec3::ZERO, Vec3::splat(1.0));
        let hit = segment_obb_intersection(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
            &obb,
            0.0,
        );
        assert!(hit.is_some());
        let p = hit.unwrap();
        // Should hit the +Z face at z=1.0
        assert!((p.z - 1.0).abs() < 1e-4, "hit z={}", p.z);
    }

    #[test]
    fn segment_miss() {
        let obb = axis_aligned_obb(Vec3::ZERO, Vec3::splat(1.0));
        let hit = segment_obb_intersection(
            Vec3::new(5.0, 0.0, 5.0),
            Vec3::new(5.0, 0.0, -5.0),
            &obb,
            0.0,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn segment_parallel_inside() {
        // Segment runs along X axis, entirely inside the OBB
        let obb = axis_aligned_obb(Vec3::ZERO, Vec3::splat(5.0));
        let hit = segment_obb_intersection(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            &obb,
            0.0,
        );
        // Starts inside, so t_min=0 → hit at p0
        assert!(hit.is_some());
        let p = hit.unwrap();
        assert!((p - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn segment_parallel_outside() {
        // Segment runs along X axis, outside the OBB on the Y axis
        let obb = axis_aligned_obb(Vec3::ZERO, Vec3::splat(1.0));
        let hit = segment_obb_intersection(
            Vec3::new(-5.0, 3.0, 0.0),
            Vec3::new(5.0, 3.0, 0.0),
            &obb,
            0.0,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn segment_starts_inside() {
        let obb = axis_aligned_obb(Vec3::ZERO, Vec3::splat(2.0));
        let hit = segment_obb_intersection(
            Vec3::new(0.0, 0.0, 0.0), // inside
            Vec3::new(0.0, 0.0, 5.0), // exits
            &obb,
            0.0,
        );
        assert!(hit.is_some());
        let p = hit.unwrap();
        // t_min stays 0, so hit point is at p0
        assert!((p - Vec3::ZERO).length() < 1e-4);
    }

    #[test]
    fn segment_too_short() {
        let obb = axis_aligned_obb(Vec3::new(0.0, 0.0, 10.0), Vec3::splat(1.0));
        // Segment from z=0 to z=2 — doesn't reach the OBB at z=10
        let hit = segment_obb_intersection(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            &obb,
            0.0,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn rotated_obb_hit() {
        // OBB rotated 90° around Y. half_extents = (3, 1, 0.5).
        // After rotation: original X axis → world Z, original Z axis → world −X.
        // So the box extends 3 units along world Z, 1 along Y, 0.5 along world X.
        let obb = rotated_obb_y90(Vec3::ZERO, Vec3::new(3.0, 1.0, 0.5));
        // Fly along X through center — the box is 0.5 wide in world X (from axis[2]=-X, he=0.5)
        // Wait, axes: [Z, Y, -X], half_extents: [3.0, 1.0, 0.5]
        // axis[0]=Z, he[0]=3.0 → extends ±3 along world Z
        // axis[1]=Y, he[1]=1.0 → extends ±1 along world Y
        // axis[2]=-X, he[2]=0.5 → extends ±0.5 along world X
        let hit = segment_obb_intersection(
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            &obb,
            0.0,
        );
        assert!(hit.is_some());
        let p = hit.unwrap();
        assert!((p.x.abs() - 0.5).abs() < 1e-4, "hit x={}", p.x);
    }

    #[test]
    fn rotated_obb_miss() {
        let obb = rotated_obb_y90(Vec3::ZERO, Vec3::new(3.0, 1.0, 0.5));
        // Fly along X but at z=4 — outside the ±3 extent along world Z
        let hit = segment_obb_intersection(
            Vec3::new(-5.0, 0.0, 4.0),
            Vec3::new(5.0, 0.0, 4.0),
            &obb,
            0.0,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn expansion_widens_hit() {
        let obb = axis_aligned_obb(Vec3::ZERO, Vec3::splat(1.0));
        // Segment at x=1.3: misses without expansion (half=1.0), hits with expansion=0.5 (half=1.5)
        let miss = segment_obb_intersection(
            Vec3::new(1.3, 0.0, 5.0),
            Vec3::new(1.3, 0.0, -5.0),
            &obb,
            0.0,
        );
        assert!(miss.is_none());

        let hit = segment_obb_intersection(
            Vec3::new(1.3, 0.0, 5.0),
            Vec3::new(1.3, 0.0, -5.0),
            &obb,
            0.5,
        );
        assert!(hit.is_some());
    }

    #[test]
    fn hit_point_on_surface() {
        let obb = axis_aligned_obb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        let hit = segment_obb_intersection(
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.0, 0.0, -10.0),
            &obb,
            0.0,
        );
        assert!(hit.is_some());
        let p = hit.unwrap();
        // Entry at +Z face → z = 2.0
        assert!((p.z - 2.0).abs() < 1e-4, "hit z={}", p.z);
        assert!(p.x.abs() < 1e-4);
        assert!(p.y.abs() < 1e-4);
    }

    // ===================================================================
    // point_in_gate_opening tests
    // ===================================================================

    #[test]
    fn opening_center() {
        let opening = default_opening();
        assert!(point_in_gate_opening(Vec3::ZERO, &opening));
    }

    #[test]
    fn opening_inside_bounds() {
        let opening = default_opening();
        assert!(point_in_gate_opening(Vec3::new(1.5, 1.0, 0.0), &opening));
    }

    #[test]
    fn opening_outside_width() {
        let opening = default_opening();
        assert!(!point_in_gate_opening(Vec3::new(2.5, 0.0, 0.0), &opening));
    }

    #[test]
    fn opening_outside_height() {
        let opening = default_opening();
        assert!(!point_in_gate_opening(Vec3::new(0.0, 2.0, 0.0), &opening));
    }

    #[test]
    fn opening_different_depth() {
        // Depth along Z should not matter (infinite tube)
        let opening = default_opening();
        assert!(point_in_gate_opening(Vec3::new(0.5, 0.5, 100.0), &opening));
        assert!(point_in_gate_opening(Vec3::new(0.5, 0.5, -50.0), &opening));
    }

    #[test]
    fn opening_rotated_axes() {
        // Opening rotated 90° around Z: right→Y, up→−X
        let opening = GateOpening {
            center: Vec3::ZERO,
            right: Vec3::Y,
            up: Vec3::NEG_X,
            half_width: 2.0,
            half_height: 1.5,
        };
        // Point at (0.5, 1.0, 0) → proj on right(Y)=1.0 (<2), proj on up(-X)=-0.5 → |0.5| (<1.5)
        assert!(point_in_gate_opening(Vec3::new(0.5, 1.0, 0.0), &opening));
        // Point at (0, 3.0, 0) → proj on right(Y)=3.0 (>2) → outside
        assert!(!point_in_gate_opening(Vec3::new(0.0, 3.0, 0.0), &opening));
    }

    // ===================================================================
    // Integration tests: segment + gate opening exemption
    // ===================================================================

    #[test]
    fn segment_through_gate_opening_exempted() {
        // OBB represents the full gate structure (e.g., 5×4×1 box)
        let mut obb = axis_aligned_obb(Vec3::ZERO, Vec3::new(5.0, 4.0, 1.0));
        // Gate opening is 3×2 at center
        obb.gate_opening = Some(GateOpening {
            center: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 1.5,
            half_height: 1.0,
        });

        // Segment flies through center — hits the OBB
        let hit = segment_obb_intersection(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
            &obb,
            0.0,
        );
        assert!(hit.is_some());
        // But hit point is inside the gate opening → should be exempted
        let p = hit.unwrap();
        assert!(point_in_gate_opening(p, obb.gate_opening.as_ref().unwrap()));
    }

    #[test]
    fn segment_through_gate_frame_not_exempted() {
        let mut obb = axis_aligned_obb(Vec3::ZERO, Vec3::new(5.0, 4.0, 1.0));
        obb.gate_opening = Some(GateOpening {
            center: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 1.5,
            half_height: 1.0,
        });

        // Segment at x=3 — hits the OBB (within ±5) but outside the opening (±1.5)
        let hit = segment_obb_intersection(
            Vec3::new(3.0, 0.0, 5.0),
            Vec3::new(3.0, 0.0, -5.0),
            &obb,
            0.0,
        );
        assert!(hit.is_some());
        let p = hit.unwrap();
        assert!(!point_in_gate_opening(p, obb.gate_opening.as_ref().unwrap()));
    }

    #[test]
    fn segment_misses_gate_entirely() {
        let mut obb = axis_aligned_obb(Vec3::ZERO, Vec3::new(5.0, 4.0, 1.0));
        obb.gate_opening = Some(GateOpening {
            center: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            half_width: 1.5,
            half_height: 1.0,
        });

        // Segment far away — misses entirely
        let hit = segment_obb_intersection(
            Vec3::new(10.0, 10.0, 5.0),
            Vec3::new(10.0, 10.0, -5.0),
            &obb,
            0.0,
        );
        assert!(hit.is_none());
    }
}
