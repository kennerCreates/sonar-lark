use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face};
use bevy::shader::ShaderRef;

const SHADER_PATH: &str = "shaders/tron_skybox.wgsl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyboxMaterial {
    #[uniform(0)]
    pub sky_dark: LinearRgba,
    #[uniform(0)]
    pub sky_mid: LinearRgba,
    #[uniform(0)]
    pub sky_bright: LinearRgba,
    #[uniform(0)]
    pub moon_color: LinearRgba,
    #[uniform(0)]
    pub neon_glow_color: LinearRgba,
    #[uniform(0)]
    pub moon_dir: Vec3,
    #[uniform(0)]
    pub star_density: f32,
    #[uniform(0)]
    pub camera_pos: Vec3,
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub fog_color: LinearRgba,
}

impl Material for SkyboxMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Render inside faces of the sphere (camera is inside the skybox)
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

#[derive(Component)]
pub struct SkyboxEntity;

pub fn spawn_skybox(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    skybox_materials: &mut Assets<SkyboxMaterial>,
) {
    // Palette colors for TRON night sky
    let sky_dark = Color::srgb(0.020, 0.055, 0.102);     // Smoky Black #050e1a
    let sky_mid = Color::srgb(0.051, 0.129, 0.251);       // Indigo #0d2140
    let sky_bright = Color::srgb(0.110, 0.157, 0.302);    // Space Cadet #1c284d
    let moon_color = Color::srgb(0.949, 0.949, 0.855);    // Vanilla #f2f2da
    let neon_glow = Color::srgb(0.286, 0.761, 0.949);     // Sky #49c2f2

    let sphere = meshes.add(Sphere::new(2000.0).mesh().ico(5).unwrap());
    let material = skybox_materials.add(SkyboxMaterial {
        sky_dark: sky_dark.to_linear(),
        sky_mid: sky_mid.to_linear(),
        sky_bright: sky_bright.to_linear(),
        moon_color: moon_color.to_linear(),
        neon_glow_color: neon_glow.to_linear(),
        moon_dir: Vec3::new(0.3, 0.65, -0.5).normalize(),
        star_density: 120.0,
        camera_pos: Vec3::ZERO,
        time: 0.0,
        fog_color: super::fog_color().to_linear(),
    });

    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        SkyboxEntity,
    ));
}

/// Keep skybox centered on the camera and update time for star twinkle.
pub fn update_skybox(
    time: Res<Time>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut skybox_query: Query<&mut Transform, With<SkyboxEntity>>,
    skybox_mat_query: Query<&MeshMaterial3d<SkyboxMaterial>, With<SkyboxEntity>>,
    mut skybox_materials: ResMut<Assets<SkyboxMaterial>>,
) {
    let Ok(cam_gt) = camera_query.single() else {
        return;
    };
    let cam_pos = cam_gt.translation();

    // Move skybox sphere to camera position
    for mut sky_transform in &mut skybox_query {
        sky_transform.translation = cam_pos;
    }

    // Update material uniforms
    for mat_handle in &skybox_mat_query {
        if let Some(mat) = skybox_materials.get_mut(&mat_handle.0) {
            mat.camera_pos = cam_pos;
            mat.time = time.elapsed_secs();
        }
    }
}
