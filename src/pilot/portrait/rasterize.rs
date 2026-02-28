use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::PortraitDescriptor;
use super::fragments::assemble_svg;
use super::loader::PortraitParts;

/// Rasterize a pilot portrait SVG into a Bevy `Image`.
///
/// Assembles the SVG from the descriptor and loaded parts, parses it with `usvg`,
/// renders it with `resvg` into a `tiny_skia::Pixmap`, and converts the resulting
/// RGBA bytes into a Bevy `Image` suitable for UI display.
///
/// On any failure (SVG parse error, render error), returns a solid-color fallback
/// image filled with `bg_color`.
pub fn rasterize_portrait(
    descriptor: &PortraitDescriptor,
    bg_color: [f32; 3],
    size: u32,
    parts: &PortraitParts,
) -> Image {
    let svg_string = assemble_svg(descriptor, bg_color, parts);

    match rasterize_svg(&svg_string, size) {
        Ok(image) => image,
        Err(e) => {
            bevy::log::warn!("Portrait rasterization failed: {e}. Using solid-color fallback.");
            solid_color_image(bg_color, size)
        }
    }
}

/// Parse and render an SVG string into a Bevy `Image`.
fn rasterize_svg(svg: &str, size: u32) -> Result<Image, String> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .map_err(|e| format!("SVG parse error: {e}"))?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Scale the SVG to fill the entire pixmap. Without this, the SVG renders
    // at its intrinsic size (viewBox units ≈ 20×20) into a 48×48 pixmap,
    // leaving most of the image transparent.
    let tree_size = tree.size();
    let sx = size as f32 / tree_size.width();
    let sy = size as f32 / tree_size.height();
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // resvg outputs premultiplied RGBA. Convert to straight alpha for Bevy's
    // Rgba8UnormSrgb format, which expects non-premultiplied data.
    let mut data = pixmap.data().to_vec();
    unpremultiply_alpha(&mut data);

    Ok(rgba_to_image(&data, size))
}

/// Convert premultiplied RGBA to straight (non-premultiplied) RGBA in place.
fn unpremultiply_alpha(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let a = pixel[3];
        if a == 0 {
            // Fully transparent -- leave RGB as zero
            continue;
        }
        if a == 255 {
            // Fully opaque -- no conversion needed
            continue;
        }
        // Undo premultiplication: C = C_premul * 255 / A
        let a_f = a as f32;
        pixel[0] = (pixel[0] as f32 * 255.0 / a_f) as u8;
        pixel[1] = (pixel[1] as f32 * 255.0 / a_f) as u8;
        pixel[2] = (pixel[2] as f32 * 255.0 / a_f) as u8;
    }
}

