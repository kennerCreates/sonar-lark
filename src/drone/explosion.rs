use bevy::prelude::*;
use rand::Rng;

use crate::states::AppState;

const PARTICLE_COUNT: usize = 24;
const PARTICLE_LIFETIME: f32 = 1.5;
const PARTICLE_SIZE: f32 = 0.08;
const PARTICLE_SPEED_MIN: f32 = 3.0;
const PARTICLE_SPEED_MAX: f32 = 12.0;
const GRAVITY: f32 = 9.81;
const EXPLOSION_SOUND_COUNT: usize = 4;

#[derive(Component)]
pub struct ExplosionParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub remaining: f32,
}

#[derive(Resource)]
pub struct ExplosionSounds(pub Vec<Handle<bevy::audio::AudioSource>>);

pub fn load_explosion_sound(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handles: Vec<Handle<bevy::audio::AudioSource>> = (1..=EXPLOSION_SOUND_COUNT)
        .map(|i| asset_server.load(format!("sounds/explosion_{i}.wav")))
        .collect();
    commands.insert_resource(ExplosionSounds(handles));
}

pub fn spawn_explosion(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    color: Color,
    explosion_sounds: Option<&ExplosionSounds>,
) {
    let mut rng = rand::thread_rng();

    let particle_mesh = meshes.add(Cuboid::new(PARTICLE_SIZE, PARTICLE_SIZE, PARTICLE_SIZE));
    let linear = color.to_linear();
    let particle_material = materials.add(StandardMaterial {
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

    for _ in 0..PARTICLE_COUNT {
        let dir = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or(Vec3::Y);

        let speed = rng.gen_range(PARTICLE_SPEED_MIN..PARTICLE_SPEED_MAX);

        commands.spawn((
            Transform::from_translation(position),
            Visibility::default(),
            Mesh3d(particle_mesh.clone()),
            MeshMaterial3d(particle_material.clone()),
            ExplosionParticle {
                velocity: dir * speed,
                lifetime: PARTICLE_LIFETIME,
                remaining: PARTICLE_LIFETIME,
            },
            DespawnOnExit(AppState::Race),
        ));
    }

    if let Some(sounds) = explosion_sounds {
        if !sounds.0.is_empty() {
            let idx = rng.gen_range(0..sounds.0.len());
            commands.spawn((
                AudioPlayer::new(sounds.0[idx].clone()),
                PlaybackSettings::DESPAWN,
                DespawnOnExit(AppState::Race),
            ));
        }
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

        particle.velocity.y -= GRAVITY * dt;
        transform.translation += particle.velocity * dt;

        let life_fraction = (particle.remaining / particle.lifetime).max(0.0);
        transform.scale = Vec3::splat(life_fraction);
    }
}

pub fn cleanup_explosion_sound(mut commands: Commands) {
    commands.remove_resource::<ExplosionSounds>();
}
