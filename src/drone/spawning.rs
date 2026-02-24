use bevy::math::cubic_splines::{CubicCardinalSpline, CubicCurve, CubicGenerator, CyclicCubicGenerator};
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

/// Per-drone colors from the Jehkoba32 palette (assets/color/color_scheme.hex).
/// 12 colors chosen for visual contrast against typical environments.
const DRONE_COLORS: [[f32; 3]; 12] = [
    [0.851, 0.298, 0.529], // Kirby     #d94c87
    [0.043, 0.686, 0.902], // Cerulean  #0bafe6
    [0.404, 0.702, 0.106], // Pear      #67b31b
    [0.969, 0.788, 0.243], // Dandelion #f7c93e
    [0.851, 0.129, 0.310], // Red       #d9214f
    [0.106, 0.651, 0.514], // Teal      #1ba683
    [0.949, 0.475, 0.380], // Rose      #f27961
    [0.141, 0.412, 0.702], // Blue      #2469b3
    [0.588, 0.890, 0.788], // Beach Glass #96e3c9
    [0.702, 0.561, 0.141], // Apricot   #b38f24
    [0.651, 0.129, 0.431], // Magenta   #a6216e
    [0.941, 0.929, 0.847], // Warm White #f0edd8
];

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

pub fn setup_drone_assets(
    mut commands: Commands,
    gltf_handle: Option<Res<DroneGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
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

pub fn spawn_drones(
    mut commands: Commands,
    drone_assets: Option<Res<DroneAssets>>,
    course: Option<Res<CourseData>>,
    library: Res<ObstacleLibrary>,
    existing_drones: Query<(), With<Drone>>,
    no_gates: Option<Res<NoGatesCourse>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

        // Generate per-drone unique spline path
        let drone_path = generate_drone_race_path(&course, &library, &config, i)
            .unwrap_or_else(|| {
                warn!("Per-drone path failed for drone {}, using shared path", i);
                RacePath {
                    spline: race_path.spline.clone(),
                    gate_positions: race_path.gate_positions.clone(),
                    gate_forwards: race_path.gate_forwards.clone(),
                }
            });

        let pid = create_pid_with_variation(&config);
        let position = start_positions[i as usize];

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
                rotation,
            },
            DronePhase::default(),
            DespawnOnExit(AppState::Race),
        ));

        let [r, g, b] = DRONE_COLORS[i as usize];
        let drone_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            ..default()
        });
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
}

// --- Pure helper functions (testable) ---

const MAX_APPROACH_OFFSET: f32 = 12.0;
const APPROACH_FRACTION: f32 = 0.3;

/// Compute the approach/departure offset for a gate based on distance to the next gate.
/// Scales linearly with inter-gate distance, capped at MAX_APPROACH_OFFSET.
pub fn adaptive_approach_offset(gate_distance: f32) -> f32 {
    (gate_distance * APPROACH_FRACTION).min(MAX_APPROACH_OFFSET)
}

pub struct RacePath {
    pub spline: CubicCurve<Vec3>,
    pub gate_positions: Vec<Vec3>,
    pub gate_forwards: Vec<Vec3>,
}

/// Extract gate positions, forwards, and 2D half-extents from course data, sorted by gate_order.
/// Half-extents are (width, height) of the trigger volume in world space, scaled by instance scale.
fn extract_sorted_gates(
    course: &CourseData,
    library: &ObstacleLibrary,
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec2>) {
    let mut gates: Vec<(u32, Vec3, Vec3, Vec2)> = course
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
                    .map(|tv| inst.rotation * (tv.offset * inst.scale))
                    .unwrap_or(Vec3::ZERO);
                let local_fwd = tv.map(|tv| tv.forward).unwrap_or(Vec3::NEG_Z);
                let world_fwd = inst.rotation
                    * if inst.gate_forward_flipped { -local_fwd } else { local_fwd };
                let half_extents_2d = tv
                    .map(|tv| Vec2::new(tv.half_extents.x * inst.scale.x, tv.half_extents.y * inst.scale.y))
                    .unwrap_or(Vec2::new(3.0, 3.0));
                (order, inst.translation + fly_through_offset, world_fwd, half_extents_2d)
            })
        })
        .collect();
    gates.sort_by_key(|(order, _, _, _)| *order);
    let positions = gates.iter().map(|(_, pos, _, _)| *pos).collect();
    let forwards = gates.iter().map(|(_, _, fwd, _)| *fwd).collect();
    let extents = gates.iter().map(|(_, _, _, ext)| *ext).collect();
    (positions, forwards, extents)
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
        // Horizontal hash
        let h_hash = (drone_index as u32)
            .wrapping_mul(1640531527)
            .wrapping_add((gate_idx as u32).wrapping_mul(2891336453))
            >> 16;
        let h_sign = (h_hash & 0xFFFF) as f32 / 65536.0 * 2.0 - 1.0;
        // Vertical hash (different prime seeds)
        let v_hash = (drone_index as u32)
            .wrapping_mul(2246822519)
            .wrapping_add((gate_idx as u32).wrapping_mul(1640531527))
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

        // Deterministic per-drone-per-leg hash for shift direction
        let hash = (drone_index as u32)
            .wrapping_mul(2654435761)
            .wrapping_add((i as u32).wrapping_mul(2246822519))
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

