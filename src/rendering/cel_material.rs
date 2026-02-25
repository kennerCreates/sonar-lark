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
    pub shadow_color: LinearRgba,
    #[uniform(0)]
    pub light_dir: Vec3,
    #[uniform(0)]
    pub halftone_scale: f32,
}

impl Material for CelMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

/// Compute hue-shifted highlight and shadow colors from a base color.
///
/// Highlight: warmer (boost red/green), lighter.
/// Shadow: cooler (boost blue, reduce red/green), darker.
pub fn compute_cel_colors(base: Color) -> (LinearRgba, LinearRgba) {
    let linear = base.to_linear();
    let r = linear.red;
    let g = linear.green;
    let b = linear.blue;

    let highlight = LinearRgba::new(
        (r * 1.15 + 0.06).min(1.0),
        (g * 1.10 + 0.04).min(1.0),
        (b * 0.85).min(1.0),
        linear.alpha,
    );

    let shadow = LinearRgba::new(
        (r * 0.50).max(0.0),
        (g * 0.50).max(0.0),
        (b * 0.65 + 0.04).min(1.0),
        linear.alpha,
    );

    (highlight, shadow)
}

/// Create a CelMaterial from a base color with precomputed hue-shifted highlight/shadow.
pub fn cel_material_from_color(base: Color, light_dir: Vec3) -> CelMaterial {
    let (highlight, shadow) = compute_cel_colors(base);
    CelMaterial {
        base_color: base.to_linear(),
        highlight_color: highlight,
        shadow_color: shadow,
        light_dir,
        halftone_scale: DEFAULT_HALFTONE_SCALE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_is_warmer_and_lighter() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let (highlight, _shadow) = compute_cel_colors(base);
        let base_lin = base.to_linear();
        assert!(
            highlight.red >= base_lin.red,
            "highlight red should be >= base red"
        );
        assert!(
            highlight.green >= base_lin.green,
            "highlight green should be >= base green"
        );
        assert!(
            highlight.blue <= base_lin.blue + 0.01,
            "highlight blue should not increase much"
        );
    }

    #[test]
    fn shadow_is_cooler_and_darker() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let (_highlight, shadow) = compute_cel_colors(base);
        let base_lin = base.to_linear();
        assert!(
            shadow.red <= base_lin.red,
            "shadow red should be <= base red"
        );
        assert!(
            shadow.green <= base_lin.green,
            "shadow green should be <= base green"
        );
        // Blue channel gets a slight boost for cooler tone
        let luminance_shadow = shadow.red * 0.299 + shadow.green * 0.587 + shadow.blue * 0.114;
        let luminance_base =
            base_lin.red * 0.299 + base_lin.green * 0.587 + base_lin.blue * 0.114;
        assert!(
            luminance_shadow < luminance_base,
            "shadow luminance ({luminance_shadow}) should be darker than base ({luminance_base})"
        );
    }

    #[test]
    fn colors_stay_in_range() {
        // Test with extreme colors
        for color in [
            Color::srgb(1.0, 1.0, 1.0),
            Color::srgb(0.0, 0.0, 0.0),
            Color::srgb(1.0, 0.0, 0.0),
            Color::srgb(0.0, 0.0, 1.0),
        ] {
            let (h, s) = compute_cel_colors(color);
            assert!(h.red >= 0.0 && h.red <= 1.0);
            assert!(h.green >= 0.0 && h.green <= 1.0);
            assert!(h.blue >= 0.0 && h.blue <= 1.0);
            assert!(s.red >= 0.0 && s.red <= 1.0);
            assert!(s.green >= 0.0 && s.green <= 1.0);
            assert!(s.blue >= 0.0 && s.blue <= 1.0);
        }
    }
}
