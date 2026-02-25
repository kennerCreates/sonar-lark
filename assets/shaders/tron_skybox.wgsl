#import bevy_pbr::forward_io::VertexOutput

struct SkyboxUniforms {
    sky_dark: vec4<f32>,
    sky_mid: vec4<f32>,
    sky_bright: vec4<f32>,
    moon_color: vec4<f32>,
    neon_glow_color: vec4<f32>,
    moon_dir: vec3<f32>,
    star_density: f32,
    camera_pos: vec3<f32>,
    time: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> sky: SkyboxUniforms;

// Hash functions for deterministic star placement
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(443.897, 441.423, 437.195));
    p3 += dot(p3, p3.yzx + 19.19);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash21(p), hash21(p + vec2<f32>(127.1, 311.7)));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Direction from camera to fragment on the sky sphere
    let dir = normalize(mesh.world_position.xyz - sky.camera_pos);
    let elevation = dir.y;

    // --- Sky gradient ---
    var color: vec3<f32>;
    if elevation < -0.02 {
        // Below horizon: dark ground reflection
        color = sky.sky_dark.rgb * 0.5;
    } else if elevation < 0.12 {
        // Horizon band: dark -> mid with neon glow
        let t = clamp((elevation + 0.02) / 0.14, 0.0, 1.0);
        color = mix(sky.sky_dark.rgb, sky.sky_mid.rgb, t);
        // Neon horizon glow (strongest at horizon, fading upward)
        let glow_t = 1.0 - t;
        color += sky.neon_glow_color.rgb * glow_t * glow_t * 0.25;
    } else {
        // Upper sky: mid -> slightly brighter toward zenith
        let t = clamp((elevation - 0.12) / 0.88, 0.0, 1.0);
        color = mix(sky.sky_mid.rgb, sky.sky_bright.rgb, sqrt(t));
    }

    // --- Stars (above horizon only) ---
    if elevation > 0.01 {
        // Spherical coordinate grid for star placement
        let theta = atan2(dir.z, dir.x);
        let phi = asin(clamp(dir.y, -1.0, 1.0));
        let grid_scale = sky.star_density;
        let grid_uv = vec2<f32>(theta, phi) * grid_scale;
        let cell = floor(grid_uv);
        let cell_frac = fract(grid_uv);

        // Check 3x3 neighborhood for smooth star rendering across cell boundaries
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                let neighbor = cell + vec2<f32>(f32(dx), f32(dy));
                let rnd = hash22(neighbor);

                // ~12% of cells contain a star
                if rnd.x < 0.12 {
                    let star_pos = vec2<f32>(f32(dx), f32(dy)) + rnd;
                    let d = length(cell_frac - star_pos);

                    // Star size varies per star
                    let star_size = 0.015 + rnd.y * 0.035;
                    if d < star_size {
                        // Twinkle animation
                        let twinkle = 0.6 + 0.4 * sin(sky.time * (1.5 + rnd.x * 5.0) + rnd.y * 6.2832);
                        let brightness = (1.0 - d / star_size) * twinkle;

                        // Star color: mostly warm white, some neon-tinted
                        let star_tint = select(
                            sky.moon_color.rgb,                                          // Vanilla white
                            mix(sky.moon_color.rgb, sky.neon_glow_color.rgb, 0.5),       // Cyan-tinted
                            rnd.x < 0.04
                        );
                        color += star_tint * brightness * 0.9;
                    }
                }
            }
        }
    }

    // --- Moon disc + glow ---
    let moon_dir = normalize(sky.moon_dir);
    let moon_dot = dot(dir, moon_dir);

    if moon_dot > 0.994 {
        // Moon disc: sharp bright circle
        let moon_t = (moon_dot - 0.994) / 0.006;
        let moon_blend = clamp(moon_t * moon_t, 0.0, 1.0);
        color = mix(color, sky.moon_color.rgb * 1.3, moon_blend);
    } else if moon_dot > 0.98 {
        // Inner glow halo
        let glow_t = (moon_dot - 0.98) / 0.014;
        color += sky.moon_color.rgb * glow_t * glow_t * 0.2;
    } else if moon_dot > 0.95 {
        // Outer subtle glow
        let outer_t = (moon_dot - 0.95) / 0.03;
        color += sky.neon_glow_color.rgb * outer_t * outer_t * 0.04;
    }

    return vec4<f32>(color, 1.0);
}