/// Generates a non-cyclic Catmull-Rom spline from the drone's finish position
/// back to its start position. Each drone gets a unique path based on its index
/// and config, producing organic per-drone variation.
pub fn generate_return_path(
    current_pos: Vec3,
    velocity: Vec3,
    start_pos: Vec3,
    config: &DroneConfig,
    drone_index: u8,
) -> Option<CubicCurve<Vec3>> {
    let speed = velocity.length();
    let vel_dir = if speed > 0.1 {
        velocity / speed
    } else {
        Vec3::NEG_Z
    };

    // Deterministic per-drone hash for variation (no RNG needed)
    let hash = ((drone_index as u32).wrapping_mul(2654435761)) >> 16;
    let hash_f = (hash & 0xFFFF) as f32 / 65536.0; // 0.0..1.0

    // W0: current position
    let w0 = current_pos;

    // W1: momentum carry-through (drone continues in its current direction)
    let carry_dist = 12.0 + (drone_index as f32 * 1.7) % 8.0;
    let w1 = current_pos + vel_dir * carry_dist;

    // Direction from carry point toward start (used for lateral offsets)
    let return_dir = (start_pos - w1).normalize_or(Vec3::NEG_Z);
    let lateral = Vec3::Y.cross(return_dir).normalize_or(Vec3::X);

    // W2: high midpoint with large lateral + vertical offset
    let mid = (w1 + start_pos) * 0.5;
    let lat_offset_high = config.line_offset * 6.0 + (hash_f - 0.5) * 16.0;
    let alt_offset_high = 10.0 + hash_f * 8.0;
    let w2 = mid + lateral * lat_offset_high + Vec3::Y * alt_offset_high;

    // W3: lower midpoint with moderate offset (opposite lateral bias for S-curve feel)
    let mid_low = (mid + start_pos) * 0.5;
    let lat_offset_low = -config.line_offset * 3.0 + (hash_f - 0.5) * 6.0;
    let alt_offset_low = 3.0 + hash_f * 5.0;
    let w3 = mid_low + lateral * lat_offset_low + Vec3::Y * alt_offset_low;

    // W4: approach from above start (staggered height per drone)
    let approach_height = 3.0 + drone_index as f32 * 0.4;
    let w4 = start_pos + Vec3::Y * approach_height;

    // W5: final start position
    let w5 = start_pos;

    let points = [w0, w1, w2, w3, w4, w5];
    let spline = CubicCardinalSpline::new_catmull_rom(points)
        .to_curve()
        .ok()?;
    Some(spline)
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
    fn return_path_produces_valid_spline() {
        let config = DroneConfig {
            line_offset: 0.5,
            ..neutral_drone_config()
        };

        let current_pos = Vec3::new(50.0, 5.0, 30.0);
        let velocity = Vec3::new(0.0, 0.0, -20.0);
        let start_pos = Vec3::new(0.0, 1.5, 5.0);

        let spline =
            generate_return_path(current_pos, velocity, start_pos, &config, 0)
                .expect("should produce a return path");

        // Spline starts near current_pos and ends near start_pos
        let start = spline.position(0.0);
        let end_t = spline.segments().len() as f32;
        let end = spline.position(end_t);

        assert!(
            (start - current_pos).length() < 1.0,
            "spline start {:?} should be near current_pos {:?}",
            start,
            current_pos
        );
        assert!(
            (end - start_pos).length() < 1.0,
            "spline end {:?} should be near start_pos {:?}",
            end,
            start_pos
        );
    }

    #[test]
    fn return_path_varies_by_drone_index() {
        let config = DroneConfig {
            line_offset: 0.5,
            ..neutral_drone_config()
        };

        let current_pos = Vec3::new(50.0, 5.0, 30.0);
        let velocity = Vec3::new(0.0, 0.0, -20.0);
        let start_pos = Vec3::new(0.0, 1.5, 5.0);

        let spline0 =
            generate_return_path(current_pos, velocity, start_pos, &config, 0).unwrap();
        let spline5 =
            generate_return_path(current_pos, velocity, start_pos, &config, 5).unwrap();

        // Sample at midpoint — different drones should take different paths
        let mid_t = spline0.segments().len() as f32 / 2.0;
        let pos0 = spline0.position(mid_t);
        let pos5 = spline5.position(mid_t);
        assert!(
            (pos0 - pos5).length() > 1.0,
            "drone 0 and drone 5 return paths should differ at midpoint"
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

        let path_a = generate_drone_race_path(&course, &lib, &config_a, 0).unwrap();
        let path_b = generate_drone_race_path(&course, &lib, &config_b, 1).unwrap();

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
        };

        let config = DroneConfig {
            racing_line_bias: 4.0,
            approach_offset_scale: 0.9,
            ..neutral_drone_config()
        };

        let path = generate_drone_race_path(&course, &lib, &config, 7).unwrap();

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
        };

        let config = DroneConfig {
            racing_line_bias: 3.5,
            approach_offset_scale: 0.9,
            ..neutral_drone_config()
        };

        let path = generate_drone_race_path(&course, &lib, &config, 3).unwrap();

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
            let path = generate_drone_race_path(&course, &lib, &config, idx).unwrap();
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
        };

        let base = generate_race_path(&course, &lib).unwrap();
        let drone = generate_drone_race_path(&course, &lib, &neutral_drone_config(), 0).unwrap();

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
