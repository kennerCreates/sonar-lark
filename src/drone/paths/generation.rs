use bevy::math::cubic_splines::{CubicCardinalSpline, CyclicCubicGenerator};
use bevy::prelude::*;

use crate::course::data::CourseData;
use crate::obstacle::library::ObstacleLibrary;
use super::super::components::*;
use super::{RacePath, extract_sorted_gates};

const MAX_APPROACH_OFFSET: f32 = 12.0;
const APPROACH_FRACTION: f32 = 0.3;

/// Compute the approach/departure offset for a gate based on distance to the next gate.
/// Scales linearly with inter-gate distance, capped at MAX_APPROACH_OFFSET.
pub fn adaptive_approach_offset(gate_distance: f32) -> f32 {
    (gate_distance * APPROACH_FRACTION).min(MAX_APPROACH_OFFSET)
}

pub fn generate_race_path(course: &CourseData, library: &ObstacleLibrary) -> Option<RacePath> {
    let (gate_positions, gate_forwards, _gate_extents) = extract_sorted_gates(course, library);

    if gate_positions.len() < 2 {
        return None;
    }

    // Build approach / departure waypoints per gate, plus a midleg waypoint
    // between consecutive gates. The midleg breaks each inter-gate transition
    // into two spline segments, distributing the turn across a longer arc and
    // significantly reducing peak curvature (= higher cornering speed).
    // 3 control points per gate: approach, departure, midleg-to-next.
    // Approach offset scales with inter-gate distance so tight courses don't
    // waste all their space on straight committed-direction segments.
    let n = gate_positions.len();
    let mut control_points = Vec::with_capacity(n * 3);
    for i in 0..n {
        let pos = gate_positions[i];
        let fwd = gate_forwards[i];
        let next = (i + 1) % n;
        let gate_dist = (gate_positions[next] - pos).length();
        let approach_offset = adaptive_approach_offset(gate_dist);
        let approach = pos - fwd * approach_offset;
        let departure = pos + fwd * approach_offset;
        control_points.push(approach);
        control_points.push(departure);

        // Midleg: halfway between this gate's departure and next gate's approach.
        let next_gate_dist = (gate_positions[(next + 1) % n] - gate_positions[next]).length();
        let next_offset = adaptive_approach_offset(next_gate_dist);
        let next_approach = gate_positions[next] - gate_forwards[next] * next_offset;
        control_points.push((departure + next_approach) * 0.5);
    }

    let spline = CubicCardinalSpline::new_catmull_rom(control_points.iter().copied())
        .to_curve_cyclic()
        .ok()?;

    Some(RacePath { spline, gate_positions, gate_forwards })
}