/// Create a Bevy `Image` from raw RGBA bytes at a given square size.
fn rgba_to_image(data: &[u8], size: u32) -> Image {
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data.to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Create a solid-color fallback image when SVG rasterization fails.
fn solid_color_image(color: [f32; 3], size: u32) -> Image {
    let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
    let pixel = [r, g, b, 255u8];

    let pixel_count = (size * size) as usize;
    let mut data = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        data.extend_from_slice(&pixel);
    }

    rgba_to_image(&data, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_color_image_correct_dimensions() {
        let img = solid_color_image([1.0, 0.0, 0.0], 16);
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 16);
        let data = img.data.as_ref().unwrap();
        assert_eq!(data.len(), 16 * 16 * 4);
    }

    #[test]
    fn solid_color_image_fills_correctly() {
        let img = solid_color_image([1.0, 0.5, 0.0], 2);
        let data = img.data.as_ref().unwrap();
        let r = (1.0f32 * 255.0) as u8;
        let g = (0.5f32 * 255.0) as u8;
        let b = 0u8;
        // Check first pixel
        assert_eq!(data[0], r);
        assert_eq!(data[1], g);
        assert_eq!(data[2], b);
        assert_eq!(data[3], 255);
        // Check last pixel
        let last = data.len() - 4;
        assert_eq!(data[last], r);
        assert_eq!(data[last + 1], g);
        assert_eq!(data[last + 2], b);
        assert_eq!(data[last + 3], 255);
    }

    #[test]
    fn unpremultiply_fully_opaque_is_noop() {
        let mut data = vec![128, 64, 32, 255];
        unpremultiply_alpha(&mut data);
        assert_eq!(data, vec![128, 64, 32, 255]);
    }

    #[test]
    fn unpremultiply_fully_transparent_stays_zero() {
        let mut data = vec![0, 0, 0, 0];
        unpremultiply_alpha(&mut data);
        assert_eq!(data, vec![0, 0, 0, 0]);
    }

    #[test]
    fn unpremultiply_half_alpha() {
        // Premultiplied: R=64, A=128 → straight: R = 64 * 255 / 128 = 127
        let mut data = vec![64, 64, 64, 128];
        unpremultiply_alpha(&mut data);
        assert_eq!(data[0], 127);
        assert_eq!(data[1], 127);
        assert_eq!(data[2], 127);
        assert_eq!(data[3], 128);
    }

    #[test]
    fn solid_color_clamps_out_of_range() {
        let img = solid_color_image([1.5, -0.5, 0.5], 1);
        let data = img.data.as_ref().unwrap();
        assert_eq!(data[0], 255); // clamped to 1.0
        assert_eq!(data[1], 0); // clamped to 0.0
        assert_eq!(data[2], 127); // 0.5 * 255
        assert_eq!(data[3], 255);
    }

    /// Build minimal `PortraitParts` with hex-colored SVG content for rasterization tests.
    fn test_parts() -> PortraitParts {
        let mut parts = PortraitParts::default();
        let bg = r##"<rect width="20" height="20" rx="1.5" ry="1.5" fill="#808080"/>"##;
        parts.insert("background", bg);
        for v in &["oval", "round", "square", "angular"] {
            let s = r##"<ellipse cx="10" cy="8" rx="4" ry="5" fill="#000000"/>"##;
            parts.insert(format!("face_{v}"), s);
        }
        for v in &["normal", "narrow", "wide", "visor"] {
            let s = r##"<circle cx="8" cy="7" r="1" fill="#808080"/><circle cx="8" cy="7" r="0.5" fill="#ffffff"/>"##;
            parts.insert(format!("eyes_{v}"), s);
        }
        for v in &["neutral", "smile", "smirk", "frown"] {
            let s = r##"<path d="M8 11 Q10 12 12 11" stroke="#000000" fill="none"/>"##;
            parts.insert(format!("mouth_{v}"), s);
        }
        for v in &["short_crop", "long_sweep", "beanie"] {
            let s = r##"<ellipse cx="10" cy="5" rx="5" ry="3" fill="#000000"/>"##;
            parts.insert(format!("hair_back_{v}"), s);
        }
        for v in &["short_crop", "mohawk", "long_sweep", "beanie"] {
            let s = r##"<path d="M5 5 Q10 2 15 5" fill="#000000"/>"##;
            parts.insert(format!("hair_front_{v}"), s);
        }
        for v in &["crew", "round", "turtleneck", "vneck"] {
            let s = r##"<rect x="4" y="14" width="12" height="6" fill="#000000"/>"##;
            parts.insert(format!("shirt_{v}"), s);
        }
        for v in &["earring_round", "earring_ring", "necklace_chain", "necklace_pendant"] {
            let s = r##"<circle cx="10" cy="14" r="0.5" fill="#000000"/>"##;
            parts.insert(format!("acc_{v}"), s);
        }
        parts
    }

    #[test]
    fn rasterize_portrait_produces_correct_size() {
        let mut rng = rand::thread_rng();
        let desc = super::super::PortraitDescriptor::generate(&mut rng, [0.8, 0.2, 0.4]);
        let parts = test_parts();
        let img = rasterize_portrait(&desc, [0.8, 0.2, 0.4], 48, &parts);
        assert_eq!(img.width(), 48);
        assert_eq!(img.height(), 48);
        let data = img.data.as_ref().unwrap();
        assert_eq!(data.len(), 48 * 48 * 4);
        // Verify non-zero data (not all black)
        assert!(data.iter().any(|&b| b > 0));
    }
}
