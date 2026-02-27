use bevy::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;

use crate::course::data::CourseData;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;
use crate::rendering::{CelLightDir, CelMaterial, cel_material_from_color};
use crate::states::AppState;
use super::components::*;
use super::interpolation::{PreviousTranslation, PreviousRotation};
use super::paths::{RacePath, generate_race_path, generate_drone_race_path, compute_start_positions};

const DRONE_COUNT: u8 = 12;
const GRAVITY: f32 = 9.81;

/// Per-drone colors — 12 palette colors chosen for maximum hue spread.
pub const DRONE_COLORS: [Color; 12] = [
    palette::NEON_RED,
    palette::SUNFLOWER,
    palette::LIMON,
    palette::GRASS,
    palette::FROG,
    palette::JADE,
    palette::SKY,
    palette::HOMEWORLD,
    palette::PERIWINKLE,
    palette::AMETHYST,
    palette::PINK,
    palette::VANILLA,
];

/// Callsigns for each of the 12 drones, matching `DRONE_COLORS` indices.
pub const DRONE_NAMES: [&str; 12] = [
    "FALCON", "VIPER", "HAWK", "PHANTOM",
    "SPARK", "BLITZ", "NOVA", "DRIFT",
    "SURGE", "BOLT", "ECHO", "FURY",
];

const _: () = assert!(DRONE_COLORS.len() == DRONE_COUNT as usize);
const _: () = assert!(DRONE_NAMES.len() == DRONE_COUNT as usize);

/// Marker resource inserted when `spawn_drones` detects the course has no gates.
/// Prevents the warning from repeating every frame and signals the UI to show a banner.
#[derive(Resource)]
pub struct NoGatesCourse;

#[derive(Resource)]
pub struct DroneGltfHandle(pub Handle<bevy::gltf::Gltf>);

#[derive(Resource)]
pub struct DroneAssets {
    pub mesh_primitives: Vec<Handle<Mesh>>,
    pub mesh_transform: Transform,
}

pub fn load_drone_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("models/drone.glb");
    commands.insert_resource(DroneGltfHandle(handle));
}

/// Run condition: true when the drone glTF and all its dependencies are loaded.
pub fn drone_gltf_ready(
    handle: Option<Res<DroneGltfHandle>>,
    asset_server: Res<AssetServer>,
) -> bool {
    handle.is_some_and(|h| asset_server.is_loaded_with_dependencies(&h.0))
}

