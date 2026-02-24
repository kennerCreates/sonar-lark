use bevy::math::cubic_splines::{CubicCardinalSpline, CubicCurve, CyclicCubicGenerator};
use bevy::prelude::*;
use rand::Rng;

use crate::course::data::CourseData;
use crate::obstacle::library::ObstacleLibrary;
use crate::states::AppState;
use super::components::*;

const DRONE_COUNT: u8 = 12;
const START_DISTANCE_BEHIND_GATE: f32 = 5.0;
const HOVER_HEIGHT: f32 = 1.5;
const GRAVITY: f32 = 9.81;
/// Fraction of gate width used for the start line (leaves margin at edges).
const GATE_WIDTH_USAGE: f32 = 0.8;

/// Marker resource inserted when `spawn_drones` detects the course has no gates.
/// Prevents the warning from repeating every frame and signals the UI to show a banner.
#[derive(Resource)]
pub struct NoGatesCourse;

#[derive(Resource)]
pub struct DroneGltfHandle(pub Handle<bevy::gltf::Gltf>);

#[derive(Resource)]
pub struct DroneAssets {
    pub mesh_primitives: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    pub mesh_transform: Transform,
}

pub fn load_drone_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("models/drone.glb");
    commands.insert_resource(DroneGltfHandle(handle));
}

pub fn setup_drone_assets(
    mut commands: Commands,
    gltf_handle: Option<Res<DroneGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing: Option<Res<DroneAssets>>,
) {
    if existing.is_some() {
        return;
    }
    let Some(handle) = gltf_handle else { return };
    let Some(gltf) = gltf_assets.get(&handle.0) else {
        return;
    };

    let red_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.2),
        ..default()
    });

    let node_name: Box<str> = "Drone".into();
    if let Some(node_handle) = gltf.named_nodes.get(&node_name) {
        if let Some(node) = node_assets.get(node_handle) {
            if let Some(gltf_mesh_handle) = node.mesh.as_ref() {
                if let Some(gltf_mesh) = mesh_assets.get(gltf_mesh_handle) {
                    let primitives: Vec<(Handle<Mesh>, Handle<StandardMaterial>)> = gltf_mesh
                        .primitives
                        .iter()
                        .map(|p| (p.mesh.clone(), red_mat.clone()))
                        .collect();

                    if !primitives.is_empty() {
                        commands.insert_resource(DroneAssets {
                            mesh_primitives: primitives,
                            mesh_transform: node.transform,
                        });
                        return;
                    }
                }
            }
        }
    }

    // Fallback: placeholder red cube if glTF node is missing or empty
    let mesh = meshes.add(Cuboid::new(0.5, 0.3, 0.5));
    commands.insert_resource(DroneAssets {
        mesh_primitives: vec![(mesh, red_mat)],
        mesh_transform: Transform::IDENTITY,
    });
}

