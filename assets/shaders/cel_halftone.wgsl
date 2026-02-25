#import bevy_pbr::forward_io::VertexOutput

struct CelMaterial {
    base_color: vec4<f32>,
    highlight_color: vec4<f32>,
    highlight2_color: vec4<f32>,
    shadow_color: vec4<f32>,
    shadow2_color: vec4<f32>,
    light_dir: vec3<f32>,
    halftone_scale: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: CelMaterial;

// Band boundaries (Half-Lambert ndotl space, 0..1)
// 5 solid bands: highlight2 | highlight | base | shadow | shadow2
const THRESH_H2: f32 = 0.78;
const THRESH_H1: f32 = 0.58;
const THRESH_S1: f32 = 0.38;
const THRESH_S2: f32 = 0.18;

// Half-width of each halftone transition zone around a threshold
const TRANS_HW: f32 = 0.04;

const HALFTONE_ANGLE: f32 = 0.7854; // 45 degrees
const SMOOTHING: f32 = 0.4;         // Anti-alias softness at dot edges

fn halftone_blend(dark: vec3<f32>, light: vec3<f32>, band_t: f32, screen_pos: vec2<f32>) -> vec3<f32> {
    let cs = cos(HALFTONE_ANGLE);
    let sn = sin(HALFTONE_ANGLE);
    let rotated = vec2<f32>(
        screen_pos.x * cs - screen_pos.y * sn,
        screen_pos.x * sn + screen_pos.y * cs,
    );

    let cell = floor(rotated / material.halftone_scale);
    let cell_center = (cell + 0.5) * material.halftone_scale;
    let dist = length(rotated - cell_center) / (material.halftone_scale * 0.5);

    // band_t=1 (near light side) => small dots (radius~0), band_t=0 (near dark side) => big dots
    let dot_radius = 1.0 - band_t;
    let dot_mask = smoothstep(dot_radius - SMOOTHING, dot_radius + SMOOTHING, dist);

    // dot_mask=1 outside dot (light color), dot_mask=0 inside dot (dark color)
    return mix(dark, light, dot_mask);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(mesh.world_normal);
    let light_dir = normalize(material.light_dir);

    // Half-Lambert shading: NdotL remapped to 0..1 for softer falloff
    let ndotl = dot(normal, light_dir) * 0.5 + 0.5;

    let screen_pos = mesh.position.xy;
    var final_color: vec3<f32>;

    if ndotl >= THRESH_H2 + TRANS_HW {
        // Solid highlight2
        final_color = material.highlight2_color.rgb;
    } else if ndotl >= THRESH_H2 - TRANS_HW {
        // Halftone: highlight → highlight2
        let t = (ndotl - (THRESH_H2 - TRANS_HW)) / (TRANS_HW * 2.0);
        final_color = halftone_blend(material.highlight_color.rgb, material.highlight2_color.rgb, t, screen_pos);
    } else if ndotl >= THRESH_H1 + TRANS_HW {
        // Solid highlight
        final_color = material.highlight_color.rgb;
    } else if ndotl >= THRESH_H1 - TRANS_HW {
        // Halftone: base → highlight
        let t = (ndotl - (THRESH_H1 - TRANS_HW)) / (TRANS_HW * 2.0);
        final_color = halftone_blend(material.base_color.rgb, material.highlight_color.rgb, t, screen_pos);
    } else if ndotl >= THRESH_S1 + TRANS_HW {
        // Solid base
        final_color = material.base_color.rgb;
    } else if ndotl >= THRESH_S1 - TRANS_HW {
        // Halftone: shadow → base
        let t = (ndotl - (THRESH_S1 - TRANS_HW)) / (TRANS_HW * 2.0);
        final_color = halftone_blend(material.shadow_color.rgb, material.base_color.rgb, t, screen_pos);
    } else if ndotl >= THRESH_S2 + TRANS_HW {
        // Solid shadow
        final_color = material.shadow_color.rgb;
    } else if ndotl >= THRESH_S2 - TRANS_HW {
        // Halftone: shadow2 → shadow
        let t = (ndotl - (THRESH_S2 - TRANS_HW)) / (TRANS_HW * 2.0);
        final_color = halftone_blend(material.shadow2_color.rgb, material.shadow_color.rgb, t, screen_pos);
    } else {
        // Solid shadow2
        final_color = material.shadow2_color.rgb;
    }

    return vec4<f32>(final_color, material.base_color.a);
}
