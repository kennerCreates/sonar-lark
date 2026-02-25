use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

const SHADER_PATH: &str = "shaders/cel_halftone.wgsl";
const DEFAULT_HALFTONE_SCALE: f32 = 5.0;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CelMaterial {
    #[uniform(0)]
    pub base_color: LinearRgba,
    #[uniform(0)]
    pub highlight_color: LinearRgba,
    #[uniform(0)]
    pub highlight2_color: LinearRgba,
    #[uniform(0)]
    pub shadow_color: LinearRgba,
    #[uniform(0)]
    pub shadow2_color: LinearRgba,
    #[uniform(0)]
    pub light_dir: Vec3,
    #[uniform(0)]
    pub halftone_scale: f32,
    #[uniform(0)]
    pub fog_color: LinearRgba,
    #[uniform(0)]
    pub fog_start: f32,
    #[uniform(0)]
    pub fog_end: f32,
}

impl Material for CelMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

/// Compute hue-shifted highlight and shadow colors from a base color.
///
/// Returns (highlight2, highlight, shadow, shadow2) — two levels each side.
/// Highlights shift warmer (boost red/green, reduce blue).
/// Shadows shift cooler (reduce red/green, boost blue).
pub fn compute_cel_colors(base: Color) -> (LinearRgba, LinearRgba, LinearRgba, LinearRgba) {
    let linear = base.to_linear();
    let r = linear.red;
    let g = linear.green;
    let b = linear.blue;

    let highlight2 = LinearRgba::new(
        (r * 1.25 + 0.08).min(1.0),
        (g * 1.15 + 0.06).min(1.0),
        (b * 0.75).min(1.0),
        linear.alpha,
    );

    let highlight = LinearRgba::new(
        (r * 1.10 + 0.03).min(1.0),
        (g * 1.05 + 0.02).min(1.0),
        (b * 0.92).min(1.0),
        linear.alpha,
    );

    let shadow = LinearRgba::new(
        (r * 0.75).max(0.0),
        (g * 0.75).max(0.0),
        (b * 0.82 + 0.02).min(1.0),
        linear.alpha,
    );

    let shadow2 = LinearRgba::new(
        (r * 0.40).max(0.0),
        (g * 0.40).max(0.0),
        (b * 0.55 + 0.04).min(1.0),
        linear.alpha,
    );

    (highlight2, highlight, shadow, shadow2)
}

/// Create a CelMaterial from a base color with precomputed hue-shifted highlight/shadow.
pub fn cel_material_from_color(base: Color, light_dir: Vec3) -> CelMaterial {
    let (highlight2, highlight, shadow, shadow2) = compute_cel_colors(base);
    CelMaterial {
        base_color: base.to_linear(),
        highlight_color: highlight,
        highlight2_color: highlight2,
        shadow_color: shadow,
        shadow2_color: shadow2,
        light_dir,
        halftone_scale: DEFAULT_HALFTONE_SCALE,
        fog_color: super::fog_color().to_linear(),
        fog_start: super::FOG_START,
        fog_end: super::FOG_END,
    }
}

/// Create a flat CelMaterial that ignores lighting — all bands show the base color.
/// Use for surfaces with uniform normals (e.g. ground plane) where cel-shading
/// produces a single solid band instead of useful shading variation.
pub fn cel_material_flat(base: Color, light_dir: Vec3) -> CelMaterial {
    let linear = base.to_linear();
    CelMaterial {
        base_color: linear,
        highlight_color: linear,
        highlight2_color: linear,
        shadow_color: linear,
        shadow2_color: linear,
        light_dir,
        halftone_scale: DEFAULT_HALFTONE_SCALE,
        fog_color: super::fog_color().to_linear(),
        fog_start: super::FOG_START,
        fog_end: super::FOG_END,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: &LinearRgba) -> f32 {
        c.red * 0.299 + c.green * 0.587 + c.blue * 0.114
    }

    #[test]
    fn highlights_are_warmer_and_lighter() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let (h2, h1, _s1, _s2) = compute_cel_colors(base);
        let base_lin = base.to_linear();
        // Both highlight levels should be brighter than base
        assert!(luminance(&h1) > luminance(&base_lin));
        assert!(luminance(&h2) > luminance(&h1));
        // Warm shift: red boosted, blue reduced
        assert!(h1.red >= base_lin.red);
        assert!(h1.blue <= base_lin.blue + 0.01);
    }

    #[test]
    fn shadows_are_cooler_and_darker() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let (_h2, _h1, s1, s2) = compute_cel_colors(base);
        let base_lin = base.to_linear();
        // Both shadow levels should be darker than base
        assert!(luminance(&s1) < luminance(&base_lin));
        assert!(luminance(&s2) < luminance(&s1));
        // Cool shift: red/green reduced
        assert!(s1.red <= base_lin.red);
        assert!(s1.green <= base_lin.green);
    }

    #[test]
    fn luminance_ordering() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let (h2, h1, s1, s2) = compute_cel_colors(base);
        let base_lin = base.to_linear();
        assert!(luminance(&h2) > luminance(&h1));
        assert!(luminance(&h1) > luminance(&base_lin));
        assert!(luminance(&base_lin) > luminance(&s1));
        assert!(luminance(&s1) > luminance(&s2));
    }

    #[test]
    fn colors_stay_in_range() {
        for color in [
            Color::srgb(1.0, 1.0, 1.0),
            Color::srgb(0.0, 0.0, 0.0),
            Color::srgb(1.0, 0.0, 0.0),
            Color::srgb(0.0, 0.0, 1.0),
        ] {
            let (h2, h1, s1, s2) = compute_cel_colors(color);
            for c in [h2, h1, s1, s2] {
                assert!(c.red >= 0.0 && c.red <= 1.0);
                assert!(c.green >= 0.0 && c.green <= 1.0);
                assert!(c.blue >= 0.0 && c.blue <= 1.0);
            }
        }
    }
}
