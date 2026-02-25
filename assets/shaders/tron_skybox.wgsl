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

// ─── Sky band thresholds ───
const THRESH_HORIZON: f32 = -0.01;
const THRESH_LOW: f32 = 0.15;
const THRESH_HIGH: f32 = 0.50;
const TRANS_HW: f32 = 0.14;

// ─── Halftone ───
const HALFTONE_ANGLE: f32 = 0.7854;
const HALFTONE_SCALE: f32 = 6.0;
const SMOOTHING: f32 = 0.3;

// ─── Hash functions ───
fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(443.897, 441.423, 437.195));
    p3 += dot(p3, p3.yzx + 19.19);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash21(p), hash21(p + vec2<f32>(127.1, 311.7)));
}

// ─── Value noise ───
fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm3(p: vec2<f32>) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i: i32 = 0; i < 3; i++) {
        val += amp * noise2d(pos);
        pos *= 2.2;
        amp *= 0.45;
    }
    return val;
}

// ─── Halftone blend ───
fn halftone_blend(dark: vec3<f32>, light: vec3<f32>, band_t: f32, screen_pos: vec2<f32>) -> vec3<f32> {
    let cs = cos(HALFTONE_ANGLE);
    let sn = sin(HALFTONE_ANGLE);
    let rotated = vec2<f32>(
        screen_pos.x * cs - screen_pos.y * sn,
        screen_pos.x * sn + screen_pos.y * cs,
    );

    let cell = floor(rotated / HALFTONE_SCALE);
    let cell_center = (cell + 0.5) * HALFTONE_SCALE;
    let dist = length(rotated - cell_center) / (HALFTONE_SCALE * 0.5);

    let dot_radius = 1.0 - band_t;
    let dot_mask = smoothstep(dot_radius - SMOOTHING, dot_radius + SMOOTHING, dist);

    return mix(dark, light, dot_mask);
}

