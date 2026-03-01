use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize};

use super::slot_enums::{
    Accessory, EyeStyle, FaceShape, HairStyle, MouthStyle, ShirtStyle,
};

// ── Skin tone presets ───────────────────────────────────────────────────────

pub(crate) const SKIN_TONES: [[f32; 3]; 8] = [
    [0.96, 0.82, 0.71],
    [0.87, 0.72, 0.58],
    [0.78, 0.61, 0.47],
    [0.65, 0.50, 0.36],
    [0.55, 0.40, 0.28],
    [0.44, 0.30, 0.20],
    [0.35, 0.22, 0.14],
    [0.25, 0.16, 0.10],
];

// ── Secondary color enum ────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum SecondaryColor {
    /// Use the pilot's drone color (default).
    #[default]
    DroneColor,
    /// An explicit palette color chosen by the user.
    Chosen([f32; 3]),
}

// ── Portrait descriptor ─────────────────────────────────────────────────────

/// Helper struct for backward-compatible deserialization of `PortraitDescriptor`.
/// Old RON files have the bool+Option pattern; new ones use `SecondaryColor`.
#[derive(Deserialize)]
struct PortraitDescriptorRaw {
    #[serde(default)]
    face_shape: FaceShape,
    #[serde(default)]
    eyes: EyeStyle,
    #[serde(default)]
    mouth: MouthStyle,
    #[serde(default)]
    hair: HairStyle,
    #[serde(default)]
    shirt: ShirtStyle,
    #[serde(default)]
    accessory: Option<Accessory>,
    #[serde(default)]
    skin_tone: [f32; 3],
    #[serde(default)]
    hair_color: [f32; 3],
    #[serde(default)]
    eye_color: [f32; 3],
    #[serde(default)]
    accessory_color: [f32; 3],
    #[serde(default)]
    shirt_color: [f32; 3],
    // New fields
    #[serde(default)]
    skin_secondary: SecondaryColor,
    #[serde(default)]
    acc_secondary: SecondaryColor,
    /// Handles both old `Option<[f32;3]>` and new `SecondaryColor` formats.
    #[serde(default, deserialize_with = "deserialize_eye_secondary_compat")]
    eye_secondary: SecondaryColor,
    // Old fields (legacy format — used for migration when new fields are absent)
    #[serde(default)]
    skin_highlight: Option<[f32; 3]>,
    #[serde(default)]
    acc_shadow: Option<[f32; 3]>,
    #[serde(default)]
    generated: bool,
}

/// Deserialize `eye_secondary` from either old `Option<[f32;3]>` or new `SecondaryColor`.
fn deserialize_eye_secondary_compat<'de, D>(
    deserializer: D,
) -> Result<SecondaryColor, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;

    struct EyeSecVisitor;

    impl<'de> de::Visitor<'de> for EyeSecVisitor {
        type Value = SecondaryColor;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "SecondaryColor, Option<[f32; 3]>, or None")
        }

        fn visit_enum<A: de::EnumAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
            let (variant, data): (String, _) = access.variant()?;
            match variant.as_str() {
                "DroneColor" => {
                    de::VariantAccess::unit_variant(data)?;
                    Ok(SecondaryColor::DroneColor)
                }
                "Chosen" => {
                    let arr: [f32; 3] = de::VariantAccess::newtype_variant(data)?;
                    Ok(SecondaryColor::Chosen(arr))
                }
                "None" => {
                    de::VariantAccess::unit_variant(data)?;
                    Ok(SecondaryColor::DroneColor)
                }
                "Some" => {
                    // Old format: Some((x, y, z)) — inner value is [f32; 3]
                    let arr: [f32; 3] = de::VariantAccess::newtype_variant(data)?;
                    Ok(SecondaryColor::Chosen(arr))
                }
                other => Err(de::Error::unknown_variant(
                    other,
                    &["DroneColor", "Chosen", "None", "Some"],
                )),
            }
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(SecondaryColor::DroneColor)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(SecondaryColor::DroneColor)
        }

        fn visit_some<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
            SecondaryColor::deserialize(d)
        }
    }

    deserializer.deserialize_any(EyeSecVisitor)
}

