use rand::Rng;
use serde::{Deserialize, Serialize};

// ── Face shape ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum FaceShape {
    #[default]
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
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_FACE_SHAPES[rng.gen_range(0..ALL_FACE_SHAPES.len())]
    }

    pub fn index(&self) -> usize {
        ALL_FACE_SHAPES.iter().position(|s| s == self).unwrap()
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

// ── Eye style ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum EyeStyle {
    #[default]
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
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_EYE_STYLES[rng.gen_range(0..ALL_EYE_STYLES.len())]
    }

    pub fn index(&self) -> usize {
        ALL_EYE_STYLES.iter().position(|s| s == self).unwrap()
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

// ── Mouth style ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum MouthStyle {
    #[default]
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
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_MOUTH_STYLES[rng.gen_range(0..ALL_MOUTH_STYLES.len())]
    }

    #[allow(dead_code)]
    pub fn index(&self) -> usize {
        ALL_MOUTH_STYLES.iter().position(|s| s == self).unwrap()
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

// ── Hair style ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum HairStyle {
    #[default]
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
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_HAIR_STYLES[rng.gen_range(0..ALL_HAIR_STYLES.len())]
    }

    pub fn index(&self) -> usize {
        ALL_HAIR_STYLES.iter().position(|s| s == self).unwrap()
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

// ── Accessory ───────────────────────────────────────────────────────────────

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
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_ACCESSORIES[rng.gen_range(0..ALL_ACCESSORIES.len())]
    }

    pub fn index(&self) -> usize {
        ALL_ACCESSORIES.iter().position(|s| s == self).unwrap()
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

// ── Shirt style ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ShirtStyle {
    #[default]
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
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_SHIRT_STYLES[rng.gen_range(0..ALL_SHIRT_STYLES.len())]
    }

    pub fn index(&self) -> usize {
        ALL_SHIRT_STYLES.iter().position(|s| s == self).unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn accessory_alias_backward_compat() {
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
}
