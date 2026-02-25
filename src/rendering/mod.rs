pub mod cel_material;
pub mod skybox;

use bevy::prelude::*;

use crate::palette;
pub use cel_material::{CelMaterial, cel_material_flat, cel_material_from_color};
pub use skybox::SkyboxMaterial;

/// Shared fog parameters — single source of truth for cel shader, skybox, and camera fog.
pub fn fog_color() -> Color {
    palette::SAPPHIRE
}
pub const FOG_START: f32 = 200.0;
pub const FOG_END: f32 = 500.0;

/// World-space light direction for cel materials (toward the light source).
/// Computed once from the DirectionalLight transform and shared by all CelMaterial instances.
#[derive(Resource)]
pub struct CelLightDir(pub Vec3);

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CelMaterial>::default())
            .add_plugins(MaterialPlugin::<SkyboxMaterial>::default())
            .add_systems(Update, skybox::update_skybox);
    }
}

/// Compute the world-space direction toward the light from a DirectionalLight transform.
/// DirectionalLight shines along its local -Z axis.
pub fn light_direction_from_transform(transform: &Transform) -> Vec3 {
    -(transform.rotation * Vec3::NEG_Z)
}