/// Extracts mesh primitives from the loaded drone glTF into `DroneAssets`.
/// Gated by `run_if(drone_gltf_ready)` and `run_if(not(resource_exists::<DroneAssets>))`.
pub fn setup_drone_assets(
    mut commands: Commands,
    gltf_handle: Res<DroneGltfHandle>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let gltf = gltf_assets.get(&gltf_handle.0).expect("run condition guarantees loaded");

    let node_name: Box<str> = "Drone".into();
    if let Some(node_handle) = gltf.named_nodes.get(&node_name) {
        if let Some(node) = node_assets.get(node_handle) {
            if let Some(gltf_mesh_handle) = node.mesh.as_ref() {
                if let Some(gltf_mesh) = mesh_assets.get(gltf_mesh_handle) {
                    let primitives: Vec<Handle<Mesh>> = gltf_mesh
                        .primitives
                        .iter()
                        .map(|p| p.mesh.clone())
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

    // Fallback: placeholder cube if glTF node is missing or empty
    let mesh = meshes.add(Cuboid::new(0.5, 0.3, 0.5));
    commands.insert_resource(DroneAssets {
        mesh_primitives: vec![mesh],
        mesh_transform: Transform::IDENTITY,
    });
}

/// Spawns 12 AI drones once `DroneAssets` and `CourseData` are ready.
/// Gated by `run_if(resource_exists::<DroneAssets>)` and `run_if(resource_exists::<CourseData>)`.
pub fn spawn_drones(
    mut commands: Commands,
    assets: Res<DroneAssets>,
    course: Res<CourseData>,
    library: Res<ObstacleLibrary>,
    existing_drones: Query<(), With<Drone>>,
    no_gates: Option<Res<NoGatesCourse>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    light_dir: Res<CelLightDir>,
) {
    if !existing_drones.is_empty() || no_gates.is_some() {
        return;
    }

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
    let race_seed = rng.gen_range(0u32..=u32::MAX);
    commands.insert_resource(RaceSeed(race_seed));

    // Shuffle grid slot order so drones don't always line up the same way
    let mut grid_slots: Vec<u8> = (0..DRONE_COUNT).collect();
    grid_slots.shuffle(&mut rng);

    for i in 0..DRONE_COUNT {
        let config = randomize_drone_config(&mut rng);

        // Generate per-drone unique spline path
        let drone_path = generate_drone_race_path(&course, &library, &config, i, race_seed)
            .unwrap_or_else(|| {
                warn!("Per-drone path failed for drone {}, using shared path", i);
                RacePath {
                    spline: race_path.spline.clone(),
                    gate_positions: race_path.gate_positions.clone(),
                    gate_forwards: race_path.gate_forwards.clone(),
                }
            });

        let pid = create_pid_with_variation(&config);
        let position = start_positions[grid_slots[i as usize] as usize];

        let look_dir =
            (drone_path.gate_positions[0] - position).normalize_or(Vec3::NEG_Z);
        let flat_dir =
            Vec3::new(look_dir.x, 0.0, look_dir.z).normalize_or(Vec3::NEG_Z);
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, flat_dir);

        let transform = Transform::from_translation(position).with_rotation(rotation);

        let mut dynamics = DroneDynamics::default();
        let hover_thrust = GRAVITY * dynamics.mass;
        dynamics.thrust = hover_thrust;
        dynamics.commanded_thrust = hover_thrust;

        let mut attitude_pd = AttitudePd::default();
        attitude_pd.kp_roll_pitch *= config.attitude_kp_mult;
        attitude_pd.kd_roll_pitch *= config.attitude_kd_mult;

        let gate_count = drone_path.gate_positions.len() as u32;
        let mut entity_cmd = commands.spawn((
            transform,
            Visibility::default(),
            Drone { index: i },
            pid,
            attitude_pd,
            dynamics,
            config,
            AIController {
                target_gate_index: 0,
                gate_count,
                spline: drone_path.spline,
                spline_t: 0.0,
                gate_positions: drone_path.gate_positions,
                gate_forwards: drone_path.gate_forwards,
            },
            DesiredPosition {
                position,
                velocity_hint: look_dir,
                max_speed: 45.0,
            },
            DesiredAttitude {
                orientation: rotation,
                thrust_magnitude: hover_thrust,
            },
            DroneStartPosition {
                translation: position,
            },
            DronePhase::default(),
            PreviousTranslation(position),
            PreviousRotation(rotation),
            DespawnOnExit(AppState::Results),
        ));

        let drone_color = DRONE_COLORS[i as usize];
        let drone_mat =
            cel_materials.add(cel_material_from_color(drone_color, light_dir.0));
        entity_cmd.with_children(|children| {
            for mesh in &assets.mesh_primitives {
                children.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(drone_mat.clone()),
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
    commands.remove_resource::<RaceSeed>();
}

// --- Private helpers ---

fn randomize_drone_config(rng: &mut impl Rng) -> DroneConfig {
    // Generate cornering_aggression first — path params derive from it
    let cornering_aggression: f32 = rng.gen_range(0.8..=1.2);

    // Aggressive drones get larger midleg shifts (bolder line choices)
    let raw_bias: f32 = rng.gen_range(-1.0..=1.0);
    let racing_line_bias = raw_bias * (2.0 + cornering_aggression * 2.0);

    // Aggressive drones commit later to gate direction (shorter approach)
    let approach_offset_scale = 1.0 - (cornering_aggression - 1.0) * 0.5;

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
        cornering_aggression,
        braking_distance: rng.gen_range(0.8..=1.2),
        attitude_kp_mult: rng.gen_range(0.9..=1.1),
        attitude_kd_mult: rng.gen_range(0.9..=1.1),
        racing_line_bias,
        approach_offset_scale,
        // Fraction of gate opening used for pass-through offset (0.3–0.6).
        // Aggressive drones use wider offsets (bolder lines through gates).
        gate_pass_offset: rng.gen_range(0.2..=0.4) + (cornering_aggression - 0.8) * 0.5,
    }
}

fn create_pid_with_variation(config: &DroneConfig) -> PositionPid {
    let base = PositionPid::default();
    PositionPid {
        kp: base.kp * (Vec3::ONE + config.pid_variation),
        ki: base.ki * (Vec3::ONE + config.pid_variation),
        kd: base.kd * (Vec3::ONE + config.pid_variation),
        integral: Vec3::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!((0.8..=1.2).contains(&config.cornering_aggression));
            assert!((0.8..=1.2).contains(&config.braking_distance));
            assert!((0.9..=1.1).contains(&config.attitude_kp_mult));
            assert!((0.9..=1.1).contains(&config.attitude_kd_mult));
            // racing_line_bias: max magnitude = 1.0 * (2.0 + 1.2 * 2.0) = 4.4
            assert!(config.racing_line_bias.abs() <= 4.5);
            // approach_offset_scale: aggression 0.8 → 1.1, aggression 1.2 → 0.9
            assert!((0.89..=1.11).contains(&config.approach_offset_scale));
            // gate_pass_offset: fraction 0.2..0.6 (0.2+0 to 0.4+0.2)
            assert!(
                (0.19..=0.61).contains(&config.gate_pass_offset),
                "gate_pass_offset {} out of range", config.gate_pass_offset
            );
        }
    }

    #[test]
    fn create_pid_with_variation_applies_correctly() {
        let config = DroneConfig {
            pid_variation: Vec3::new(0.1, -0.1, 0.05),
            hover_noise_amp: Vec3::splat(0.02),
            hover_noise_freq: Vec3::splat(1.0),
            ..neutral_drone_config()
        };
        let pid = create_pid_with_variation(&config);
        let base = PositionPid::default();

        assert!((pid.kp.x - base.kp.x * 1.1).abs() < 0.001);
        assert!((pid.kp.y - base.kp.y * 0.9).abs() < 0.001);
        assert!((pid.kp.z - base.kp.z * 1.05).abs() < 0.001);
    }
}