fn migrate_secondary(
    new_field: SecondaryColor,
    old_color: Option<[f32; 3]>,
) -> SecondaryColor {
    // If the new field is the default (DroneColor) and an old explicit color
    // exists, prefer the legacy value.
    if matches!(new_field, SecondaryColor::DroneColor)
        && let Some(c) = old_color
    {
        return SecondaryColor::Chosen(c);
    }
    new_field
}

impl<'de> Deserialize<'de> for PortraitDescriptor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = PortraitDescriptorRaw::deserialize(deserializer)?;

        Ok(PortraitDescriptor {
            face_shape: raw.face_shape,
            eyes: raw.eyes,
            mouth: raw.mouth,
            hair: raw.hair,
            shirt: raw.shirt,
            accessory: raw.accessory,
            skin_tone: raw.skin_tone,
            hair_color: raw.hair_color,
            eye_color: raw.eye_color,
            accessory_color: raw.accessory_color,
            shirt_color: raw.shirt_color,
            skin_secondary: migrate_secondary(raw.skin_secondary, raw.skin_highlight),
            acc_secondary: migrate_secondary(raw.acc_secondary, raw.acc_shadow),
            eye_secondary: migrate_secondary(raw.eye_secondary, None),
            generated: raw.generated,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PortraitDescriptor {
    #[serde(default = "default_face_shape")]
    pub face_shape: FaceShape,
    #[serde(default = "default_eye_style")]
    pub eyes: EyeStyle,
    #[serde(default = "default_mouth_style")]
    pub mouth: MouthStyle,
    #[serde(default = "default_hair_style")]
    pub hair: HairStyle,
    #[serde(default = "default_shirt_style")]
    pub shirt: ShirtStyle,
    #[serde(default)]
    pub accessory: Option<Accessory>,
    #[serde(default)]
    pub skin_tone: [f32; 3],
    #[serde(default)]
    pub hair_color: [f32; 3],
    #[serde(default)]
    pub eye_color: [f32; 3],
    #[serde(default)]
    pub accessory_color: [f32; 3],
    #[serde(default)]
    pub shirt_color: [f32; 3],
    #[serde(default)]
    pub skin_secondary: SecondaryColor,
    #[serde(default)]
    pub acc_secondary: SecondaryColor,
    #[serde(default)]
    pub eye_secondary: SecondaryColor,
    #[serde(default)]
    pub generated: bool,
}

impl Default for PortraitDescriptor {
    fn default() -> Self {
        Self {
            face_shape: FaceShape::Oval,
            eyes: EyeStyle::Normal,
            mouth: MouthStyle::Neutral,
            hair: HairStyle::ShortCrop,
            shirt: ShirtStyle::Crew,
            accessory: None,
            skin_tone: [0.0; 3],
            hair_color: [0.0; 3],
            eye_color: [0.0; 3],
            accessory_color: [0.0; 3],
            shirt_color: [0.0; 3],
            skin_secondary: SecondaryColor::DroneColor,
            acc_secondary: SecondaryColor::DroneColor,
            eye_secondary: SecondaryColor::DroneColor,
            generated: false,
        }
    }
}

impl PortraitDescriptor {
    /// Generate a randomized portrait descriptor using default palette config.
    pub fn generate(rng: &mut impl Rng, primary_color: [f32; 3]) -> Self {
        use crate::dev_menu::portrait_config::PALETTE_COLORS;
        Self::generate_with_config(
            rng,
            primary_color,
            &PALETTE_COLORS,
            &crate::dev_menu::portrait_config::PortraitPaletteConfig::default(),
        )
    }

    /// Generate a randomized portrait using a palette configuration.
    ///
    /// Colors are picked from non-vetoed palette entries. Secondary colors
    /// use explicit complementary mappings when available.
    pub fn generate_with_config(
        rng: &mut impl Rng,
        primary_color: [f32; 3],
        palette_colors: &[(&str, [f32; 3])],
        config: &crate::dev_menu::portrait_config::PortraitPaletteConfig,
    ) -> Self {
        use crate::dev_menu::portrait_config::PortraitColorSlot;

        let face_shape = FaceShape::random(rng);
        let eyes = EyeStyle::random(rng);
        let mouth = MouthStyle::random(rng);
        let hair = HairStyle::random(rng);
        let shirt = ShirtStyle::random(rng);

        let accessory = if rng.gen_bool(0.5) {
            Some(Accessory::random(rng))
        } else {
            None
        };

        // Compute variant indices for variant-aware veto/complementary lookups
        let face_vi = Some(face_shape.index());
        let eye_vi = Some(eyes.index());
        let hair_vi = Some(hair.index());
        let shirt_vi = Some(shirt.index());
        let acc_vi = accessory.map(|a| a.index());

        let mut pick =
            |slot: PortraitColorSlot, vi: Option<usize>, fallback: [f32; 3]| -> (usize, [f32; 3]) {
                let allowed = config.allowed_indices_for(slot, vi);
                if allowed.is_empty() {
                    return (0, fallback);
                }
                let idx = allowed[rng.gen_range(0..allowed.len())];
                (idx, palette_colors[idx].1)
            };

        let (skin_idx, skin_tone) = pick(PortraitColorSlot::Skin, face_vi, SKIN_TONES[0]);
        let (_hair_idx, hair_color) = pick(PortraitColorSlot::Hair, hair_vi, [0.3, 0.2, 0.1]);
        let (_eye_idx, eye_color_fallback) =
            pick(PortraitColorSlot::Eye, eye_vi, [0.3, 0.5, 0.7]);
        let (_shirt_idx, shirt_color_from_palette) = pick(
            PortraitColorSlot::Shirt,
            shirt_vi,
            derive_shirt_color(primary_color),
        );
        let (acc_idx, acc_color_from_palette) = pick(
            PortraitColorSlot::Accessory,
            acc_vi,
            derive_accessory_color(primary_color),
        );

        // Secondary colors: check complementary map.
        // A sentinel value of DRONE_COLOR_INDEX means "use pilot drone color."
        use crate::dev_menu::portrait_config::DRONE_COLOR_INDEX;

        let resolve_comp = |comp: Option<usize>| -> SecondaryColor {
            match comp {
                Some(i) if i == DRONE_COLOR_INDEX => SecondaryColor::DroneColor,
                Some(i) => SecondaryColor::Chosen(palette_colors[i].1),
                None => SecondaryColor::DroneColor,
            }
        };

        let skin_comp = config.get_complementary_for(PortraitColorSlot::Skin, face_vi, skin_idx);
        let skin_secondary = resolve_comp(skin_comp);

        let eye_color = eye_color_fallback;
        let eye_comp = config.get_complementary_for(PortraitColorSlot::Eye, eye_vi, _eye_idx);
        let eye_secondary = resolve_comp(eye_comp);

        let acc_comp =
            config.get_complementary_for(PortraitColorSlot::Accessory, acc_vi, acc_idx);
        let acc_secondary = resolve_comp(acc_comp);

        Self {
            face_shape,
            eyes,
            mouth,
            hair,
            shirt,
            accessory,
            skin_tone,
            hair_color,
            eye_color,
            accessory_color: acc_color_from_palette,
            shirt_color: shirt_color_from_palette,
            skin_secondary,
            acc_secondary,
            eye_secondary,
            generated: true,
        }
    }

    /// Returns `true` if this descriptor is the empty placeholder (not yet generated).
    pub fn is_empty(&self) -> bool {
        !self.generated
    }
}

// ── Color helpers ───────────────────────────────────────────────────────────

/// Convert HSL to sRGB. All inputs/outputs in [0..1] range, except `h` which is [0..360).
fn hsl_to_srgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    [
        (r1 + m).clamp(0.0, 1.0),
        (g1 + m).clamp(0.0, 1.0),
        (b1 + m).clamp(0.0, 1.0),
    ]
}

/// Convert sRGB [0..1] to HSL. Returns (h [0..360), s [0..1], l [0..1]).
fn srgb_to_hsl(rgb: [f32; 3]) -> (f32, f32, f32) {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        let mut hue = (g - b) / d;
        if hue < 0.0 {
            hue += 6.0;
        }
        hue
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, l)
}