pub fn spawn_drones(
    mut commands: Commands,
    drone_assets: Option<Res<DroneAssets>>,
    course: Option<Res<CourseData>>,
    library: Res<ObstacleLibrary>,
    existing_drones: Query<(), With<Drone>>,
    no_gates: Option<Res<NoGatesCourse>>,
) {
    if !existing_drones.is_empty() || no_gates.is_some() {
        return;
    }
    let Some(assets) = drone_assets else { return };
    let Some(course) = course else { return };

    let Some(race_path) = generate_race_path(&course, &library) else {
        warn!("Not enough gates for a race path — drones will not spawn");
        commands.insert_resource(NoGatesCourse);
        return;
    };

    let first_gate_inst = course
        .instances
        .iter()
        .filter_map(|inst| inst.gate_order.map(|order| (order, inst)))
        .min_by_key(|(order, _)| *order)
        .map(|(_, inst)| inst);
    let Some(first_gate) = first_gate_inst else {
        warn!("No gate instances found in course — drones will not spawn");
        commands.insert_resource(NoGatesCourse);
        return;
    };
    let gate_half_width = library
        .get(&first_gate.obstacle_id)
        .and_then(|def| def.trigger_volume.as_ref())
        .map(|tv| tv.half_extents.x)
        .unwrap_or(5.0);
    let start_positions = compute_start_positions(
        first_gate.translation,
        first_gate.rotation,
        gate_half_width,
        race_path.gate_forwards[0],
        DRONE_COUNT,
    );
    let mut rng = rand::thread_rng();

    for i in 0..DRONE_COUNT {
        let config = randomize_drone_config(&mut rng);
        let pid = create_pid_with_variation(&config);
        let position = start_positions[i as usize];

        let look_dir =
            (race_path.gate_positions[0] - position).normalize_or(Vec3::NEG_Z);
        let flat_dir =
            Vec3::new(look_dir.x, 0.0, look_dir.z).normalize_or(Vec3::NEG_Z);
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat_dir);

        let transform = Transform::from_translation(position).with_rotation(rotation);

        let mut dynamics = DroneDynamics::default();
        let hover_thrust = GRAVITY * dynamics.mass;
        dynamics.thrust = hover_thrust;
        dynamics.commanded_thrust = hover_thrust;

        let gate_count = race_path.gate_positions.len() as u32;
        let mut entity_cmd = commands.spawn((
            transform,
            Visibility::default(),
            Drone { index: i },
            pid,
            AttitudePd::default(),
            dynamics,
            config,
            AIController {
                target_gate_index: 0,
                gate_count,
                spline: race_path.spline.clone(),
                spline_t: 0.0,
                gate_positions: race_path.gate_positions.clone(),
                gate_forwards: race_path.gate_forwards.clone(),
            },
            DesiredPosition {
                position,
                velocity_hint: look_dir,
            },
            DesiredAttitude {
                orientation: rotation,
                thrust_magnitude: hover_thrust,
            },
            DroneStartPosition {
                translation: position,
                rotation,
            },
            DespawnOnExit(AppState::Race),
        ));

        entity_cmd.with_children(|children| {
            for (mesh, material) in &assets.mesh_primitives {
                children.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    assets.mesh_transform,
                ));
            }
        });
    }

    info!(
        "Spawned {} drones on {}-gate spline course",
        DRONE_COUNT,
        race_path.gate_positions.len()
    );
}

pub fn cleanup_drone_resources(mut commands: Commands) {
    commands.remove_resource::<DroneAssets>();
    commands.remove_resource::<DroneGltfHandle>();
    commands.remove_resource::<NoGatesCourse>();
}

// --- Pure helper functions (testable) ---

pub struct RacePath {
    pub spline: CubicCurve<Vec3>,
    pub gate_positions: Vec<Vec3>,
    pub gate_forwards: Vec<Vec3>,
}

pub fn generate_race_path(course: &CourseData, library: &ObstacleLibrary) -> Option<RacePath> {
    let mut gates: Vec<(u32, Vec3, Vec3)> = course
        .instances
        .iter()
        .filter_map(|inst| {
            inst.gate_order.map(|order| {
                let tv = library
                    .get(&inst.obstacle_id)
                    .and_then(|def| def.trigger_volume.as_ref());
                // Use the trigger volume center as the fly-through target,
                // not the obstacle origin (which is at ground level).
                let fly_through_offset = tv
                    .map(|tv| inst.rotation * tv.offset)
                    .unwrap_or(Vec3::ZERO);
                let local_fwd = tv.map(|tv| tv.forward).unwrap_or(Vec3::NEG_Z);
                let world_fwd = inst.rotation
                    * if inst.gate_forward_flipped { -local_fwd } else { local_fwd };
                (order, inst.translation + fly_through_offset, world_fwd)
            })
        })
        .collect();
    gates.sort_by_key(|(order, _, _)| *order);
    let gate_positions: Vec<Vec3> = gates.iter().map(|(_, pos, _)| *pos).collect();
    let gate_forwards: Vec<Vec3> = gates.iter().map(|(_, _, fwd)| *fwd).collect();

    if gate_positions.len() < 2 {
        return None;
    }

    // Build approach / departure waypoints per gate so the Catmull-Rom
    // spline approaches each gate from the correct side.  No center point —
    // the spline sweeps smoothly through the gate region between approach
    // and departure, allowing diagonal lines when gates aren't aligned.
    const APPROACH_OFFSET: f32 = 8.0;
    let mut control_points = Vec::with_capacity(gate_positions.len() * 2);
    for (pos, fwd) in gate_positions.iter().zip(gate_forwards.iter()) {
        control_points.push(*pos - *fwd * APPROACH_OFFSET);
        control_points.push(*pos + *fwd * APPROACH_OFFSET);
    }

    let spline = CubicCardinalSpline::new_catmull_rom(control_points.iter().copied())
        .to_curve_cyclic()
        .ok()?;

    Some(RacePath { spline, gate_positions, gate_forwards })
}

