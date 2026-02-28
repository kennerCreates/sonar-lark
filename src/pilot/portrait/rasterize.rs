use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::PortraitDescriptor;
use super::fragments::assemble_svg;

/// Rasterize a pilot portrait SVG into a Bevy `Image`.
///
/// Assembles the SVG from the descriptor, parses it with `usvg`, renders it with
/// `resvg` into a `tiny_skia::Pixmap`, and converts the resulting RGBA bytes into
/// a Bevy `Image` suitable for UI display.
///
/// On any failure (SVG parse error, render error), returns a solid-color fallback
/// image filled with `bg_color`.
pub fn rasterize_portrait(descriptor: &PortraitDescriptor, bg_color: [f32; 3], size: u32) -> Image {
    let svg_string = assemble_svg(descriptor, bg_color);

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

    #[test]
    fn rasterize_portrait_produces_correct_size() {
        let mut rng = rand::thread_rng();
        let desc = super::super::PortraitDescriptor::generate(&mut rng, [0.8, 0.2, 0.4]);
        let img = rasterize_portrait(&desc, [0.8, 0.2, 0.4], 48);
        assert_eq!(img.width(), 48);
        assert_eq!(img.height(), 48);
        let data = img.data.as_ref().unwrap();
        assert_eq!(data.len(), 48 * 48 * 4);
        // Verify non-zero data (not all black)
        assert!(data.iter().any(|&b| b > 0));
    }
}
