use bevy::log::warn;

use super::{EyeStyle, PortraitDescriptor};
use super::loader::PortraitParts;

// ---------------------------------------------------------------------------
// Fixed color constants (hex values for SVG output)
// ---------------------------------------------------------------------------

/// VANILLA from the palette: srgb(0.949, 0.949, 0.855) = #f2f2da
const VANILLA_HEX: &str = "#f2f2da";
const BLACK_HEX: &str = "#000000";

// ---------------------------------------------------------------------------
// SVG source colors (convention used in the master SVG)
// ---------------------------------------------------------------------------

/// Primary color placeholder in the master SVG.
const SRC_BLACK: &str = "#000000";
/// Secondary color placeholder in the master SVG.
const SRC_WHITE: &str = "#ffffff";
/// Eye whites / fixed VANILLA color in the master SVG.
const SRC_GRAY_50: &str = "#808080";
/// Pupil / fixed BLACK color in the master SVG.
const SRC_GRAY_80: &str = "#333333";
/// Mouth smile fill (teeth) in the master SVG.
const SRC_TEETH: &str = "#e5e5e5";

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Brighten a color to ~130% lightness for highlight tones, clamped to 1.0.
pub fn compute_highlight(base: [f32; 3]) -> [f32; 3] {
    [
        (base[0] * 1.3).min(1.0),
        (base[1] * 1.3).min(1.0),
        (base[2] * 1.3).min(1.0),
    ]
}

/// Darken a color to ~70% lightness for shadow tones.
pub fn compute_shadow(base: [f32; 3]) -> [f32; 3] {
    [base[0] * 0.7, base[1] * 0.7, base[2] * 0.7]
}

