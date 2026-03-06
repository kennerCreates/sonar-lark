use bevy::prelude::default;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::{PaintStroke, PosterAction};

pub const CANVAS_WIDTH: u32 = 400;
pub const CANVAS_HEIGHT: u32 = 600;

/// Display dimensions in UI pixels (matching 2:3 portrait aspect ratio).
pub const CANVAS_DISPLAY_WIDTH: f32 = 450.0;
pub const CANVAS_DISPLAY_HEIGHT: f32 = 675.0;

/// Create a blank white RGBA canvas image.
pub fn create_blank_canvas() -> Image {
    let pixel_count = (CANVAS_WIDTH * CANVAS_HEIGHT) as usize;
    let data = vec![255u8; pixel_count * 4];
    Image::new(
        Extent3d {
            width: CANVAS_WIDTH,
            height: CANVAS_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

/// Paint a filled circle at (cx, cy) with given radius and color.
pub fn paint_circle(
    data: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: [u8; 4],
) {
    let r2 = radius * radius;
    let min_x = ((cx - radius).floor() as i32).max(0) as u32;
    let max_x = ((cx + radius).ceil() as i32).min(width as i32 - 1).max(0) as u32;
    let min_y = ((cy - radius).floor() as i32).max(0) as u32;
    let max_y = ((cy + radius).ceil() as i32).min(height as i32 - 1).max(0) as u32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                let idx = ((y * width + x) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

/// Paint an entire stroke (interpolating between consecutive points).
pub fn paint_stroke(data: &mut [u8], width: u32, height: u32, stroke: &PaintStroke) {
    if stroke.points.is_empty() {
        return;
    }
    paint_circle(
        data,
        width,
        height,
        stroke.points[0][0],
        stroke.points[0][1],
        stroke.radius,
        stroke.color,
    );
    for pair in stroke.points.windows(2) {
        let [x0, y0] = pair[0];
        let [x1, y1] = pair[1];
        let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let step = (stroke.radius * 0.3).max(1.0);
        let steps = (dist / step).ceil() as u32;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let cx = x0 + (x1 - x0) * t;
            let cy = y0 + (y1 - y0) * t;
            paint_circle(data, width, height, cx, cy, stroke.radius, stroke.color);
        }
    }
}

/// Flood fill from (start_x, start_y) with the given color.
/// Replaces all connected pixels matching the target color.
pub fn flood_fill(data: &mut [u8], width: u32, height: u32, start_x: u32, start_y: u32, color: [u8; 4]) {
    if start_x >= width || start_y >= height {
        return;
    }

    let idx = ((start_y * width + start_x) * 4) as usize;
    let target = [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]];

    if target == color {
        return;
    }

    let mut stack = vec![(start_x, start_y)];
    while let Some((x, y)) = stack.pop() {
        let i = ((y * width + x) * 4) as usize;
        if data[i..i + 4] != target {
            continue;
        }
        data[i..i + 4].copy_from_slice(&color);

        if x > 0 { stack.push((x - 1, y)); }
        if x + 1 < width { stack.push((x + 1, y)); }
        if y > 0 { stack.push((x, y - 1)); }
        if y + 1 < height { stack.push((x, y + 1)); }
    }
}

/// Replay all actions onto a fresh white canvas. Returns the new pixel data.
pub fn replay_actions(actions: &[PosterAction], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut data = vec![255u8; pixel_count * 4];
    for action in actions {
        match action {
            PosterAction::Stroke(stroke) => paint_stroke(&mut data, width, height, stroke),
            PosterAction::Fill { x, y, color } => flood_fill(&mut data, width, height, *x, *y, *color),
        }
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_canvas_is_all_white() {
        let img = create_blank_canvas();
        let data = img.data.as_ref().unwrap();
        assert_eq!(data.len(), (CANVAS_WIDTH * CANVAS_HEIGHT * 4) as usize);
        assert!(data.iter().all(|&b| b == 255));
    }

    #[test]
    fn paint_circle_center() {
        let w = 16;
        let h = 16;
        let mut data = vec![255u8; (w * h * 4) as usize];
        paint_circle(&mut data, w, h, 8.0, 8.0, 2.0, [0, 0, 0, 255]);
        // Center pixel should be black
        let idx = ((8 * w + 8) * 4) as usize;
        assert_eq!(data[idx], 0);
        assert_eq!(data[idx + 3], 255);
        // Corner pixel (0,0) should still be white
        assert_eq!(data[0], 255);
    }

    #[test]
    fn paint_circle_clips_at_edge() {
        let w = 8;
        let h = 8;
        let mut data = vec![255u8; (w * h * 4) as usize];
        // Circle centered at (0,0) with radius 3 — should not panic
        paint_circle(&mut data, w, h, 0.0, 0.0, 3.0, [255, 0, 0, 255]);
        // Origin pixel should be red
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 0);
    }

    #[test]
    fn replay_empty_is_white() {
        let data = replay_actions(&[], 8, 8);
        assert!(data.iter().all(|&b| b == 255));
    }

    #[test]
    fn replay_matches_sequential_painting() {
        let w = 32;
        let h = 32;
        let strokes = vec![
            PaintStroke {
                points: vec![[5.0, 5.0], [10.0, 5.0]],
                color: [255, 0, 0, 255],
                radius: 2.0,
            },
            PaintStroke {
                points: vec![[5.0, 15.0], [15.0, 15.0]],
                color: [0, 0, 255, 255],
                radius: 3.0,
            },
        ];

        let actions: Vec<PosterAction> = strokes.iter().cloned().map(PosterAction::Stroke).collect();

        // Sequential painting
        let mut sequential = vec![255u8; (w * h * 4) as usize];
        for s in &strokes {
            paint_stroke(&mut sequential, w, h, s);
        }

        // Replay
        let replayed = replay_actions(&actions, w, h);
        assert_eq!(sequential, replayed);
    }

    #[test]
    fn flood_fill_basic() {
        let w = 8;
        let h = 8;
        let mut data = vec![255u8; (w * h * 4) as usize];
        flood_fill(&mut data, w, h, 0, 0, [255, 0, 0, 255]);
        // All pixels should be red
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                assert_eq!(data[i], 255);
                assert_eq!(data[i + 1], 0);
                assert_eq!(data[i + 2], 0);
            }
        }
    }

    #[test]
    fn flood_fill_respects_boundary() {
        let w = 8;
        let h = 8;
        let mut data = vec![255u8; (w * h * 4) as usize];
        // Draw a black horizontal line at y=4
        for x in 0..w {
            let i = ((4 * w + x) * 4) as usize;
            data[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
        // Fill top half with red
        flood_fill(&mut data, w, h, 0, 0, [255, 0, 0, 255]);
        // Pixel above the line should be red
        let above = ((3 * w + 0) * 4) as usize;
        assert_eq!(data[above], 255);
        assert_eq!(data[above + 1], 0);
        // Pixel below the line should still be white
        let below = ((5 * w + 0) * 4) as usize;
        assert_eq!(data[below], 255);
        assert_eq!(data[below + 1], 255);
    }
}