/// Derive accessory color from pilot primary: shift hue +30 deg, slightly desaturate.
fn derive_accessory_color(primary: [f32; 3]) -> [f32; 3] {
    let (h, s, l) = srgb_to_hsl(primary);
    let new_h = (h + 30.0) % 360.0;
    let new_s = (s * 0.8).clamp(0.0, 1.0);
    hsl_to_srgb(new_h, new_s, l)
}

/// Derive shirt color from pilot primary: heavily desaturated and lightened.
fn derive_shirt_color(primary: [f32; 3]) -> [f32; 3] {
    let (h, s, l) = srgb_to_hsl(primary);
    let new_s = (s * 0.3).clamp(0.0, 1.0);
    let new_l = l.max(0.75);
    hsl_to_srgb(h, new_s, new_l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let d = PortraitDescriptor::default();
        assert!(d.is_empty());
        assert!(!d.generated);
    }

    #[test]
    fn generated_is_not_empty() {
        let mut rng = rand::thread_rng();
        let d = PortraitDescriptor::generate(&mut rng, [0.8, 0.2, 0.4]);
        assert!(!d.is_empty());
        assert!(d.generated);
    }

    #[test]
    fn generated_colors_in_range() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let primary = [
                rng.gen_range(0.0..=1.0),
                rng.gen_range(0.0..=1.0),
                rng.gen_range(0.0..=1.0),
            ];
            let d = PortraitDescriptor::generate(&mut rng, primary);
            for ch in d.skin_tone {
                assert!((0.0..=1.0).contains(&ch), "skin_tone out of range: {ch}");
            }
            for ch in d.hair_color {
                assert!((0.0..=1.0).contains(&ch), "hair_color out of range: {ch}");
            }
            for ch in d.eye_color {
                assert!((0.0..=1.0).contains(&ch), "eye_color out of range: {ch}");
            }
            for ch in d.accessory_color {
                assert!(
                    (0.0..=1.0).contains(&ch),
                    "accessory_color out of range: {ch}"
                );
            }
            for ch in d.shirt_color {
                assert!((0.0..=1.0).contains(&ch), "shirt_color out of range: {ch}");
            }
        }
    }

    #[test]
    fn hsl_roundtrip_primary_colors() {
        // Red
        let rgb = hsl_to_srgb(0.0, 1.0, 0.5);
        assert!((rgb[0] - 1.0).abs() < 0.001);
        assert!(rgb[1].abs() < 0.001);
        assert!(rgb[2].abs() < 0.001);

        // Green
        let rgb = hsl_to_srgb(120.0, 1.0, 0.5);
        assert!(rgb[0].abs() < 0.001);
        assert!((rgb[1] - 1.0).abs() < 0.001);
        assert!(rgb[2].abs() < 0.001);

        // Blue
        let rgb = hsl_to_srgb(240.0, 1.0, 0.5);
        assert!(rgb[0].abs() < 0.001);
        assert!(rgb[1].abs() < 0.001);
        assert!((rgb[2] - 1.0).abs() < 0.001);
    }

    #[test]
    fn hsl_achromatic() {
        // White
        let rgb = hsl_to_srgb(0.0, 0.0, 1.0);
        assert!((rgb[0] - 1.0).abs() < 0.001);
        assert!((rgb[1] - 1.0).abs() < 0.001);
        assert!((rgb[2] - 1.0).abs() < 0.001);

        // Black
        let rgb = hsl_to_srgb(0.0, 0.0, 0.0);
        assert!(rgb[0].abs() < 0.001);
        assert!(rgb[1].abs() < 0.001);
        assert!(rgb[2].abs() < 0.001);

        // Mid-gray
        let rgb = hsl_to_srgb(0.0, 0.0, 0.5);
        assert!((rgb[0] - 0.5).abs() < 0.001);
        assert!((rgb[1] - 0.5).abs() < 0.001);
        assert!((rgb[2] - 0.5).abs() < 0.001);
    }

    #[test]
    fn srgb_to_hsl_roundtrip() {
        let test_colors: &[[f32; 3]] = &[
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.5, 0.5, 0.5],
            [0.8, 0.2, 0.6],
            [0.3, 0.7, 0.4],
        ];
        for &color in test_colors {
            let (h, s, l) = srgb_to_hsl(color);
            let back = hsl_to_srgb(h, s, l);
            for i in 0..3 {
                assert!(
                    (color[i] - back[i]).abs() < 0.01,
                    "Roundtrip failed for {color:?}: got {back:?} (hsl={h},{s},{l})"
                );
            }
        }
    }

    #[test]
    fn accessory_color_shifts_hue() {
        let primary = [1.0, 0.0, 0.0]; // pure red, hue=0
        let acc = derive_accessory_color(primary);
        let (h, _s, _l) = srgb_to_hsl(acc);
        // Should be around hue 30 (orange)
        assert!(
            (h - 30.0).abs() < 1.0,
            "Expected hue ~30, got {h}"
        );
    }

    #[test]
    fn accessory_color_desaturates() {
        let primary = hsl_to_srgb(180.0, 1.0, 0.5);
        let acc = derive_accessory_color(primary);
        let (_h, s_acc, _l) = srgb_to_hsl(acc);
        let (_, s_orig, _) = srgb_to_hsl(primary);
        assert!(
            s_acc < s_orig,
            "Accessory should be less saturated: {s_acc} vs {s_orig}"
        );
    }

    #[test]
    fn serde_backward_compat_empty_struct() {
        // Phase 1 rosters serialize PortraitDescriptor as `()` — verify we can
        // deserialize from the unit variant.
        let ron_str = "()";
        let result: Result<PortraitDescriptor, _> = ron::from_str(ron_str);
        assert!(result.is_ok(), "Should deserialize from unit: {result:?}");
        let d = result.unwrap();
        assert!(d.is_empty());
    }

    #[test]
    fn serde_full_roundtrip() {
        let mut rng = rand::thread_rng();
        let d = PortraitDescriptor::generate(&mut rng, [0.5, 0.3, 0.8]);
        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&d, pretty).unwrap();
        let deserialized: PortraitDescriptor = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.face_shape, d.face_shape);
        assert_eq!(deserialized.eyes, d.eyes);
        assert_eq!(deserialized.mouth, d.mouth);
        assert_eq!(deserialized.hair, d.hair);
        assert_eq!(deserialized.shirt, d.shirt);
        assert_eq!(deserialized.accessory, d.accessory);
        assert!(deserialized.generated);
    }

    #[test]
    fn shirt_color_is_light_and_desaturated() {
        let primary = [1.0, 0.0, 0.0]; // pure red
        let shirt = derive_shirt_color(primary);
        let (_h, s, l) = srgb_to_hsl(shirt);
        assert!(l >= 0.75, "Shirt lightness should be >= 0.75, got {l}");
        assert!(s < 0.5, "Shirt should be desaturated, got {s}");
    }

    #[test]
    fn accessory_roughly_fifty_percent() {
        let mut rng = rand::thread_rng();
        let mut with_acc = 0;
        let total = 10_000;
        for _ in 0..total {
            let d = PortraitDescriptor::generate(&mut rng, [0.5, 0.5, 0.5]);
            if d.accessory.is_some() {
                with_acc += 1;
            }
        }
        let ratio = with_acc as f64 / total as f64;
        assert!(
            (0.4..=0.6).contains(&ratio),
            "Accessory rate {ratio} is outside 40-60% expected range"
        );
    }
}