pub fn color_to_hex(rgb: [f32; 3]) -> String {
    let r = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

// ---------------------------------------------------------------------------
// Per-layer color replacement
// ---------------------------------------------------------------------------

/// Determines which color replacements to apply for a given layer.
#[derive(Clone, Copy, Debug, PartialEq)]
enum LayerType {
    Background,
    Face,
    Hair,
    Eyes,
    EyesVisor,
    Mouth,
    Shirt,
    Accessory,
}

/// Precomputed hex color strings for portrait assembly.
struct PortraitColors {
    bg_hex: String,
    skin_hex: String,
    skin_highlight_hex: String,
    hair_hex: String,
    eye_hex: String,
    eye_shadow_hex: String,
    shirt_hex: String,
    acc_hex: String,
    acc_shadow_hex: String,
}

impl PortraitColors {
    fn from_descriptor(desc: &PortraitDescriptor, bg_color: [f32; 3]) -> Self {
        Self {
            bg_hex: color_to_hex(bg_color),
            skin_hex: color_to_hex(desc.skin_tone),
            skin_highlight_hex: color_to_hex(compute_highlight(desc.skin_tone)),
            hair_hex: color_to_hex(desc.hair_color),
            eye_hex: color_to_hex(desc.eye_color),
            eye_shadow_hex: color_to_hex(compute_shadow(desc.eye_color)),
            shirt_hex: color_to_hex(desc.shirt_color),
            acc_hex: color_to_hex(desc.accessory_color),
            acc_shadow_hex: color_to_hex(compute_shadow(desc.accessory_color)),
        }
    }
}

/// Apply color replacements to a fragment based on its layer type.
///
/// All replacements are applied in a single pass to avoid chained substitutions
/// (e.g., #000000 → #808080 then #808080 → VANILLA).
fn replace_layer_colors(content: &str, layer_type: LayerType, colors: &PortraitColors) -> String {
    // Build the replacement table: (source_hex, target_hex).
    // Order doesn't matter — they're applied simultaneously.
    let mut replacements: Vec<(&str, &str)> = Vec::with_capacity(5);

    match layer_type {
        LayerType::Background => {
            replacements.push((SRC_GRAY_50, &colors.bg_hex));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::Face => {
            replacements.push((SRC_WHITE, &colors.skin_highlight_hex));
            replacements.push((SRC_BLACK, &colors.skin_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::Hair => {
            replacements.push((SRC_BLACK, &colors.hair_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::Eyes => {
            replacements.push((SRC_WHITE, &colors.eye_hex));
            replacements.push((SRC_BLACK, &colors.hair_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::EyesVisor => {
            // Visor: WHITE = eye_color, BLACK = eye shadow (darker tint)
            replacements.push((SRC_WHITE, &colors.eye_hex));
            replacements.push((SRC_BLACK, &colors.eye_shadow_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::Mouth => {
            replacements.push((SRC_TEETH, VANILLA_HEX));
            replacements.push((SRC_BLACK, &colors.skin_highlight_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::Shirt => {
            replacements.push((SRC_BLACK, &colors.shirt_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
        LayerType::Accessory => {
            replacements.push((SRC_WHITE, &colors.acc_shadow_hex));
            replacements.push((SRC_BLACK, &colors.acc_hex));
            replacements.push((SRC_GRAY_50, VANILLA_HEX));
            replacements.push((SRC_GRAY_80, BLACK_HEX));
        }
    }

    replace_all_simultaneous(content, &replacements)
}

/// Replace multiple patterns simultaneously so no replacement's output can
/// be consumed by another replacement's source pattern.
fn replace_all_simultaneous(input: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        let mut matched = false;
        for &(src, dst) in replacements {
            if input[i..].starts_with(src) {
                result.push_str(dst);
                i += src.len();
                matched = true;
                break;
            }
        }
        if !matched {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// SVG assembly from loaded parts
// ---------------------------------------------------------------------------

/// Fetch a fragment from `PortraitParts`, logging a warning if missing.
fn get_or_warn<'a>(parts: &'a PortraitParts, slot: &str, variant: &str) -> &'a str {
    parts.get(slot, variant).unwrap_or_else(|| {
        warn!("Missing portrait SVG part: {slot}_{variant}");
        ""
    })
}

/// Assemble a complete portrait SVG from a descriptor, background color, and loaded parts.
///
/// Layer order: background → hair back → face → shirt → eyes →
///              mouth → hair front → accessory.
pub fn assemble_svg(
    descriptor: &PortraitDescriptor,
    bg_color: [f32; 3],
    parts: &PortraitParts,
) -> String {
    let colors = PortraitColors::from_descriptor(descriptor, bg_color);
    let hair_id = descriptor.hair.group_id();

    let mut svg = String::with_capacity(16384);
    svg.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" viewBox="0 0 20 20">"#);

    // Background
    if let Some(bg) = parts.get_by_label("background") {
        svg.push_str(&replace_layer_colors(bg, LayerType::Background, &colors));
    }

    // Hair back
    if let Some(hb) = parts.get("hair_back", hair_id) {
        svg.push_str(&replace_layer_colors(hb, LayerType::Hair, &colors));
    }

    // Face
    let face = get_or_warn(parts, "face", descriptor.face_shape.group_id());
    if !face.is_empty() {
        svg.push_str(&replace_layer_colors(face, LayerType::Face, &colors));
    }

    // Shirt
    let shirt = get_or_warn(parts, "shirt", descriptor.shirt.group_id());
    if !shirt.is_empty() {
        svg.push_str(&replace_layer_colors(shirt, LayerType::Shirt, &colors));
    }

    // Eyes
    let eyes = get_or_warn(parts, "eyes", descriptor.eyes.group_id());
    if !eyes.is_empty() {
        let eye_layer = if descriptor.eyes == EyeStyle::Visor {
            LayerType::EyesVisor
        } else {
            LayerType::Eyes
        };
        svg.push_str(&replace_layer_colors(eyes, eye_layer, &colors));
    }

    // Mouth
    let mouth = get_or_warn(parts, "mouth", descriptor.mouth.group_id());
    if !mouth.is_empty() {
        svg.push_str(&replace_layer_colors(mouth, LayerType::Mouth, &colors));
    }

    // Hair front
    let hair_front = get_or_warn(parts, "hair_front", hair_id);
    if !hair_front.is_empty() {
        svg.push_str(&replace_layer_colors(hair_front, LayerType::Hair, &colors));
    }

    // Accessory (optional)
    if let Some(acc) = &descriptor.accessory {
        let acc_content = get_or_warn(parts, "acc", acc.group_id());
        if !acc_content.is_empty() {
            svg.push_str(&replace_layer_colors(
                acc_content,
                LayerType::Accessory,
                &colors,
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn color_to_hex_black() {
        assert_eq!(color_to_hex([0.0, 0.0, 0.0]), "#000000");
    }

    #[test]
    fn color_to_hex_white() {
        assert_eq!(color_to_hex([1.0, 1.0, 1.0]), "#ffffff");
    }

    #[test]
    fn color_to_hex_red() {
        assert_eq!(color_to_hex([1.0, 0.0, 0.0]), "#ff0000");
    }

    #[test]
    fn color_to_hex_clamped() {
        assert_eq!(color_to_hex([1.5, -0.5, 0.5]), "#ff0080");
    }

    #[test]
    fn compute_shadow_darkens() {
        let base = [1.0, 0.8, 0.6];
        let shadow = compute_shadow(base);
        assert!((shadow[0] - 0.7).abs() < 1e-6);
        assert!((shadow[1] - 0.56).abs() < 1e-6);
        assert!((shadow[2] - 0.42).abs() < 1e-6);
    }

    #[test]
    fn compute_highlight_brightens_and_clamps() {
        let base = [0.5, 0.9, 1.0];
        let highlight = compute_highlight(base);
        assert!((highlight[0] - 0.65).abs() < 1e-6);
        assert!((highlight[1] - 1.0).abs() < 1e-6);
        assert!((highlight[2] - 1.0).abs() < 1e-6);
    }

    /// Build a `PortraitParts` with hex-colored placeholder content for all slots.
    fn test_parts() -> PortraitParts {
        let mut parts = PortraitParts::default();

        let bg = r##"<rect width="20" height="20" rx="1.5" ry="1.5" fill="#808080"/>"##;
        parts.insert("background", bg);

        for v in &["oval", "round", "square", "angular"] {
            parts.insert(
                format!("face_{v}"),
                format!(r##"<path id="face-{v}" fill="#000000"/><path id="chin-{v}" stroke="#ffffff"/>"##),
            );
        }

        for v in &["normal", "narrow", "wide", "visor"] {
            parts.insert(
                format!("eyes_{v}"),
                format!(
                    r##"<path id="white-{v}" fill="#808080"/><path id="iris-{v}" fill="#ffffff"/><circle id="pupil-{v}" fill="#333333"/><path id="brow-{v}" stroke="#000000"/>"##
                ),
            );
        }

        for v in &["neutral", "smile", "smirk", "frown"] {
            if *v == "smile" {
                parts.insert(
                    format!("mouth_{v}"),
                    format!(r##"<path id="mouth-{v}" fill="#e5e5e5" stroke="#000000"/>"##),
                );
            } else {
                parts.insert(
                    format!("mouth_{v}"),
                    format!(r##"<path id="mouth-{v}" stroke="#000000"/>"##),
                );
            }
        }

        for v in &["short_crop", "long_sweep", "beanie"] {
            parts.insert(
                format!("hair_back_{v}"),
                format!(r##"<path id="hair-back-{v}" fill="#000000"/>"##),
            );
        }

        for v in &["short_crop", "mohawk", "long_sweep", "beanie"] {
            parts.insert(
                format!("hair_front_{v}"),
                format!(r##"<path id="hair-front-{v}" fill="#000000"/>"##),
            );
        }

        for v in &["crew", "round", "turtleneck", "vneck"] {
            parts.insert(
                format!("shirt_{v}"),
                format!(r##"<path id="shirt-{v}" fill="#000000"/>"##),
            );
        }

        for v in &["earring_round", "earring_ring", "necklace_chain", "necklace_pendant"] {
            parts.insert(
                format!("acc_{v}"),
                format!(r##"<circle id="acc-{v}" fill="#000000" stroke="#ffffff"/>"##),
            );
        }

        parts
    }

    fn test_desc() -> PortraitDescriptor {
        PortraitDescriptor {
            face_shape: FaceShape::Oval,
            eyes: EyeStyle::Normal,
            mouth: MouthStyle::Neutral,
            hair: HairStyle::ShortCrop,
            shirt: ShirtStyle::Crew,
            accessory: None,
            skin_tone: [0.9, 0.7, 0.55],
            hair_color: [0.2, 0.15, 0.1],
            eye_color: [0.2, 0.5, 0.8],
            accessory_color: [0.5, 0.5, 0.5],
            shirt_color: [0.8, 0.8, 0.85],
            generated: true,
        }
    }

    #[test]
    fn assemble_svg_viewbox_20x20() {
        let parts = test_parts();
        let desc = test_desc();
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.2], &parts);
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" viewBox="0 0 20 20">"#));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn assemble_svg_contains_layer_content() {
        let parts = test_parts();
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Angular,
            eyes: EyeStyle::Wide,
            mouth: MouthStyle::Smile,
            hair: HairStyle::ShortCrop,
            shirt: ShirtStyle::Round,
            accessory: Some(Accessory::NecklacePendant),
            skin_tone: [0.8, 0.6, 0.4],
            hair_color: [0.1, 0.1, 0.1],
            eye_color: [0.3, 0.3, 0.7],
            accessory_color: [0.7, 0.2, 0.2],
            shirt_color: [0.8, 0.8, 0.8],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.0, 0.0, 0.0], &parts);
        assert!(svg.contains("face-angular"), "missing face content");
        assert!(svg.contains("iris-wide"), "missing eyes content");
        assert!(svg.contains("mouth-smile"), "missing mouth content");
        assert!(svg.contains("shirt-round"), "missing shirt content");
        assert!(svg.contains("acc-necklace_pendant"), "missing accessory content");
    }

    #[test]
    fn replace_face_colors() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.1, 0.1, 0.2]);
        let content = r##"<path fill="#000000"/><path stroke="#ffffff"/>"##;
        let result = replace_layer_colors(content, LayerType::Face, &colors);
        assert!(result.contains(&colors.skin_hex), "Face fill should be skin_tone");
        assert!(
            result.contains(&colors.skin_highlight_hex),
            "Face detail stroke should be skin_highlight"
        );
        assert!(!result.contains(SRC_BLACK));
        assert!(!result.contains(SRC_WHITE));
    }

    #[test]
    fn replace_hair_colors() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.1, 0.1, 0.2]);
        let content = r##"<path fill="#000000"/>"##;
        let result = replace_layer_colors(content, LayerType::Hair, &colors);
        assert!(result.contains(&colors.hair_hex), "Hair fill should be hair_color");
        assert!(!result.contains(SRC_BLACK));
    }

    #[test]
    fn replace_eye_colors() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.1, 0.1, 0.2]);
        let content = r##"<path fill="#808080"/><path fill="#ffffff"/><circle fill="#333333"/><path stroke="#000000"/>"##;
        let result = replace_layer_colors(content, LayerType::Eyes, &colors);
        assert!(result.contains(VANILLA_HEX), "Eye whites should be VANILLA");
        assert!(result.contains(&colors.eye_hex), "Iris should be eye_color");
        assert!(result.contains(&colors.hair_hex), "Eyebrow should be hair_color");
    }

    #[test]
    fn replace_mouth_colors() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.1, 0.1, 0.2]);
        let content = r##"<path fill="#e5e5e5" stroke="#000000"/>"##;
        let result = replace_layer_colors(content, LayerType::Mouth, &colors);
        assert!(result.contains(VANILLA_HEX), "Teeth fill should be VANILLA");
        assert!(
            result.contains(&colors.skin_highlight_hex),
            "Mouth stroke should be skin_highlight"
        );
    }

    #[test]
    fn replace_shirt_colors() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.1, 0.1, 0.2]);
        let content = r##"<path fill="#000000"/>"##;
        let result = replace_layer_colors(content, LayerType::Shirt, &colors);
        assert!(result.contains(&colors.shirt_hex), "Shirt fill should be shirt_color");
    }

    #[test]
    fn replace_accessory_colors() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.1, 0.1, 0.2]);
        let content = r##"<circle fill="#000000" stroke="#ffffff"/>"##;
        let result = replace_layer_colors(content, LayerType::Accessory, &colors);
        assert!(result.contains(&colors.acc_hex), "Acc fill should be acc_color");
        assert!(result.contains(&colors.acc_shadow_hex), "Acc stroke should be acc_shadow");
    }

    #[test]
    fn replace_background_uses_bg_color_not_vanilla() {
        let colors = PortraitColors::from_descriptor(&test_desc(), [0.0, 1.0, 0.0]);
        let content = r##"<rect fill="#808080"/>"##;
        let result = replace_layer_colors(content, LayerType::Background, &colors);
        assert!(
            result.contains(&colors.bg_hex),
            "Background should become bg_color, not VANILLA"
        );
        assert!(!result.contains(VANILLA_HEX), "Background should NOT use VANILLA");
    }

    #[test]
    fn assemble_svg_no_accessory_produces_no_acc_content() {
        let parts = test_parts();
        let desc = test_desc();
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.2], &parts);
        assert!(!svg.contains("acc-"), "No accessory content should be present");
    }

    #[test]
    fn layer_order_correct() {
        let parts = test_parts();
        let desc = PortraitDescriptor {
            face_shape: FaceShape::Oval,
            eyes: EyeStyle::Normal,
            mouth: MouthStyle::Smile,
            hair: HairStyle::LongSwept,
            shirt: ShirtStyle::Crew,
            accessory: Some(Accessory::NecklacePendant),
            skin_tone: [0.8, 0.6, 0.45],
            hair_color: [0.3, 0.15, 0.05],
            eye_color: [0.2, 0.6, 0.3],
            accessory_color: [0.7, 0.2, 0.2],
            shirt_color: [0.8, 0.8, 0.85],
            generated: true,
        };
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.15], &parts);

        let bg_pos = svg.find("rx=\"1.5\"").unwrap();
        let hair_back_pos = svg.find("hair-back-long_sweep").unwrap();
        let face_pos = svg.find("face-oval").unwrap();
        let shirt_pos = svg.find("shirt-crew").unwrap();
        let eyes_pos = svg.find("iris-normal").unwrap();
        let mouth_pos = svg.find("mouth-smile").unwrap();
        let hair_front_pos = svg.find("hair-front-long_sweep").unwrap();
        let acc_pos = svg.find("acc-necklace_pendant").unwrap();

        assert!(bg_pos < hair_back_pos, "bg before hair-back");
        assert!(hair_back_pos < face_pos, "hair-back before face");
        assert!(face_pos < shirt_pos, "face before shirt");
        assert!(shirt_pos < eyes_pos, "shirt before eyes");
        assert!(eyes_pos < mouth_pos, "eyes before mouth");
        assert!(mouth_pos < hair_front_pos, "mouth before hair-front");
        assert!(hair_front_pos < acc_pos, "hair-front before accessory");
    }

    #[test]
    fn assemble_svg_hair_back_layer_present() {
        let parts = test_parts();
        let desc = PortraitDescriptor {
            hair: HairStyle::LongSwept,
            ..test_desc()
        };
        let svg = assemble_svg(&desc, [0.1, 0.1, 0.2], &parts);
        assert!(svg.contains("hair-back-long_sweep"));
    }
}
