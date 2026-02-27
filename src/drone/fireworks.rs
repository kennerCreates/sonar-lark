use bevy::prelude::*;
use rand::Rng;

use crate::course::data::PropKind;
use crate::drone::spawning::DRONE_COLORS;
use crate::palette;
use crate::pilot::SelectedPilots;
use crate::race::gate::{GateForward, GateIndex};
use crate::race::lifecycle::RaceEndSound;
use crate::race::progress::RaceProgress;
use crate::states::AppState;

// --- Spark (main burst particle) ---
const SPARK_COUNT: usize = 35;
const SPARK_SIZE: f32 = 0.06;
const SPARK_LIFETIME: f32 = 2.0;
const SPARK_SPEED_MIN: f32 = 8.0;
const SPARK_SPEED_MAX: f32 = 15.0;
const SPARK_GRAVITY: f32 = 4.0;
const SPARK_DRAG: f32 = 1.5;

// --- Willow (trailing weep particle) ---
const WILLOW_COUNT: usize = 4;
const WILLOW_SIZE: f32 = 0.1;
const WILLOW_LIFETIME: f32 = 3.0;
const WILLOW_SPEED_MIN: f32 = 3.0;
const WILLOW_SPEED_MAX: f32 = 6.0;
const WILLOW_GRAVITY: f32 = 2.0;
const WILLOW_DRAG: f32 = 2.5;
const WILLOW_GROW_PEAK: f32 = 0.15;

// --- Confetti (gate-level burst) ---
const CONFETTI_COUNT: usize = 30;
const CONFETTI_SIZE: f32 = 0.08;
const CONFETTI_LIFETIME: f32 = 1.5;
const CONFETTI_SPEED_MIN: f32 = 5.0;
const CONFETTI_SPEED_MAX: f32 = 12.0;
const CONFETTI_GRAVITY: f32 = 6.0;
const CONFETTI_DRAG: f32 = 2.0;

// --- Shell layout ---
const SHELL_HEIGHTS: [f32; 3] = [15.0, 20.0, 25.0];
const SHELL_DELAYS: [f32; 3] = [0.3, 0.5, 0.7];
const SHELL_LATERAL_OFFSETS: [f32; 3] = [-5.0, 0.0, 5.0];
const ACCENT_RATIO: f32 = 0.3;

#[derive(Clone, Copy)]
pub enum FireworkKind {
    Spark,
    Willow,
    Confetti,
}

#[derive(Component)]
pub struct FireworkParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub remaining: f32,
    pub kind: FireworkKind,
}

#[derive(Resource)]
pub struct FireworkMeshes {
    pub spark: Handle<Mesh>,
    pub willow: Handle<Mesh>,
    pub confetti: Handle<Mesh>,
}

#[derive(Resource)]
pub struct FireworkSounds(pub Vec<Handle<bevy::audio::AudioSource>>);

#[derive(Component)]
pub struct PendingShell {
    pub position: Vec3,
    pub delay: f32,
    pub color: Color,
    pub accent_color: Color,
}

#[derive(Resource)]
pub struct FireworksTriggered;

/// Marker for a firework emitter placed in the course. Spawned at race time from course props.
#[derive(Component)]
pub struct FireworkEmitter {
    pub kind: PropKind,
    pub color_override: Option<Color>,
}

pub fn load_firework_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(FireworkMeshes {
        spark: meshes.add(Cuboid::new(SPARK_SIZE, SPARK_SIZE, SPARK_SIZE)),
        willow: meshes.add(Cuboid::new(WILLOW_SIZE, WILLOW_SIZE, WILLOW_SIZE)),
        confetti: meshes.add(Cuboid::new(CONFETTI_SIZE, CONFETTI_SIZE, CONFETTI_SIZE)),
    });

    // Gracefully handle missing sound file
    let handle: Handle<bevy::audio::AudioSource> = asset_server.load("sounds/firework.wav");
    commands.insert_resource(FireworkSounds(vec![handle]));
}

