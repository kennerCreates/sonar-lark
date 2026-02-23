use bevy::prelude::*;
use rand::Rng;

use crate::course::data::CourseData;
use crate::obstacle::library::ObstacleLibrary;
use crate::states::AppState;
use super::components::*;

const DRONE_COUNT: u8 = 12;
const START_DISTANCE_BEHIND_GATE: f32 = 15.0;
const LATERAL_SPACING: f32 = 2.0;
const ROWS: usize = 3;
const HOVER_HEIGHT: f32 = 1.5;

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
        let mesh = meshes.add(Cuboid::new(0.3, 0.1, 0.3));
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

    let start_positions = compute_start_positions(&waypoints, DRONE_COUNT);
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

        let mut entity_cmd = commands.spawn((
            transform,
            Visibility::default(),
            Drone { index: i },
            pid,
            DroneDynamics::default(),
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

pub fn compute_start_positions(waypoints: &[Vec3], count: u8) -> Vec<Vec3> {
    let first_gate = waypoints[0];

    // Direction from first gate toward second (or default -Z)
    let forward = if waypoints.len() > 1 {
        let dir = waypoints[1] - waypoints[0];
        Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::NEG_Z)
    } else {
        Vec3::NEG_Z
    };

    // Start line is behind the first gate
    let start_center = first_gate - forward * START_DISTANCE_BEHIND_GATE;

    // Lateral direction (perpendicular to forward on XZ plane)
    let lateral = Vec3::Y.cross(forward).normalize_or(Vec3::X);

    let cols = (count as usize + ROWS - 1) / ROWS;
    let mut positions = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let row = i / cols;
        let col = i % cols;

        let row_offset = -(row as f32) * LATERAL_SPACING * 1.5;
        let col_center = (cols as f32 - 1.0) / 2.0;
        let col_offset = (col as f32 - col_center) * LATERAL_SPACING;

        let pos =
            start_center + forward * row_offset + lateral * col_offset + Vec3::Y * HOVER_HEIGHT;

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
    }
}

fn create_pid_with_variation(config: &DroneConfig) -> PidController {
    let base = PidController::default();
    PidController {
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
        let waypoints = vec![Vec3::ZERO, Vec3::new(0.0, 0.0, -20.0)];
        let positions = compute_start_positions(&waypoints, 12);
        assert_eq!(positions.len(), 12);
    }

    #[test]
    fn compute_start_positions_behind_first_gate() {
        let waypoints = vec![Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 2.0, -20.0)];
        let positions = compute_start_positions(&waypoints, 12);

        for pos in &positions {
            assert!(
                pos.z > waypoints[0].z,
                "drone at z={} should be behind gate at z={}",
                pos.z,
                waypoints[0].z
            );
        }
    }

    #[test]
    fn compute_start_positions_no_overlap() {
        let waypoints = vec![Vec3::ZERO, Vec3::new(0.0, 0.0, -20.0)];
        let positions = compute_start_positions(&waypoints, 12);

        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let dist = (positions[i] - positions[j]).length();
                assert!(dist > 1.0, "drones {} and {} too close: {:.2}", i, j, dist);
            }
        }
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
        }
    }

    #[test]
    fn create_pid_with_variation_applies_correctly() {
        let config = DroneConfig {
            pid_variation: Vec3::new(0.1, -0.1, 0.05),
            line_offset: 0.0,
            noise_amplitude: 1.0,
            noise_frequency: 1.0,
        };
        let pid = create_pid_with_variation(&config);
        let base = PidController::default();

        assert!((pid.kp.x - base.kp.x * 1.1).abs() < 0.001);
        assert!((pid.kp.y - base.kp.y * 0.9).abs() < 0.001);
        assert!((pid.kp.z - base.kp.z * 1.05).abs() < 0.001);
    }
}
