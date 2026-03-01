pub mod cache;
pub mod fragments;
pub mod loader;
pub mod rasterize;

use rand::Rng;
use serde::{Deserialize, Serialize};

// ── Slot enums ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FaceShape {
    Oval,
    Round,
    Square,
    Angular,
    Long,
    Diamond,
}

pub const ALL_FACE_SHAPES: [FaceShape; 6] = [
    FaceShape::Oval,
    FaceShape::Round,
    FaceShape::Square,
    FaceShape::Angular,
    FaceShape::Long,
    FaceShape::Diamond,
];

impl FaceShape {
    fn random(rng: &mut impl Rng) -> Self {
        ALL_FACE_SHAPES[rng.gen_range(0..ALL_FACE_SHAPES.len())]
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            FaceShape::Oval | FaceShape::Long => "oval",
            FaceShape::Round => "round",
            FaceShape::Square => "square",
            FaceShape::Angular | FaceShape::Diamond => "angular",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EyeStyle {
    Normal,
    Narrow,
    Wide,
    Visor,
    Goggles,
    Winking,
}

pub const ALL_EYE_STYLES: [EyeStyle; 6] = [
    EyeStyle::Normal,
    EyeStyle::Narrow,
    EyeStyle::Wide,
    EyeStyle::Visor,
    EyeStyle::Goggles,
    EyeStyle::Winking,
];

impl EyeStyle {
    fn random(rng: &mut impl Rng) -> Self {
        ALL_EYE_STYLES[rng.gen_range(0..ALL_EYE_STYLES.len())]
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            EyeStyle::Normal | EyeStyle::Winking => "normal",
            EyeStyle::Narrow => "narrow",
            EyeStyle::Wide | EyeStyle::Goggles => "wide",
            EyeStyle::Visor => "visor",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MouthStyle {
    Neutral,
    Smile,
    Smirk,
    Gritted,
    Frown,
}

pub const ALL_MOUTH_STYLES: [MouthStyle; 5] = [
    MouthStyle::Neutral,
    MouthStyle::Smile,
    MouthStyle::Smirk,
    MouthStyle::Gritted,
    MouthStyle::Frown,
];

impl MouthStyle {
    fn random(rng: &mut impl Rng) -> Self {
        ALL_MOUTH_STYLES[rng.gen_range(0..ALL_MOUTH_STYLES.len())]
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            MouthStyle::Neutral => "neutral",
            MouthStyle::Smile => "smile",
            MouthStyle::Smirk => "smirk",
            MouthStyle::Gritted | MouthStyle::Frown => "frown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum HairStyle {
    ShortCrop,
    Mohawk,
    LongSwept,
    Helmet,
    Beanie,
    Bald,
    Ponytail,
}

pub const ALL_HAIR_STYLES: [HairStyle; 7] = [
    HairStyle::ShortCrop,
    HairStyle::Mohawk,
    HairStyle::LongSwept,
    HairStyle::Helmet,
    HairStyle::Beanie,
    HairStyle::Bald,
    HairStyle::Ponytail,
];

impl HairStyle {
    fn random(rng: &mut impl Rng) -> Self {
        ALL_HAIR_STYLES[rng.gen_range(0..ALL_HAIR_STYLES.len())]
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            HairStyle::ShortCrop | HairStyle::Bald => "short_crop",
            HairStyle::Mohawk => "mohawk",
            HairStyle::LongSwept | HairStyle::Ponytail => "long_sweep",
            HairStyle::Helmet | HairStyle::Beanie => "beanie",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Accessory {
    #[serde(alias = "Earring", alias = "GogglesUp", alias = "Antenna")]
    EarringRound,
    #[serde(alias = "SpikedCollar", alias = "FacePaint")]
    EarringRing,
    #[serde(alias = "Piercings", alias = "Earpiece")]
    NecklaceChain,
    #[serde(alias = "Necklace", alias = "Scar")]
    NecklacePendant,
}

pub const ALL_ACCESSORIES: [Accessory; 4] = [
    Accessory::EarringRound,
    Accessory::EarringRing,
    Accessory::NecklaceChain,
    Accessory::NecklacePendant,
];

impl Accessory {
    fn random(rng: &mut impl Rng) -> Self {
        ALL_ACCESSORIES[rng.gen_range(0..ALL_ACCESSORIES.len())]
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            Accessory::EarringRound => "earring_round",
            Accessory::EarringRing => "earring_ring",
            Accessory::NecklaceChain => "necklace_chain",
            Accessory::NecklacePendant => "necklace_pendant",
        }
    }

    /// Whether this accessory uses a secondary (shadow) color in its SVG.
    pub fn has_shadow(&self) -> bool {
        matches!(self, Accessory::NecklacePendant)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShirtStyle {
    Crew,
    Round,
    Turtleneck,
    Vneck,
}

pub const ALL_SHIRT_STYLES: [ShirtStyle; 4] = [
    ShirtStyle::Crew,
    ShirtStyle::Round,
    ShirtStyle::Turtleneck,
    ShirtStyle::Vneck,
];

impl ShirtStyle {
    fn random(rng: &mut impl Rng) -> Self {
        ALL_SHIRT_STYLES[rng.gen_range(0..ALL_SHIRT_STYLES.len())]
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            ShirtStyle::Crew => "crew",
            ShirtStyle::Round => "round",
            ShirtStyle::Turtleneck => "turtleneck",
            ShirtStyle::Vneck => "vneck",
        }
    }
}

// ── Skin tone presets ───────────────────────────────────────────────────────

const SKIN_TONES: [[f32; 3]; 8] = [
    [0.96, 0.82, 0.71],
    [0.87, 0.72, 0.58],
    [0.78, 0.61, 0.47],
    [0.65, 0.50, 0.36],
    [0.55, 0.40, 0.28],
    [0.44, 0.30, 0.20],
    [0.35, 0.22, 0.14],
    [0.25, 0.16, 0.10],
];

// ── Portrait descriptor ─────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub skin_highlight: Option<[f32; 3]>,
    #[serde(default)]
    pub acc_shadow: Option<[f32; 3]>,
    /// Visor frame color (secondary). When `None`, auto-derived as `compute_shadow(eye_color)`.
    #[serde(default)]
    pub eye_secondary: Option<[f32; 3]>,
    /// When true, skin_highlight resolves to the pilot's drone color at render time.
    #[serde(default)]
    pub skin_highlight_drone: bool,
    /// When true, acc_shadow resolves to the pilot's drone color at render time.
    #[serde(default)]
    pub acc_shadow_drone: bool,
    /// When true, eye_secondary resolves to the pilot's drone color at render time.
    #[serde(default)]
    pub eye_secondary_drone: bool,
    #[serde(default)]
    pub generated: bool,
}

fn default_face_shape() -> FaceShape {
    FaceShape::Oval
}

fn default_eye_style() -> EyeStyle {
    EyeStyle::Normal
}

fn default_mouth_style() -> MouthStyle {
    MouthStyle::Neutral
}

fn default_hair_style() -> HairStyle {
    HairStyle::ShortCrop
}

fn default_shirt_style() -> ShirtStyle {
    ShirtStyle::Crew
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
            skin_highlight: None,
            acc_shadow: None,
            eye_secondary: None,
            skin_highlight_drone: false,
            acc_shadow_drone: false,
            eye_secondary_drone: false,
            generated: false,
        }
    }
}

impl PortraitDescriptor {
    /// Generate a randomized portrait descriptor.
    ///
    /// `primary_color` is the pilot's drone color in sRGB [0..1],
    /// used to derive the accessory color.
    pub fn generate(rng: &mut impl Rng, primary_color: [f32; 3]) -> Self {
        let face_shape = FaceShape::random(rng);
        let eyes = EyeStyle::random(rng);
        let mouth = MouthStyle::random(rng);
        let hair = HairStyle::random(rng);
        let shirt = ShirtStyle::random(rng);

        // 50% chance of an accessory
        let accessory = if rng.gen_bool(0.5) {
            Some(Accessory::random(rng))
        } else {
            None
        };

        // Skin tone: pick a preset and add per-channel jitter
        let base_skin = SKIN_TONES[rng.gen_range(0..SKIN_TONES.len())];
        let skin_tone = [
            (base_skin[0] + rng.gen_range(-0.05..=0.05)).clamp(0.0, 1.0),
            (base_skin[1] + rng.gen_range(-0.05..=0.05)).clamp(0.0, 1.0),
            (base_skin[2] + rng.gen_range(-0.05..=0.05)).clamp(0.0, 1.0),
        ];

        // Hair color: random HSL
        let hair_color = hsl_to_srgb(
            rng.gen_range(0.0..360.0),
            rng.gen_range(0.3..0.9),
            rng.gen_range(0.2..0.8),
        );

        // Eye color: random HSL
        let eye_color = hsl_to_srgb(
            rng.gen_range(0.0..360.0),
            rng.gen_range(0.4..0.8),
            rng.gen_range(0.3..0.6),
        );

        // Accessory color: shift primary hue by +30 deg, slightly desaturate
        let accessory_color = derive_accessory_color(primary_color);

        // Shirt color: lighter, desaturated version of pilot primary
        let shirt_color = derive_shirt_color(primary_color);

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
            accessory_color,
            shirt_color,
            skin_highlight: None,
            acc_shadow: None,
            eye_secondary: None,
            skin_highlight_drone: false,
            acc_shadow_drone: false,
            eye_secondary_drone: false,
            generated: true,
        }
    }

    /// Generate a randomized portrait using a palette configuration.
    ///
    /// Colors are picked from non-vetoed palette entries. Secondary colors
    /// use explicit complementary mappings when available, falling back to
    /// algorithmic derivation.
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
        let face_vi = Some(ALL_FACE_SHAPES.iter().position(|s| *s == face_shape).unwrap_or(0));
        let eye_vi = Some(ALL_EYE_STYLES.iter().position(|s| *s == eyes).unwrap_or(0));
        let hair_vi = Some(ALL_HAIR_STYLES.iter().position(|s| *s == hair).unwrap_or(0));
        let shirt_vi = Some(ALL_SHIRT_STYLES.iter().position(|s| *s == shirt).unwrap_or(0));
        let acc_vi = accessory.and_then(|a| ALL_ACCESSORIES.iter().position(|x| *x == a));

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

        // Secondary colors: check complementary map, else fall back to auto-derived.
        // A sentinel value of DRONE_COLOR_INDEX means "use pilot drone color."
        use crate::dev_menu::portrait_config::DRONE_COLOR_INDEX;

        let skin_comp = config.get_complementary_for(PortraitColorSlot::Skin, face_vi, skin_idx);
        let skin_highlight_drone = skin_comp == Some(DRONE_COLOR_INDEX);
        let skin_highlight = skin_comp
            .filter(|&i| i != DRONE_COLOR_INDEX)
            .map(|i| palette_colors[i].1);

        let eye_color = eye_color_fallback;
        let eye_comp = config.get_complementary_for(PortraitColorSlot::Eye, eye_vi, _eye_idx);
        let eye_secondary_drone = eye_comp == Some(DRONE_COLOR_INDEX);
        let eye_secondary = eye_comp
            .filter(|&i| i != DRONE_COLOR_INDEX)
            .map(|i| palette_colors[i].1);

        let acc_comp =
            config.get_complementary_for(PortraitColorSlot::Accessory, acc_vi, acc_idx);
        let acc_shadow_drone = acc_comp == Some(DRONE_COLOR_INDEX);
        let acc_shadow = acc_comp
            .filter(|&i| i != DRONE_COLOR_INDEX)
            .map(|i| palette_colors[i].1);

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
            skin_highlight,
            acc_shadow,
            eye_secondary,
            skin_highlight_drone,
            acc_shadow_drone,
            eye_secondary_drone,
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
    fn all_face_shapes_reachable() {
        let mut seen = std::collections::HashSet::new();
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            seen.insert(format!("{:?}", FaceShape::random(&mut rng)));
        }
        assert_eq!(seen.len(), ALL_FACE_SHAPES.len());
    }

    #[test]
    fn all_eye_styles_reachable() {
        let mut seen = std::collections::HashSet::new();
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            seen.insert(format!("{:?}", EyeStyle::random(&mut rng)));
        }
        assert_eq!(seen.len(), ALL_EYE_STYLES.len());
    }

    #[test]
    fn all_mouth_styles_reachable() {
        let mut seen = std::collections::HashSet::new();
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            seen.insert(format!("{:?}", MouthStyle::random(&mut rng)));
        }
        assert_eq!(seen.len(), ALL_MOUTH_STYLES.len());
    }

