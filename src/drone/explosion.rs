use bevy::prelude::*;
use rand::Rng;

use crate::palette;
use crate::states::AppState;

// --- Debris ---
const DEBRIS_COUNT: usize = 55;
const DEBRIS_SIZE_SMALL: f32 = 0.04;
const DEBRIS_SIZE_MED: f32 = 0.08;
const DEBRIS_SIZE_LARGE: f32 = 0.12;
const DEBRIS_LIFETIME: f32 = 1.5;
const DEBRIS_SPEED_MIN: f32 = 3.0;
const DEBRIS_SPEED_MAX: f32 = 12.0;
const DEBRIS_MOMENTUM_FACTOR: f32 = 0.4;

// --- Hot smoke (inner bright layer) ---
const HOT_SMOKE_COUNT: usize = 10;
const HOT_SMOKE_SIZE: f32 = 0.5;
const HOT_SMOKE_LIFETIME: f32 = 2.0;
const HOT_SMOKE_RISE_SPEED: f32 = 1.2;
const HOT_SMOKE_SPREAD_SPEED: f32 = 1.5;
const HOT_SMOKE_DRAG: f32 = 1.8;
const HOT_SMOKE_GROW_PEAK: f32 = 0.25;

// --- Dark smoke (outer halo layer) ---
const DARK_SMOKE_COUNT: usize = 12;
const DARK_SMOKE_SIZE: f32 = 0.8;
const DARK_SMOKE_LIFETIME: f32 = 3.5;
const DARK_SMOKE_RISE_SPEED: f32 = 0.8;
const DARK_SMOKE_SPREAD_SPEED: f32 = 1.0;
const DARK_SMOKE_DRAG: f32 = 1.5;
const DARK_SMOKE_GROW_PEAK: f32 = 0.2;

const GRAVITY: f32 = 9.81;
const CRASH_SOUND_COUNT: usize = 6;

#[derive(Clone, Copy)]
pub enum ParticleKind {
    Debris,
    HotSmoke,
    DarkSmoke,
}

#[derive(Component)]
pub struct ExplosionParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub remaining: f32,
    pub kind: ParticleKind,
}

#[derive(Resource)]
pub struct CrashSounds(pub Vec<Handle<bevy::audio::AudioSource>>);

/// Pre-allocated meshes for explosion particles, shared across all explosions.
#[derive(Resource)]
pub struct ExplosionMeshes {
    pub debris_small: Handle<Mesh>,
    pub debris_med: Handle<Mesh>,
    pub debris_large: Handle<Mesh>,
    pub hot_smoke: Handle<Mesh>,
    pub dark_smoke: Handle<Mesh>,
}

pub fn load_explosion_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let handles: Vec<Handle<bevy::audio::AudioSource>> = (1..=CRASH_SOUND_COUNT)
        .map(|i| asset_server.load(format!("sounds/drone_crash/drone_crash_pole_hit_{i}.wav")))
        .collect();
    commands.insert_resource(CrashSounds(handles));
    commands.insert_resource(ExplosionMeshes {
        debris_small: meshes.add(Cuboid::new(DEBRIS_SIZE_SMALL, DEBRIS_SIZE_SMALL, DEBRIS_SIZE_SMALL)),
        debris_med: meshes.add(Cuboid::new(DEBRIS_SIZE_MED, DEBRIS_SIZE_MED, DEBRIS_SIZE_MED)),
        debris_large: meshes.add(Cuboid::new(DEBRIS_SIZE_LARGE, DEBRIS_SIZE_LARGE, DEBRIS_SIZE_LARGE)),
        hot_smoke: meshes.add(Cuboid::new(HOT_SMOKE_SIZE, HOT_SMOKE_SIZE, HOT_SMOKE_SIZE)),
        dark_smoke: meshes.add(Cuboid::new(DARK_SMOKE_SIZE, DARK_SMOKE_SIZE, DARK_SMOKE_SIZE)),
    });
}