/// Generate a per-drone unique race path by perturbing control points based on
/// the drone's config and index. Gate positions and forwards remain unchanged
/// (they represent actual gate centers for validation). Only the spline differs.
pub fn generate_drone_race_path(
    course: &CourseData,
    library: &ObstacleLibrary,
    config: &DroneConfig,
    drone_index: u8,
    race_seed: u32,
) -> Option<RacePath> {
    let (gate_positions, gate_forwards, gate_extents) = extract_sorted_gates(course, library);

    if gate_positions.len() < 2 {
        return None;
    }

    let n = gate_positions.len();
    let mut control_points = Vec::with_capacity(n * 3);
    // Per-drone offset gate positions for AI fallback distance checks
    let mut drone_gate_positions = Vec::with_capacity(n);

    // Helper: compute a deterministic 2D offset within a gate's opening for this drone.
    // Returns a world-space Vec3 offset from gate center.
    let gate_2d_offset = |gate_idx: usize, fwd: Vec3, extents: Vec2| -> Vec3 {
        let gate_right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
        // Horizontal hash (race_seed mixed in so directions change each race)
        let h_hash = (drone_index as u32)
            .wrapping_mul(1640531527)
            .wrapping_add((gate_idx as u32).wrapping_mul(2891336453))
            .wrapping_add(race_seed.wrapping_mul(2654435761))
            >> 16;
        let h_sign = (h_hash & 0xFFFF) as f32 / 65536.0 * 2.0 - 1.0;
        // Vertical hash (different prime seeds)
        let v_hash = (drone_index as u32)
            .wrapping_mul(2246822519)
            .wrapping_add((gate_idx as u32).wrapping_mul(1640531527))
            .wrapping_add(race_seed.wrapping_mul(1503267967))
            >> 16;
        let v_sign = (v_hash & 0xFFFF) as f32 / 65536.0 * 2.0 - 1.0;
        let h_offset = h_sign * extents.x * config.gate_pass_offset;
        let v_offset = v_sign * extents.y * config.gate_pass_offset;
        gate_right * h_offset + Vec3::Y * v_offset
    };

    for i in 0..n {
        let pos = gate_positions[i];
        let fwd = gate_forwards[i];
        let next = (i + 1) % n;
        let gate_dist = (gate_positions[next] - pos).length();

        // Per-drone 2D offset within the gate opening (width + height)
        let offset = gate_2d_offset(i, fwd, gate_extents[i]);
        let offset_pos = pos + offset;
        drone_gate_positions.push(offset_pos);

        // Per-drone approach offset scaling
        let approach_offset = adaptive_approach_offset(gate_dist) * config.approach_offset_scale;

        let approach = offset_pos - fwd * approach_offset;
        let departure = offset_pos + fwd * approach_offset;
        control_points.push(approach);
        control_points.push(departure);

        // Midleg waypoint with per-drone lateral shift
        let next_gate_dist = (gate_positions[(next + 1) % n] - gate_positions[next]).length();
        let next_offset =
            adaptive_approach_offset(next_gate_dist) * config.approach_offset_scale;
        // Next gate's offset position for midleg calculation
        let next_gate_offset = gate_2d_offset(next, gate_forwards[next], gate_extents[next]);
        let next_approach = gate_positions[next] + next_gate_offset - gate_forwards[next] * next_offset;
        let base_midleg = (departure + next_approach) * 0.5;

        // Deterministic per-drone-per-leg hash for shift direction (seed mixed in)
        let hash = (drone_index as u32)
            .wrapping_mul(2654435761)
            .wrapping_add((i as u32).wrapping_mul(2246822519))
            .wrapping_add(race_seed.wrapping_mul(3266489917))
            >> 16;
        let hash_f = (hash & 0xFFFF) as f32 / 65536.0;
        let sign = hash_f * 2.0 - 1.0; // -1.0..1.0

        // Lateral direction perpendicular to the leg and world up
        let leg_dir = (next_approach - departure).normalize_or(Vec3::Z);
        let leg_lateral = Vec3::Y.cross(leg_dir).normalize_or(Vec3::X);
        let shift = config.racing_line_bias * sign;
        let midleg = base_midleg + leg_lateral * shift;

        control_points.push(midleg);
    }

    let spline = CubicCardinalSpline::new_catmull_rom(control_points.iter().copied())
        .to_curve_cyclic()
        .ok()?;

    Some(RacePath { spline, gate_positions: drone_gate_positions, gate_forwards })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::POINTS_PER_GATE;
    use crate::course::data::ObstacleInstance;
    use crate::obstacle::definition::{ObstacleId, ObstacleDef, TriggerVolumeConfig};
    use crate::obstacle::library::ObstacleLibrary;
    use bevy::math::{Quat, Vec3};

    fn neutral_drone_config() -> DroneConfig {
        DroneConfig {
            pid_variation: Vec3::ZERO,
            line_offset: 0.0,
            noise_amplitude: 1.0,
            noise_frequency: 1.0,
            hover_noise_amp: Vec3::splat(0.1),
            hover_noise_freq: Vec3::splat(0.3),
            cornering_aggression: 1.0,
            braking_distance: 1.0,
            attitude_kp_mult: 1.0,
            attitude_kd_mult: 1.0,
            racing_line_bias: 0.0,
            approach_offset_scale: 1.0,
            gate_pass_offset: 0.0,
        }
    }

    fn gate_instance(translation: Vec3, order: u32) -> ObstacleInstance {
        ObstacleInstance {
            obstacle_id: ObstacleId("gate".to_string()),
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gate_order: Some(order),
            gate_forward_flipped: false,
            camera: None,
        }
    }

    fn wall_instance(translation: Vec3) -> ObstacleInstance {
        ObstacleInstance {
            obstacle_id: ObstacleId("wall".to_string()),
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gate_order: None,
            gate_forward_flipped: false,
            camera: None,
        }
    }

    fn library_with_gate() -> ObstacleLibrary {
        let mut lib = ObstacleLibrary::default();
        lib.insert(ObstacleDef {
            id: ObstacleId("gate".to_string()),
            glb_node_name: "gate".to_string(),
            trigger_volume: Some(TriggerVolumeConfig {
                offset: Vec3::new(0.0, 5.0, 0.0),
                half_extents: Vec3::new(3.0, 3.0, 0.5),
                forward: Vec3::NEG_Z,
            }),
            is_gate: true,
            model_offset: Vec3::ZERO,
            model_rotation: Quat::IDENTITY,
            collision_volume: None,
        });
        lib
    }

    #[test]
    fn race_path_sorts_by_gate_order() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(10.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(5.0, 0.0, 0.0), 1),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("3 gates should produce a path");
        assert_eq!(path.gate_positions.len(), 3);
        // Trigger volume offset adds Y=5.0
        assert_eq!(path.gate_positions[0], Vec3::new(0.0, 5.0, 0.0));
        assert_eq!(path.gate_positions[1], Vec3::new(5.0, 5.0, 0.0));
        assert_eq!(path.gate_positions[2], Vec3::new(10.0, 5.0, 0.0));
    }

    #[test]
    fn race_path_excludes_non_gates() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                wall_instance(Vec3::ZERO),
                gate_instance(Vec3::new(1.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(10.0, 0.0, 0.0), 1),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("2 gates should produce a path");
        // Only gates appear in gate_positions, not walls
        assert_eq!(path.gate_positions.len(), 2);
    }

    #[test]
    fn race_path_single_gate_returns_none() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![gate_instance(Vec3::new(1.0, 0.0, 0.0), 0)],
            props: vec![],
            cameras: vec![],
        };
        assert!(generate_race_path(&course, &lib).is_none());
    }

    #[test]
    fn race_path_empty_course_returns_none() {
        let lib = ObstacleLibrary::default();
        let course = CourseData {
            name: "Empty".to_string(),
            instances: vec![],
            props: vec![],
            cameras: vec![],
        };
        assert!(generate_race_path(&course, &lib).is_none());
    }

    #[test]
    fn race_path_applies_rotation_to_offset() {
        let lib = library_with_gate();
        let mut inst0 = gate_instance(Vec3::new(10.0, 0.0, 0.0), 0);
        inst0.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                inst0,
                gate_instance(Vec3::new(20.0, 0.0, 0.0), 1),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("2 gates should produce a path");
        // Y offset is along Y axis, unaffected by Y-axis rotation
        assert!((path.gate_positions[0].y - 5.0).abs() < 0.001);
    }

    #[test]
    fn race_path_spline_passes_near_gates() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("4 gates should produce a path");
        // With 2 points per gate (approach + departure, no center), the spline
        // passes through control points but sweeps near (not exactly through)
        // the gate center.  The midpoint in spline space is i * PPG + 0.5.
        for (i, gate_pos) in path.gate_positions.iter().enumerate() {
            let mid_t = i as f32 * POINTS_PER_GATE + 0.5;
            let spline_pos = path.spline.position(mid_t);
            let dist = (spline_pos - *gate_pos).length();
            assert!(
                dist < 3.0,
                "spline midpoint at t={} should pass near gate {}: spline={:?}, gate={:?}, dist={}",
                mid_t, i, spline_pos, gate_pos, dist
            );
        }
    }

    #[test]
    fn race_path_tangent_nonzero() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("3 gates should produce a path");
        let total_t = path.gate_positions.len() as f32 * POINTS_PER_GATE;
        for i in 0..30 {
            let t = (i as f32 / 30.0) * total_t;
            let vel = path.spline.velocity(t);
            assert!(
                vel.length() > 0.001,
                "tangent at t={} should be nonzero, got {:?}",
                t, vel
            );
        }
    }

    #[test]
    fn race_path_tangent_aligns_with_gate_forward() {
        let lib = library_with_gate();
        // 4 gates in a square — each with default NEG_Z forward
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("4 gates should produce a path");
        // Gate midpoint in spline space is at i * PPG + 0.5. The tangent
        // there should have a strong component along the gate forward direction.
        for (i, fwd) in path.gate_forwards.iter().enumerate() {
            let mid_t = i as f32 * POINTS_PER_GATE + 0.5;
            let tangent = path.spline.velocity(mid_t).normalize();
            let dot = tangent.dot(*fwd);
            assert!(
                dot > 0.7,
                "spline tangent at gate {} (t={}) should roughly align with gate forward: dot={}, tangent={:?}, forward={:?}",
                i, mid_t, dot, tangent, fwd
            );
        }
    }

    #[test]
    fn race_path_flipped_gate_reverses_tangent() {
        let lib = library_with_gate();
        let mut flipped = gate_instance(Vec3::new(0.0, 0.0, 0.0), 0);
        flipped.gate_forward_flipped = true;
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                flipped,
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("4 gates should produce a path");
        // Gate 0 is flipped: forward is +Z. Midpoint at t = 0 * PPG + 0.5.
        let mid0 = 0.0 * POINTS_PER_GATE + 0.5;
        let tangent0 = path.spline.velocity(mid0).normalize();
        assert!(
            tangent0.dot(Vec3::Z) > 0.7,
            "flipped gate tangent should point roughly +Z, got {:?}",
            tangent0
        );
        // Gate 1 is NOT flipped: forward is -Z. Midpoint at t = 1 * PPG + 0.5.
        let mid1 = 1.0 * POINTS_PER_GATE + 0.5;
        let tangent1 = path.spline.velocity(mid1).normalize();
        assert!(
            tangent1.dot(Vec3::NEG_Z) > 0.7,
            "non-flipped gate tangent should point roughly -Z, got {:?}",
            tangent1
        );
    }

    #[test]
    fn race_path_returns_gate_forwards() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 0.0), 1),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("2 gates should produce a path");
        assert_eq!(path.gate_forwards.len(), 2);
        // Identity rotation + NEG_Z forward = NEG_Z world forward
        for fwd in &path.gate_forwards {
            assert!((fwd.z - (-1.0)).abs() < 0.001, "expected NEG_Z forward, got {:?}", fwd);
        }
    }

    #[test]
    fn race_path_flipped_gate_negates_forward() {
        let lib = library_with_gate();
        let mut inst = gate_instance(Vec3::new(0.0, 0.0, 0.0), 0);
        inst.gate_forward_flipped = true;
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                inst,
                gate_instance(Vec3::new(20.0, 0.0, 0.0), 1),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("2 gates should produce a path");
        // Flipped gate should have +Z forward
        assert!((path.gate_forwards[0].z - 1.0).abs() < 0.001, "expected +Z forward for flipped gate");
        // Non-flipped gate should have -Z forward
        assert!((path.gate_forwards[1].z - (-1.0)).abs() < 0.001, "expected -Z forward for non-flipped gate");
    }

    #[test]
    fn race_path_rotation_applied_to_forward() {
        let lib = library_with_gate();
        let mut inst = gate_instance(Vec3::new(0.0, 0.0, 0.0), 0);
        // 90 degree rotation around Y: NEG_Z becomes NEG_X
        inst.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                inst,
                gate_instance(Vec3::new(20.0, 0.0, 0.0), 1),
            ],
            props: vec![],
            cameras: vec![],
        };

        let path = generate_race_path(&course, &lib).expect("2 gates should produce a path");
        // Rotated 90° around Y: NEG_Z → NEG_X
        assert!((path.gate_forwards[0].x - (-1.0)).abs() < 0.01, "expected NEG_X forward, got {:?}", path.gate_forwards[0]);
        assert!(path.gate_forwards[0].z.abs() < 0.01, "Z should be ~0 after 90° Y rotation");
    }

    #[test]
    fn drone_race_paths_differ_per_config() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        let config_a = DroneConfig {
            racing_line_bias: 3.0,
            approach_offset_scale: 0.9,
            cornering_aggression: 1.2,
            ..neutral_drone_config()
        };
        let config_b = DroneConfig {
            racing_line_bias: -3.0,
            approach_offset_scale: 1.1,
            cornering_aggression: 0.8,
            ..neutral_drone_config()
        };

        let path_a = generate_drone_race_path(&course, &lib, &config_a, 0, 0).unwrap();
        let path_b = generate_drone_race_path(&course, &lib, &config_b, 1, 0).unwrap();

        // Sample at midleg points (between gates) — these should differ most
        let mut any_differ = false;
        for i in 0..4 {
            let midleg_t = i as f32 * POINTS_PER_GATE + 2.5;
            let pos_a = path_a.spline.position(midleg_t);
            let pos_b = path_b.spline.position(midleg_t);
            if (pos_a - pos_b).length() > 1.0 {
                any_differ = true;
                break;
            }
        }
        assert!(any_differ, "different drone configs should produce visibly different splines");
    }

    #[test]
    fn drone_race_path_passes_near_gates() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        let config = DroneConfig {
            racing_line_bias: 4.0,
            approach_offset_scale: 0.9,
            ..neutral_drone_config()
        };

        let path = generate_drone_race_path(&course, &lib, &config, 7, 0).unwrap();

        for (i, gate_pos) in path.gate_positions.iter().enumerate() {
            let mid_t = i as f32 * POINTS_PER_GATE + 0.5;
            let spline_pos = path.spline.position(mid_t);
            let dist = (spline_pos - *gate_pos).length();
            assert!(
                dist < 3.0,
                "per-drone spline at gate {} should pass near gate center: dist={}",
                i, dist
            );
        }
    }

    #[test]
    fn drone_race_path_tangent_aligns_with_gate_forward() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        let config = DroneConfig {
            racing_line_bias: 3.5,
            approach_offset_scale: 0.9,
            ..neutral_drone_config()
        };

        let path = generate_drone_race_path(&course, &lib, &config, 3, 0).unwrap();

        for (i, fwd) in path.gate_forwards.iter().enumerate() {
            let mid_t = i as f32 * POINTS_PER_GATE + 0.5;
            let tangent = path.spline.velocity(mid_t).normalize();
            let dot = tangent.dot(*fwd);
            assert!(
                dot > 0.5,
                "per-drone spline tangent at gate {} should roughly align with forward: dot={}",
                i, dot
            );
        }
    }

    #[test]
    fn drone_race_path_gate_offset_spreads_2d() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(20.0, 0.0, -20.0), 3),
            ],
            props: vec![],
            cameras: vec![],
        };

        // Generate paths for several drones with gate offset enabled
        let config = DroneConfig {
            gate_pass_offset: 0.5,
            ..neutral_drone_config()
        };

        // Collect gate 0 positions across 12 drones
        let base_path = generate_race_path(&course, &lib).unwrap();
        let gate0_center = base_path.gate_positions[0];
        let mut max_horizontal = 0.0_f32;
        let mut max_vertical = 0.0_f32;

        for idx in 0..12u8 {
            let path = generate_drone_race_path(&course, &lib, &config, idx, 0).unwrap();
            let delta = path.gate_positions[0] - gate0_center;
            // Gate 0 forward is NEG_Z; lateral is X, vertical is Y
            max_horizontal = max_horizontal.max(delta.x.abs());
            max_vertical = max_vertical.max(delta.y.abs());
        }

        // With half_extents (3.0, 3.0) and offset fraction 0.5,
        // max possible offset is 1.5m. Across 12 drones we should see
        // meaningful spread in both dimensions.
        assert!(
            max_horizontal > 0.3,
            "drones should spread horizontally at gate: max_h={}",
            max_horizontal
        );
        assert!(
            max_vertical > 0.3,
            "drones should spread vertically at gate: max_v={}",
            max_vertical
        );
    }

    #[test]
    fn drone_race_path_neutral_matches_base() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(20.0, 0.0, 20.0), 1),
                gate_instance(Vec3::new(40.0, 0.0, 0.0), 2),
            ],
            props: vec![],
            cameras: vec![],
        };

        let base = generate_race_path(&course, &lib).unwrap();
        let drone = generate_drone_race_path(&course, &lib, &neutral_drone_config(), 0, 0).unwrap();

        // With neutral config (bias=0, scale=1.0), splines should be identical
        let total_t = 3.0 * POINTS_PER_GATE;
        for i in 0..30 {
            let t = (i as f32 / 30.0) * total_t;
            let base_pos = base.spline.position(t);
            let drone_pos = drone.spline.position(t);
            let dist = (base_pos - drone_pos).length();
            assert!(
                dist < 0.01,
                "neutral drone path should match base at t={}: dist={}",
                t, dist
            );
        }
    }
}