// ─── Shooting stars ───
fn shooting_stars(theta: f32, phi: f32, time: f32) -> f32 {
    var brightness = 0.0;

    for (var i: i32 = 0; i < 3; i++) {
        let period = 3.5 + f32(i) * 2.7;
        let idx = floor(time / period);
        let phase = fract(time / period);

        if phase > 0.1 { continue; }
        let anim = phase / 0.1;

        let seed = vec2<f32>(idx * 17.3 + f32(i) * 53.1, f32(i) * 31.1);
        let s_theta = hash21(seed) * 6.283;
        let s_phi = 0.25 + hash21(seed + vec2<f32>(1.0, 0.0)) * 0.55;
        let e_theta = s_theta + (hash21(seed + vec2<f32>(2.0, 0.0)) - 0.5) * 0.6;
        let e_phi = s_phi - 0.08 - hash21(seed + vec2<f32>(3.0, 0.0)) * 0.12;

        for (var s: i32 = 0; s < 6; s++) {
            let ss = f32(s) / 6.0;
            let trail_t = max(anim - ss * 0.3, 0.0);
            let pt_theta = mix(s_theta, e_theta, trail_t);
            let pt_phi = mix(s_phi, e_phi, trail_t);

            var dtheta = theta - pt_theta;
            if dtheta > 3.14159 { dtheta -= 6.28318; }
            if dtheta < -3.14159 { dtheta += 6.28318; }

            let d = length(vec2<f32>(dtheta * cos(phi), phi - pt_phi));
            let trail_fade = 1.0 - ss;
            brightness += smoothstep(0.006, 0.0, d) * trail_fade * (1.0 - anim * 0.5);
        }
    }

    return clamp(brightness, 0.0, 1.5);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(mesh.world_position.xyz - sky.camera_pos);
    let elevation = dir.y;
    let screen_pos = mesh.position.xy;
    let theta = atan2(dir.z, dir.x);
    let phi = asin(clamp(dir.y, -1.0, 1.0));

    // Precompute moon proximity (used for star suppression + moon rendering)
    let moon_dir = normalize(sky.moon_dir);
    let moon_dot = dot(dir, moon_dir);

    // ─── Sky gradient with halftone transitions ───
    let below_color = sky.sky_dark.rgb * 0.5;
    var color: vec3<f32>;

    if elevation >= THRESH_HIGH + TRANS_HW {
        color = sky.sky_bright.rgb;
    } else if elevation >= THRESH_HIGH - TRANS_HW {
        let t = (elevation - (THRESH_HIGH - TRANS_HW)) / (TRANS_HW * 2.0);
        color = halftone_blend(sky.sky_mid.rgb, sky.sky_bright.rgb, t, screen_pos);
    } else if elevation >= THRESH_LOW + TRANS_HW {
        color = sky.sky_mid.rgb;
    } else if elevation >= THRESH_LOW - TRANS_HW {
        let t = (elevation - (THRESH_LOW - TRANS_HW)) / (TRANS_HW * 2.0);
        color = halftone_blend(sky.sky_dark.rgb, sky.sky_mid.rgb, t, screen_pos);
    } else if elevation >= THRESH_HORIZON + TRANS_HW {
        color = sky.sky_dark.rgb;
    } else if elevation >= THRESH_HORIZON - TRANS_HW {
        let t = (elevation - (THRESH_HORIZON - TRANS_HW)) / (TRANS_HW * 2.0);
        color = halftone_blend(below_color, sky.sky_dark.rgb, t, screen_pos);
    } else {
        color = below_color;
    }

    // ─── Nebula wisps (FBM noise, subtle colored patches) ───
    if elevation > 0.05 {
        let neb_uv = vec2<f32>(theta * 0.8, phi * 2.0);
        let drift = vec2<f32>(sky.time * 0.005, sky.time * 0.002);
        let n = fbm3(neb_uv * 1.8 + drift);
        let wisp = smoothstep(0.48, 0.62, n);
        let tint_select = hash21(floor(neb_uv * 2.0));
        let nebula_color = mix(
            vec3<f32>(0.12, 0.04, 0.22),  // deep purple
            vec3<f32>(0.04, 0.12, 0.20),  // teal
            tint_select
        );
        let height_fade = smoothstep(0.05, 0.2, elevation);
        color += nebula_color * wisp * height_fade * 0.35;
    }

    // ─── Aurora shimmer (neon curtains, cyan to magenta) ───
    if elevation > 0.15 && elevation < 0.75 {
        let wave1 = sin(theta * 3.0 + sky.time * 0.15) * 0.5 + 0.5;
        let wave2 = sin(theta * 6.0 - sky.time * 0.1 + 2.0) * 0.3;
        let distort = noise2d(vec2<f32>(theta * 2.0 + sky.time * 0.08, elevation * 5.0));
        let curtain = wave1 + wave2 + distort * 0.25;
        let mask = smoothstep(0.7, 0.9, curtain);
        let height_mask = smoothstep(0.15, 0.3, elevation) * smoothstep(0.75, 0.55, elevation);
        let aurora_color = mix(
            sky.neon_glow_color.rgb * 0.6,     // cyan at base
            vec3<f32>(0.5, 0.1, 0.6),          // magenta at top
            smoothstep(0.25, 0.6, elevation)
        );
        let shimmer = 0.8 + 0.2 * sin(sky.time * 0.5 + theta * 3.0);
        color += aurora_color * mask * height_mask * shimmer * 0.12;
    }

    // ─── Pulsing neon horizon glow ───
    if elevation > -0.02 && elevation < 0.12 {
        let glow_t = 1.0 - clamp((elevation + 0.02) / 0.14, 0.0, 1.0);
        let pulse = 1.0 + 0.15 * sin(sky.time * 0.4);
        color += sky.neon_glow_color.rgb * glow_t * glow_t * 0.25 * pulse;
    }

    // ─── Stars with color variety ───
    if elevation > 0.01 {
        // Suppress stars near moon
        let star_moon_fade = 1.0 - smoothstep(0.985, 0.995, moon_dot);

        let grid_scale = sky.star_density;
        let grid_uv = vec2<f32>(theta, phi) * grid_scale;
        let cell = floor(grid_uv);
        let cell_frac = fract(grid_uv);

        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                let neighbor = cell + vec2<f32>(f32(dx), f32(dy));
                let rnd = hash22(neighbor);

                if rnd.x < 0.12 {
                    let star_pos = vec2<f32>(f32(dx), f32(dy)) + rnd;
                    let d = length(cell_frac - star_pos);

                    let star_size = 0.03 + rnd.y * 0.07;
                    if d < star_size {
                        let twinkle = 0.3 + 0.7 * sin(sky.time * (1.0 + rnd.x * 4.0) + rnd.y * 6.2832);
                        let brightness = (1.0 - d / star_size) * twinkle;

                        // Color variety: star "temperature"
                        let temp = hash21(neighbor + vec2<f32>(73.7, 19.3));
                        var star_tint: vec3<f32>;
                        if temp < 0.15 {
                            star_tint = vec3<f32>(1.0, 0.75, 0.4);   // warm amber
                        } else if temp < 0.25 {
                            star_tint = vec3<f32>(0.6, 0.7, 1.0);   // cool blue-white
                        } else if temp < 0.30 {
                            star_tint = mix(sky.moon_color.rgb, sky.neon_glow_color.rgb, 0.5);  // neon cyan
                        } else {
                            star_tint = sky.moon_color.rgb;          // default warm white
                        }

                        color += star_tint * brightness * 0.9 * star_moon_fade;
                    }
                }
            }
        }

        // Shooting stars
        let streak = shooting_stars(theta, phi, sky.time);
        color += sky.moon_color.rgb * streak * 1.2 * star_moon_fade;
    }

    // ─── Moon (smooth continuous falloff, no rings) ───
    // Outer neon halo
    let halo_t = smoothstep(0.95, 0.985, moon_dot);
    color += sky.neon_glow_color.rgb * halo_t * halo_t * 0.05;

    // Inner warm glow
    let glow_t = smoothstep(0.975, 0.997, moon_dot);
    color += sky.moon_color.rgb * glow_t * glow_t * 0.25;

    // Core disc
    let core_t = smoothstep(0.993, 0.998, moon_dot);
    color = mix(color, sky.moon_color.rgb * 1.3, core_t);

    return vec4<f32>(color, 1.0);
}