pub fn spawn_explosion(
    commands: &mut Commands,
    explosion_meshes: &ExplosionMeshes,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    drone_velocity: Vec3,
    color: Color,
) {
    let mut rng = rand::thread_rng();

    // --- Debris (reuse pre-allocated meshes) ---
    let mesh_small = &explosion_meshes.debris_small;
    let mesh_med = &explosion_meshes.debris_med;
    let mesh_large = &explosion_meshes.debris_large;

    let linear = color.to_linear();
    let debris_material = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::new(
            linear.red * 3.0,
            linear.green * 3.0,
            linear.blue * 3.0,
            1.0,
        ),
        unlit: true,
        ..default()
    });

    let momentum = drone_velocity * DEBRIS_MOMENTUM_FACTOR;

    for i in 0..DEBRIS_COUNT {
        let dir = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or(Vec3::Y);

        let speed = rng.gen_range(DEBRIS_SPEED_MIN..DEBRIS_SPEED_MAX);

        let mesh = match i % 3 {
            0 => mesh_small.clone(),
            1 => mesh_med.clone(),
            _ => mesh_large.clone(),
        };

        commands.spawn((
            Transform::from_translation(position),
            Visibility::default(),
            Mesh3d(mesh),
            MeshMaterial3d(debris_material.clone()),
            ExplosionParticle {
                velocity: dir * speed + momentum,
                lifetime: DEBRIS_LIFETIME,
                remaining: DEBRIS_LIFETIME,
                kind: ParticleKind::Debris,
            },
            DespawnOnExit(AppState::Results),
        ));
    }

    // --- Hot smoke (bright orange/red core, reuse pre-allocated mesh) ---
    let hot_smoke_mesh = &explosion_meshes.hot_smoke;
    let hot_smoke_material = materials.add(StandardMaterial {
        base_color: palette::TANGERINE,
        emissive: LinearRgba::new(2.8, 1.15, 0.37, 1.0),
        unlit: true,
        ..default()
    });

    for _ in 0..HOT_SMOKE_COUNT {
        let radial = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..0.5),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or(Vec3::Y);

        commands.spawn((
            Transform::from_translation(position).with_scale(Vec3::splat(0.1)),
            Visibility::default(),
            Mesh3d(hot_smoke_mesh.clone()),
            MeshMaterial3d(hot_smoke_material.clone()),
            ExplosionParticle {
                velocity: radial * HOT_SMOKE_SPREAD_SPEED
                    + Vec3::Y * (HOT_SMOKE_RISE_SPEED + rng.gen_range(-0.3..0.3)),
                lifetime: HOT_SMOKE_LIFETIME,
                remaining: HOT_SMOKE_LIFETIME,
                kind: ParticleKind::HotSmoke,
            },
            DespawnOnExit(AppState::Results),
        ));
    }

    // --- Dark smoke (black/charcoal halo, reuse pre-allocated mesh) ---
    let dark_smoke_mesh = &explosion_meshes.dark_smoke;
    let dark_smoke_material = materials.add(StandardMaterial {
        base_color: palette::SMOKY_BLACK,
        unlit: true,
        ..default()
    });

    for _ in 0..DARK_SMOKE_COUNT {
        let radial = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..0.3),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or(Vec3::Y);

        commands.spawn((
            Transform::from_translation(position).with_scale(Vec3::splat(0.1)),
            Visibility::default(),
            Mesh3d(dark_smoke_mesh.clone()),
            MeshMaterial3d(dark_smoke_material.clone()),
            ExplosionParticle {
                velocity: radial * DARK_SMOKE_SPREAD_SPEED
                    + Vec3::Y * (DARK_SMOKE_RISE_SPEED + rng.gen_range(-0.2..0.2)),
                lifetime: DARK_SMOKE_LIFETIME,
                remaining: DARK_SMOKE_LIFETIME,
                kind: ParticleKind::DarkSmoke,
            },
            DespawnOnExit(AppState::Results),
        ));
    }

}

pub fn update_explosion_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionParticle)>,
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
            ParticleKind::Debris => {
                particle.velocity.y -= GRAVITY * dt;
                transform.translation += particle.velocity * dt;
                transform.scale = Vec3::splat(life_fraction);
            }
            ParticleKind::HotSmoke => {
                let drag_factor = (-HOT_SMOKE_DRAG * dt).exp();
                particle.velocity *= drag_factor;
                particle.velocity.y += 0.3 * dt;
                transform.translation += particle.velocity * dt;

                let elapsed = 1.0 - life_fraction;
                let scale = if elapsed < HOT_SMOKE_GROW_PEAK {
                    // Grow from 0.1 to 1.0
                    0.1 + 0.9 * (elapsed / HOT_SMOKE_GROW_PEAK)
                } else {
                    // Shrink from 1.0 to 0.0
                    let t = (elapsed - HOT_SMOKE_GROW_PEAK) / (1.0 - HOT_SMOKE_GROW_PEAK);
                    1.0 - t
                };
                transform.scale = Vec3::splat(scale.max(0.01));
            }
            ParticleKind::DarkSmoke => {
                let drag_factor = (-DARK_SMOKE_DRAG * dt).exp();
                particle.velocity *= drag_factor;
                particle.velocity.y += 0.2 * dt;
                transform.translation += particle.velocity * dt;

                let elapsed = 1.0 - life_fraction;
                let scale = if elapsed < DARK_SMOKE_GROW_PEAK {
                    // Grow from 0.1 to 1.0
                    0.1 + 0.9 * (elapsed / DARK_SMOKE_GROW_PEAK)
                } else {
                    // Hold large then slowly shrink
                    let t = (elapsed - DARK_SMOKE_GROW_PEAK) / (1.0 - DARK_SMOKE_GROW_PEAK);
                    1.0 - t * t
                };
                transform.scale = Vec3::splat(scale.max(0.01));
            }
        }
    }
}

pub fn cleanup_explosion_assets(mut commands: Commands) {
    commands.remove_resource::<CrashSounds>();
    commands.remove_resource::<ExplosionMeshes>();
}
