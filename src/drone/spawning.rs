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

    // TODO: restore glTF drone model once visibility issue is resolved
    {
        let mesh = meshes.add(Cuboid::new(0.5, 0.3, 0.5));
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            ..default()
        });
        commands.insert_resource(DroneAssets {
            mesh_primitives: vec![(mesh, mat)],
            mesh_transform: Transform::IDENTITY,
        });
        return;
    }

    #[allow(unreachable_code)]
    let node_name = "Drone";
    let Some(node_handle) = gltf.named_nodes.get(node_name) else {
        return;
    };

    let Some(node) = node_assets.get(node_handle) else {
        return;
    };
    let Some(gltf_mesh_handle) = node.mesh.as_ref() else {
        return;
    };
    let Some(gltf_mesh) = mesh_assets.get(gltf_mesh_handle) else {
        return;
    };

    let primitives: Vec<(Handle<Mesh>, Handle<StandardMaterial>)> = gltf_mesh
        .primitives
        .iter()
        .map(|p| {
            let mat = match &p.material {
                Some(m) => m.clone(),
                None => materials.add(StandardMaterial::default()),
            };
            (p.mesh.clone(), mat)
        })
        .collect();

    commands.insert_resource(DroneAssets {
        mesh_primitives: primitives,
        mesh_transform: node.transform,
    });
}

pub fn spawn_drones(
    mut commands: Commands,
    drone_assets: Option<Res<DroneAssets>>,
    course: Option<Res<CourseData>>,
    library: Res<ObstacleLibrary>,
    existing_drones: Query<(), With<Drone>>,
) {
    if !existing_drones.is_empty() {
        return;
    }
    let Some(assets) = drone_assets else { return };
    let Some(course) = course else { return };

    let waypoints = generate_waypoints(&course, &library);
    if waypoints.is_empty() {
        warn!("No gates found in course, cannot spawn drones");
        return;
    }

    let first_gate_inst = course
        .instances
        .iter()
        .filter_map(|inst| inst.gate_order.map(|order| (order, inst)))
        .min_by_key(|(order, _)| *order)
        .map(|(_, inst)| inst);
    let Some(first_gate) = first_gate_inst else {
        warn!("No gate instances found in course");
        return;
    };
    let gate_half_width = library
        .get(&first_gate.obstacle_id)
        .and_then(|def| def.trigger_volume.as_ref())
        .map(|tv| tv.half_extents.x)
        .unwrap_or(5.0);
    let next_waypoint = if waypoints.len() > 1 {
        Some(waypoints[1])
    } else {
        None
    };

    let start_positions = compute_start_positions(
        first_gate.translation,
        first_gate.rotation,
        gate_half_width,
        next_waypoint,
        DRONE_COUNT,
    );
    let mut rng = rand::thread_rng();

    for i in 0..DRONE_COUNT {
        let config = randomize_drone_config(&mut rng);
        let pid = create_pid_with_variation(&config);
        let position = start_positions[i as usize];

        let look_dir = (waypoints[0] - position).normalize_or(Vec3::NEG_Z);
        let flat_dir =
            Vec3::new(look_dir.x, 0.0, look_dir.z).normalize_or(Vec3::NEG_Z);
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat_dir);

        let transform = Transform::from_translation(position).with_rotation(rotation);

        let mut dynamics = DroneDynamics::default();
        let hover_thrust = GRAVITY * dynamics.mass;
        // Start motors at hover thrust so there's no initial drop/overshoot
        dynamics.thrust = hover_thrust;
        dynamics.commanded_thrust = hover_thrust;

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
                waypoints: waypoints.clone(),
                current_waypoint: 0,
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
        "Spawned {} drones with {} waypoints",
        DRONE_COUNT,
        waypoints.len()
    );
}

pub fn cleanup_drone_resources(mut commands: Commands) {
    commands.remove_resource::<DroneAssets>();
    commands.remove_resource::<DroneGltfHandle>();
}

// --- Pure helper functions (testable) ---

pub fn generate_waypoints(course: &CourseData, library: &ObstacleLibrary) -> Vec<Vec3> {
    let mut gates: Vec<(u32, Vec3)> = course
        .instances
        .iter()
        .filter_map(|inst| {
            inst.gate_order.map(|order| {
                // Use the trigger volume center as the fly-through target,
                // not the obstacle origin (which is at ground level).
                let fly_through_offset = library
                    .get(&inst.obstacle_id)
                    .and_then(|def| def.trigger_volume.as_ref())
                    .map(|tv| inst.rotation * tv.offset)
                    .unwrap_or(Vec3::ZERO);
                (order, inst.translation + fly_through_offset)
            })
        })
        .collect();
    gates.sort_by_key(|(order, _)| *order);
    let mut waypoints: Vec<Vec3> = gates.into_iter().map(|(_, pos)| pos).collect();
    // Close the loop: append the first gate as the final waypoint so drones
    // fly back to the start after passing all gates.
    if waypoints.len() >= 2 {
        waypoints.push(waypoints[0]);
    }
    waypoints
}