pub fn detect_first_finish(
    mut commands: Commands,
    progress: Option<Res<RaceProgress>>,
    triggered: Option<Res<FireworksTriggered>>,
    firework_meshes: Option<Res<FireworkMeshes>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_pilots: Option<Res<SelectedPilots>>,
    gates: Query<(&GlobalTransform, &GateIndex, Option<&GateForward>)>,
    emitters: Query<(&Transform, &FireworkEmitter)>,
    race_end_sound: Option<Res<RaceEndSound>>,
) {
    if triggered.is_some() {
        return;
    }
    let Some(progress) = progress else { return };

    // Find the first drone that finished
    let winner = progress
        .drone_states
        .iter()
        .enumerate()
        .find(|(_, s)| s.finished);
    let Some((winner_idx, _)) = winner else { return };

    commands.insert_resource(FireworksTriggered);

    if let Some(ref sound) = race_end_sound {
        commands.spawn((
            AudioPlayer::new(sound.0.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }

    let Some(firework_meshes) = firework_meshes else {
        return;
    };

    let drone_color = selected_pilots
        .as_ref()
        .and_then(|s| s.pilots.get(winner_idx))
        .map(|p| p.color)
        .unwrap_or(DRONE_COLORS[winner_idx % DRONE_COLORS.len()]);
    let accent_color = palette::SUNSHINE;

    let has_emitters = !emitters.is_empty();

    if has_emitters {
        // Use placed emitters from the course
        for (transform, emitter) in &emitters {
            let color = emitter.color_override.unwrap_or(drone_color);
            let pos = transform.translation;
            let forward = (transform.rotation * Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
            let lateral = forward.cross(Vec3::Y).normalize_or(Vec3::X);

            match emitter.kind {
                PropKind::ConfettiEmitter => {
                    spawn_confetti(
                        &mut commands,
                        &firework_meshes,
                        &mut materials,
                        pos,
                        forward,
                        lateral,
                        color,
                        accent_color,
                    );
                }
                PropKind::ShellBurstEmitter => {
                    for i in 0..3 {
                        let shell_pos =
                            pos + Vec3::Y * SHELL_HEIGHTS[i] + lateral * SHELL_LATERAL_OFFSETS[i];
                        commands.spawn((
                            PendingShell {
                                position: shell_pos,
                                delay: SHELL_DELAYS[i],
                                color,
                                accent_color,
                            },
                            DespawnOnExit(AppState::Results),
                        ));
                    }
                }
            }
        }

        info!(
            "Fireworks triggered for drone {} using {} placed emitter(s)!",
            winner_idx,
            emitters.iter().count()
        );
    } else {
        // Fallback: auto-fireworks at gate 0
        let finish_gate = gates.iter().find(|(_, idx, _)| idx.0 == 0);
        let (gate_pos, gate_forward) = match finish_gate {
            Some((gt, _, fwd)) => {
                let forward =
                    fwd.map(|f| f.0).unwrap_or(Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
                (gt.translation(), forward)
            }
            None => return,
        };

        let lateral = gate_forward.cross(Vec3::Y).normalize_or(Vec3::X);

        spawn_confetti(
            &mut commands,
            &firework_meshes,
            &mut materials,
            gate_pos,
            gate_forward,
            lateral,
            drone_color,
            accent_color,
        );

        for i in 0..3 {
            let shell_pos =
                gate_pos + Vec3::Y * SHELL_HEIGHTS[i] + lateral * SHELL_LATERAL_OFFSETS[i];
            commands.spawn((
                PendingShell {
                    position: shell_pos,
                    delay: SHELL_DELAYS[i],
                    color: drone_color,
                    accent_color,
                },
                DespawnOnExit(AppState::Results),
            ));
        }

        info!(
            "Fireworks triggered for drone {} at finish gate!",
            winner_idx
        );
    }
}

fn spawn_confetti(
    commands: &mut Commands,
    firework_meshes: &FireworkMeshes,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    gate_pos: Vec3,
    gate_forward: Vec3,
    lateral: Vec3,
    drone_color: Color,
    accent_color: Color,
) {
    let mut rng = rand::thread_rng();

    let drone_linear = drone_color.to_linear();
    let confetti_primary = materials.add(StandardMaterial {
        base_color: drone_color,
        emissive: LinearRgba::new(
            drone_linear.red * 3.0,
            drone_linear.green * 3.0,
            drone_linear.blue * 3.0,
            1.0,
        ),
        unlit: true,
        ..default()
    });

    let accent_linear = accent_color.to_linear();
    let confetti_accent = materials.add(StandardMaterial {
        base_color: accent_color,
        emissive: LinearRgba::new(
            accent_linear.red * 3.0,
            accent_linear.green * 3.0,
            accent_linear.blue * 3.0,
            1.0,
        ),
        unlit: true,
        ..default()
    });

    for i in 0..CONFETTI_COUNT {
        let use_accent = (i as f32 / CONFETTI_COUNT as f32) < 0.5;
        let material = if use_accent {
            confetti_accent.clone()
        } else {
            confetti_primary.clone()
        };

        // Horizontal fan: gate forward + lateral spread + slight upward
        let forward_bias = gate_forward * rng.gen_range(0.3..1.0);
        let lateral_spread = lateral * rng.gen_range(-1.0..1.0);
        let up_bias = Vec3::Y * rng.gen_range(0.4..1.6);
        let dir = (forward_bias + lateral_spread + up_bias).normalize_or(Vec3::Y);
        let speed = rng.gen_range(CONFETTI_SPEED_MIN..CONFETTI_SPEED_MAX);

        commands.spawn((
            Transform::from_translation(gate_pos + Vec3::Y * 2.0),
            Visibility::default(),
            Mesh3d(firework_meshes.confetti.clone()),
            MeshMaterial3d(material),
            FireworkParticle {
                velocity: dir * speed,
                lifetime: CONFETTI_LIFETIME,
                remaining: CONFETTI_LIFETIME,
                kind: FireworkKind::Confetti,
            },
            DespawnOnExit(AppState::Results),
        ));
    }
}

fn spawn_shell_burst(
    commands: &mut Commands,
    firework_meshes: &FireworkMeshes,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    color: Color,
    accent_color: Color,
) {
    let mut rng = rand::thread_rng();

    let primary_linear = color.to_linear();
    let spark_primary = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::new(
            primary_linear.red * 4.0,
            primary_linear.green * 4.0,
            primary_linear.blue * 4.0,
            1.0,
        ),
        unlit: true,
        ..default()
    });

    let accent_linear = accent_color.to_linear();
    let spark_accent = materials.add(StandardMaterial {
        base_color: accent_color,
        emissive: LinearRgba::new(
            accent_linear.red * 4.0,
            accent_linear.green * 4.0,
            accent_linear.blue * 4.0,
            1.0,
        ),
        unlit: true,
        ..default()
    });

    // Dimmer version for willow particles
    let willow_material = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::new(
            primary_linear.red * 1.5,
            primary_linear.green * 1.5,
            primary_linear.blue * 1.5,
            1.0,
        ),
        unlit: true,
        ..default()
    });

    // --- Sparks ---
    for _ in 0..SPARK_COUNT {
        let use_accent = rng.r#gen::<f32>() < ACCENT_RATIO;
        let material = if use_accent {
            spark_accent.clone()
        } else {
            spark_primary.clone()
        };

        // Spherical distribution with slight upward bias
        let dir = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-0.3..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or(Vec3::Y);

        let speed = rng.gen_range(SPARK_SPEED_MIN..SPARK_SPEED_MAX);

        // Slight lifetime variation so they don't all vanish at once
        let life_var = rng.gen_range(0.8..1.0);

        commands.spawn((
            Transform::from_translation(position),
            Visibility::default(),
            Mesh3d(firework_meshes.spark.clone()),
            MeshMaterial3d(material),
            FireworkParticle {
                velocity: dir * speed,
                lifetime: SPARK_LIFETIME * life_var,
                remaining: SPARK_LIFETIME * life_var,
                kind: FireworkKind::Spark,
            },
            DespawnOnExit(AppState::Results),
        ));
    }

    // --- Willow particles ---
    for _ in 0..WILLOW_COUNT {
        let dir = Vec3::new(
            rng.gen_range(-0.5..0.5),
            rng.gen_range(-0.2..0.5),
            rng.gen_range(-0.5..0.5),
        )
        .normalize_or(Vec3::Y);

        let speed = rng.gen_range(WILLOW_SPEED_MIN..WILLOW_SPEED_MAX);

        commands.spawn((
            Transform::from_translation(position).with_scale(Vec3::splat(0.1)),
            Visibility::default(),
            Mesh3d(firework_meshes.willow.clone()),
            MeshMaterial3d(willow_material.clone()),
            FireworkParticle {
                velocity: dir * speed,
                lifetime: WILLOW_LIFETIME,
                remaining: WILLOW_LIFETIME,
                kind: FireworkKind::Willow,
            },
            DespawnOnExit(AppState::Results),
        ));
    }
}

pub fn tick_firework_shells(
    mut commands: Commands,
    time: Res<Time>,
    firework_meshes: Option<Res<FireworkMeshes>>,
    firework_sounds: Option<Res<FireworkSounds>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shells: Query<(Entity, &mut PendingShell)>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    let Some(firework_meshes) = firework_meshes else {
        return;
    };

    for (entity, mut shell) in &mut shells {
        shell.delay -= dt;
        if shell.delay <= 0.0 {
            spawn_shell_burst(
                &mut commands,
                &firework_meshes,
                &mut materials,
                shell.position,
                shell.color,
                shell.accent_color,
            );

            // TODO: firework sound temporarily disconnected
            let _ = &firework_sounds;

            commands.entity(entity).despawn();
        }
    }
}

pub fn update_firework_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut FireworkParticle)>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }

    for (entity, mut transform, mut particle) in &mut query {
        particle.remaining -= dt;

        if particle.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let life_fraction = (particle.remaining / particle.lifetime).clamp(0.0, 1.0);

        match particle.kind {
            FireworkKind::Spark => {
                particle.velocity.y -= SPARK_GRAVITY * dt;
                let drag_factor = (-SPARK_DRAG * dt).exp();
                particle.velocity *= drag_factor;
                transform.translation += particle.velocity * dt;
                transform.scale = Vec3::splat(life_fraction.max(0.01));
            }
            FireworkKind::Willow => {
                particle.velocity.y -= WILLOW_GRAVITY * dt;
                let drag_factor = (-WILLOW_DRAG * dt).exp();
                particle.velocity *= drag_factor;
                transform.translation += particle.velocity * dt;

                // Grow then slowly shrink (quadratic fade for lingering)
                let elapsed = 1.0 - life_fraction;
                let scale = if elapsed < WILLOW_GROW_PEAK {
                    0.1 + 0.9 * (elapsed / WILLOW_GROW_PEAK)
                } else {
                    let t = (elapsed - WILLOW_GROW_PEAK) / (1.0 - WILLOW_GROW_PEAK);
                    1.0 - t * t
                };
                transform.scale = Vec3::splat(scale.max(0.01));
            }
            FireworkKind::Confetti => {
                particle.velocity.y -= CONFETTI_GRAVITY * dt;
                let drag_factor = (-CONFETTI_DRAG * dt).exp();
                particle.velocity *= drag_factor;
                transform.translation += particle.velocity * dt;
                transform.scale = Vec3::splat(life_fraction.max(0.01));
            }
        }
    }
}

pub fn cleanup_firework_assets(mut commands: Commands) {
    commands.remove_resource::<FireworkMeshes>();
    commands.remove_resource::<FireworkSounds>();
    commands.remove_resource::<FireworksTriggered>();
}