    #[test]
    fn all_hair_styles_reachable() {
        let mut seen = std::collections::HashSet::new();
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            seen.insert(format!("{:?}", HairStyle::random(&mut rng)));
        }
        assert_eq!(seen.len(), ALL_HAIR_STYLES.len());
    }

    #[test]
    fn all_accessories_reachable() {
        let mut seen = std::collections::HashSet::new();
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            seen.insert(format!("{:?}", Accessory::random(&mut rng)));
        }
        assert_eq!(seen.len(), ALL_ACCESSORIES.len());
    }

    #[test]
    fn all_shirt_styles_reachable() {
        let mut seen = std::collections::HashSet::new();
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            seen.insert(format!("{:?}", ShirtStyle::random(&mut rng)));
        }
        assert_eq!(seen.len(), ALL_SHIRT_STYLES.len());
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
    fn accessory_alias_backward_compat() {
        // Old variant names should deserialize to new ones
        let old_names = [
            "Necklace", "Scar", "SpikedCollar", "FacePaint",
            "Piercings", "Earpiece", "Earring", "GogglesUp", "Antenna",
        ];
        for name in old_names {
            let ron_str = format!("{name}");
            let result: Result<Accessory, _> = ron::from_str(&ron_str);
            assert!(result.is_ok(), "Failed to deserialize alias '{name}': {result:?}");
        }
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