pub fn compute_start_positions(
    gate_translation: Vec3,
    gate_rotation: Quat,
    gate_half_width: f32,
    next_waypoint: Option<Vec3>,
    count: u8,
) -> Vec<Vec3> {
    // Gate's local Z axis = fly-through axis
    let gate_z = gate_rotation * Vec3::Z;
    let gate_z_flat = Vec3::new(gate_z.x, 0.0, gate_z.z).normalize_or(Vec3::Z);

    // Determine which direction along the gate's Z axis leads toward the next gate.
    // Drones start on the opposite side and fly through.
    let through_dir = if let Some(next) = next_waypoint {
        let to_next = next - gate_translation;
        let to_next_flat = Vec3::new(to_next.x, 0.0, to_next.z).normalize_or(gate_z_flat);
        if gate_z_flat.dot(to_next_flat) > 0.0 {
            gate_z_flat
        } else {
            -gate_z_flat
        }
    } else {
        gate_z_flat
    };

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
        }
    }

    fn wall_instance(translation: Vec3) -> ObstacleInstance {
        ObstacleInstance {
            obstacle_id: ObstacleId("wall".to_string()),
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gate_order: None,
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
            }),
            is_gate: true,
            model_offset: Vec3::ZERO,
        });
        lib
    }

    #[test]
    fn generate_waypoints_sorts_by_gate_order() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                gate_instance(Vec3::new(10.0, 0.0, 0.0), 2),
                gate_instance(Vec3::new(0.0, 0.0, 0.0), 0),
                gate_instance(Vec3::new(5.0, 0.0, 0.0), 1),
            ],
        };

        let waypoints = generate_waypoints(&course, &lib);
        // 3 gates + 1 loop-closing waypoint back to gate 0
        assert_eq!(waypoints.len(), 4);
        // Trigger volume offset adds Y=5.0
        assert_eq!(waypoints[0], Vec3::new(0.0, 5.0, 0.0));
        assert_eq!(waypoints[1], Vec3::new(5.0, 5.0, 0.0));
        assert_eq!(waypoints[2], Vec3::new(10.0, 5.0, 0.0));
        assert_eq!(waypoints[3], waypoints[0]); // loop closure
    }

    #[test]
    fn generate_waypoints_excludes_non_gates() {
        let lib = library_with_gate();
        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![
                wall_instance(Vec3::ZERO),
                gate_instance(Vec3::new(1.0, 0.0, 0.0), 0),
            ],
        };

        let waypoints = generate_waypoints(&course, &lib);
        // Single gate: no loop closure (need >= 2 gates)
        assert_eq!(waypoints.len(), 1);
        assert_eq!(waypoints[0], Vec3::new(1.0, 5.0, 0.0));
    }

    #[test]
    fn generate_waypoints_empty_course() {
        let lib = ObstacleLibrary::default();
        let course = CourseData {
            name: "Empty".to_string(),
            instances: vec![],
        };
        let waypoints = generate_waypoints(&course, &lib);
        assert!(waypoints.is_empty());
    }

    #[test]
    fn generate_waypoints_applies_rotation_to_offset() {
        let lib = library_with_gate();
        // Rotate gate 90 degrees around Y axis — offset (0,5,0) stays (0,5,0)
        let mut inst = gate_instance(Vec3::new(10.0, 0.0, 0.0), 0);
        inst.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let course = CourseData {
            name: "Test".to_string(),
            instances: vec![inst],
        };

        let waypoints = generate_waypoints(&course, &lib);
        assert_eq!(waypoints.len(), 1);
        // Y offset is along Y axis, unaffected by Y-axis rotation
        assert!((waypoints[0].y - 5.0).abs() < 0.001);
    }

    #[test]
    fn compute_start_positions_correct_count() {
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            5.0,
            Some(Vec3::new(0.0, 0.0, -20.0)),
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
            Some(Vec3::new(0.0, 2.0, -20.0)),
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
            Some(Vec3::new(0.0, 0.0, -20.0)),
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
            Some(Vec3::new(0.0, 0.0, -20.0)),
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
            Some(Vec3::new(0.0, 0.0, -20.0)),
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
            Some(Vec3::new(-20.0, 0.0, 0.0)),
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
