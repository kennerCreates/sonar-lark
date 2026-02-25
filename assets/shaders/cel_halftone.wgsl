#import bevy_pbr::forward_io::VertexOutput

struct CelMaterial {
    base_color: vec4<f32>,
    highlight_color: vec4<f32>,
    shadow_color: vec4<f32>,
    light_dir: vec3<f32>,
    halftone_scale: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: CelMaterial;

const HIGHLIGHT_THRESHOLD: f32 = 0.62;
const SHADOW_THRESHOLD: f32 = 0.38;
const HALFTONE_ANGLE: f32 = 0.7854; // 45 degrees for classic halftone rotation
const SMOOTHING: f32 = 0.4;         // Anti-alias softness at dot edges

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(mesh.world_normal);
    let light_dir = normalize(material.light_dir);

    // Half-Lambert shading: NdotL remapped to 0..1 for softer falloff
    let ndotl = dot(normal, light_dir) * 0.5 + 0.5;

    // Screen-space coordinates for halftone pattern
    let screen_pos = mesh.position.xy;

    var final_color: vec3<f32>;

    if ndotl >= HIGHLIGHT_THRESHOLD {
        // Full highlight band
        final_color = material.highlight_color.rgb;
    } else if ndotl >= SHADOW_THRESHOLD {
        // Halftone transition band
        // Remap threshold range to 0..1 (0 = shadow edge, 1 = highlight edge)
        let band_t = (ndotl - SHADOW_THRESHOLD) / (HIGHLIGHT_THRESHOLD - SHADOW_THRESHOLD);

        // Rotate screen coordinates for classic halftone angle
        let cs = cos(HALFTONE_ANGLE);
        let sn = sin(HALFTONE_ANGLE);
        let rotated = vec2<f32>(
            screen_pos.x * cs - screen_pos.y * sn,
            screen_pos.x * sn + screen_pos.y * cs,
        );

        // Grid cell coordinates
        let cell = floor(rotated / material.halftone_scale);
        let cell_center = (cell + 0.5) * material.halftone_scale;

        // Distance from fragment to nearest cell center, normalized
        let dist = length(rotated - cell_center) / (material.halftone_scale * 0.5);

        // Dot radius: larger dots in darker areas
        // band_t=1 (near highlight) => small dots (radius~0), band_t=0 (near shadow) => big dots (radius~1)
        let dot_radius = 1.0 - band_t;

        // Smooth step for anti-aliased dot edges
        let dot_mask = smoothstep(dot_radius - SMOOTHING, dot_radius + SMOOTHING, dist);

        // dot_mask=1 outside dot (highlight), dot_mask=0 inside dot (shadow)
        final_color = mix(material.shadow_color.rgb, material.highlight_color.rgb, dot_mask);
    } else {
        // Full shadow band
        final_color = material.shadow_color.rgb;
    }

    return vec4<f32>(final_color, material.base_color.a);
}