pub fn compute_start_positions(
    gate_translation: Vec3,
    gate_rotation: Quat,
    gate_half_width: f32,
    gate_forward: Vec3,
    count: u8,
) -> Vec<Vec3> {
    // Use the explicit gate forward direction (projected to XZ plane).
    let through_dir = Vec3::new(gate_forward.x, 0.0, gate_forward.z).normalize_or(Vec3::NEG_Z);

    // Start line center: behind the gate at ground level
    let start_center = Vec3::new(gate_translation.x, 0.0, gate_translation.z)
        - through_dir * START_DISTANCE_BEHIND_GATE;

    // Gate's local X axis projected to XZ = lateral spread direction
    let gate_x = gate_rotation * Vec3::X;
    let lateral = Vec3::new(gate_x.x, 0.0, gate_x.z).normalize_or(Vec3::X);

    // Fit drones within a fraction of the gate width
    let usable_width = gate_half_width * 2.0 * GATE_WIDTH_USAGE;
    let spacing = if count > 1 {
        usable_width / (count as f32 - 1.0)
    } else {
        0.0
    };

    let center_offset = (count as f32 - 1.0) / 2.0;
    let mut positions = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let col_offset = (i as f32 - center_offset) * spacing;
        let pos = start_center + lateral * col_offset + Vec3::Y * HOVER_HEIGHT;
        positions.push(pos);
    }
    positions
}

fn randomize_drone_config(rng: &mut impl Rng) -> DroneConfig {
    DroneConfig {
        pid_variation: Vec3::new(
            rng.gen_range(-0.15..=0.15),
            rng.gen_range(-0.15..=0.15),
            rng.gen_range(-0.15..=0.15),
        ),
        line_offset: rng.gen_range(-1.5..=1.5),
        noise_amplitude: rng.gen_range(0.3..=1.5),
        noise_frequency: rng.gen_range(0.5..=2.0),
        hover_noise_amp: Vec3::new(
            rng.gen_range(0.05..=0.15),
            rng.gen_range(0.02..=0.06),
            rng.gen_range(0.05..=0.12),
        ),
        hover_noise_freq: Vec3::new(
            rng.gen_range(0.1..=0.5),
            rng.gen_range(0.15..=0.5),
            rng.gen_range(0.1..=0.4),
        ),
    }
}

fn create_pid_with_variation(config: &DroneConfig) -> PositionPid {
    let base = PositionPid::default();
    PositionPid {
        kp: base.kp * (Vec3::ONE + config.pid_variation),
        ki: base.ki * (Vec3::ONE + config.pid_variation),
        kd: base.kd * (Vec3::ONE + config.pid_variation),
        integral: Vec3::ZERO,
        prev_error: Vec3::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::data::ObstacleInstance;
    use crate::obstacle::definition::{ObstacleId, ObstacleDef, TriggerVolumeConfig};
    use crate::obstacle::library::ObstacleLibrary;
    use bevy::math::{Quat, Vec3};

    fn gate_instance(translation: Vec3, order: u32) -> ObstacleInstance {
        ObstacleInstance {
            obstacle_id: ObstacleId("gate".to_string()),
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gate_order: Some(order),
            gate_forward_flipped: false,
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
        };
        assert!(generate_race_path(&course, &lib).is_none());
    }

    #[test]
    fn race_path_empty_course_returns_none() {
        let lib = ObstacleLibrary::default();
        let course = CourseData {
            name: "Empty".to_string(),
            instances: vec![],
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
        };

        let path = generate_race_path(&course, &lib).expect("4 gates should produce a path");
        // Gate 0 is flipped: forward is +Z. Midpoint at t = 0 * 2 + 0.5 = 0.5.
        let tangent0 = path.spline.velocity(0.5).normalize();
        assert!(
            tangent0.dot(Vec3::Z) > 0.7,
            "flipped gate tangent should point roughly +Z, got {:?}",
            tangent0
        );
        // Gate 1 is NOT flipped: forward is -Z. Midpoint at t = 1 * 2 + 0.5 = 2.5.
        let tangent1 = path.spline.velocity(2.5).normalize();
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
        };

        let path = generate_race_path(&course, &lib).expect("2 gates should produce a path");
        // Rotated 90° around Y: NEG_Z → NEG_X
        assert!((path.gate_forwards[0].x - (-1.0)).abs() < 0.01, "expected NEG_X forward, got {:?}", path.gate_forwards[0]);
        assert!(path.gate_forwards[0].z.abs() < 0.01, "Z should be ~0 after 90° Y rotation");
    }

    #[test]
    fn compute_start_positions_correct_count() {
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            5.0,
            Vec3::NEG_Z,
            12,
        );
        assert_eq!(positions.len(), 12);
    }

    #[test]
    fn compute_start_positions_behind_first_gate() {
        // Gate at origin, identity rotation, next gate at -Z.
        // Through direction is -Z, so drones should be at +Z (behind).
        let gate_pos = Vec3::ZERO;
        let positions = compute_start_positions(
            gate_pos,
            Quat::IDENTITY,
            5.0,
            Vec3::NEG_Z,
            12,
        );

        for pos in &positions {
            assert!(
                pos.z > gate_pos.z,
                "drone at z={} should be behind gate at z={}",
                pos.z,
                gate_pos.z
            );
        }
    }

    #[test]
    fn compute_start_positions_at_hover_height() {
        let positions = compute_start_positions(
            Vec3::new(0.0, 10.0, 0.0), // elevated gate
            Quat::IDENTITY,
            5.0,
            Vec3::NEG_Z,
            4,
        );

        for pos in &positions {
            assert!(
                (pos.y - HOVER_HEIGHT).abs() < 0.01,
                "drone at y={} should be at HOVER_HEIGHT={}, not gate elevation",
                pos.y,
                HOVER_HEIGHT,
            );
        }
    }

    #[test]
    fn compute_start_positions_fits_within_gate_width() {
        let half_width = 5.0;
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            half_width,
            Vec3::NEG_Z,
            12,
        );

        // With identity rotation, lateral axis is X
        let usable = half_width * 2.0 * GATE_WIDTH_USAGE;
        for pos in &positions {
            assert!(
                pos.x.abs() <= usable / 2.0 + 0.01,
                "drone at x={} exceeds usable width {}",
                pos.x,
                usable,
            );
        }
    }

    #[test]
    fn compute_start_positions_no_overlap() {
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            8.0,
            Vec3::NEG_Z,
            12,
        );

        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let dist = (positions[i] - positions[j]).length();
                assert!(dist > 0.5, "drones {} and {} too close: {:.2}", i, j, dist);
            }
        }
    }

    #[test]
    fn compute_start_positions_respects_gate_rotation() {
        // Gate rotated 90 degrees around Y — lateral axis becomes world Z
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let positions = compute_start_positions(
            Vec3::ZERO,
            rotation,
            5.0,
            Vec3::NEG_X,
            3,
        );

        // All drones should share the same X (spread along Z, not X)
        let first_x = positions[0].x;
        for pos in &positions {
            assert!(
                (pos.x - first_x).abs() < 0.01,
                "expected drones aligned on X, got x={}",
                pos.x,
            );
        }
        // But Z should differ
        assert!(
            (positions[0].z - positions[2].z).abs() > 1.0,
            "drones should be spread along Z"
        );
    }

    #[test]
    fn randomize_drone_config_within_bounds() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let config = randomize_drone_config(&mut rng);
            assert!(config.pid_variation.x.abs() <= 0.15);
            assert!(config.pid_variation.y.abs() <= 0.15);
            assert!(config.pid_variation.z.abs() <= 0.15);
            assert!(config.line_offset.abs() <= 1.5);
            assert!((0.3..=1.5).contains(&config.noise_amplitude));
            assert!((0.5..=2.0).contains(&config.noise_frequency));
            assert!((0.05..=0.15).contains(&config.hover_noise_amp.x));
            assert!((0.02..=0.06).contains(&config.hover_noise_amp.y));
            assert!((0.05..=0.12).contains(&config.hover_noise_amp.z));
            assert!((0.1..=0.5).contains(&config.hover_noise_freq.x));
            assert!((0.15..=0.5).contains(&config.hover_noise_freq.y));
            assert!((0.1..=0.4).contains(&config.hover_noise_freq.z));
        }
    }

    #[test]
    fn create_pid_with_variation_applies_correctly() {
        let config = DroneConfig {
            pid_variation: Vec3::new(0.1, -0.1, 0.05),
            line_offset: 0.0,
            noise_amplitude: 1.0,
            noise_frequency: 1.0,
            hover_noise_amp: Vec3::splat(0.02),
            hover_noise_freq: Vec3::splat(1.0),
        };
        let pid = create_pid_with_variation(&config);
        let base = PositionPid::default();

        assert!((pid.kp.x - base.kp.x * 1.1).abs() < 0.001);
        assert!((pid.kp.y - base.kp.y * 0.9).abs() < 0.001);
        assert!((pid.kp.z - base.kp.z * 1.05).abs() < 0.001);
    }
}
