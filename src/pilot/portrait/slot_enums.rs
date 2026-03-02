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
}

pub const ALL_FACE_SHAPES: [FaceShape; 4] = [
    FaceShape::Oval,
    FaceShape::Round,
    FaceShape::Square,
    FaceShape::Angular,
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
            FaceShape::Oval => "oval",
            FaceShape::Round => "round",
            FaceShape::Square => "square",
            FaceShape::Angular => "angular",
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
}

pub const ALL_EYE_STYLES: [EyeStyle; 4] = [
    EyeStyle::Normal,
    EyeStyle::Narrow,
    EyeStyle::Wide,
    EyeStyle::Visor,
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
            EyeStyle::Normal => "normal",
            EyeStyle::Narrow => "narrow",
            EyeStyle::Wide => "wide",
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
    Frown,
}

pub const ALL_MOUTH_STYLES: [MouthStyle; 4] = [
    MouthStyle::Neutral,
    MouthStyle::Smile,
    MouthStyle::Smirk,
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
            MouthStyle::Frown => "frown",
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
}

pub const ALL_HAIR_STYLES: [HairStyle; 6] = [
    HairStyle::ShortCrop,
    HairStyle::Mohawk,
    HairStyle::LongSwept,
    HairStyle::Helmet,
    HairStyle::Beanie,
    HairStyle::Bald,
];

impl HairStyle {
    pub(crate) fn random(rng: &mut impl Rng) -> Self {
        ALL_HAIR_STYLES[rng.gen_range(0..ALL_HAIR_STYLES.len())]
    }

    pub fn index(&self) -> usize {
        ALL_HAIR_STYLES.iter().position(|s| s == self).unwrap()
    }

    pub fn is_bald(&self) -> bool {
        matches!(self, HairStyle::Bald)
    }

    pub fn group_id(&self) -> &'static str {
        match self {
            HairStyle::ShortCrop => "short_crop",
            HairStyle::Mohawk => "mohawk",
            HairStyle::LongSwept => "long_sweep",
            HairStyle::Helmet | HairStyle::Beanie => "beanie",
            HairStyle::Bald => "bald",
        }
    }
}

// ── Accessory ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EarringKind {
    Round,
    Ring,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum NecklaceKind {
    Chain,
    Pendant,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Accessory {
    Earring(EarringKind),
    Necklace(NecklaceKind),
}

pub const ALL_ACCESSORIES: [Accessory; 4] = [
    Accessory::Earring(EarringKind::Round),
    Accessory::Earring(EarringKind::Ring),
    Accessory::Necklace(NecklaceKind::Chain),
    Accessory::Necklace(NecklaceKind::Pendant),
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
            Accessory::Earring(EarringKind::Round) => "earring_round",
            Accessory::Earring(EarringKind::Ring) => "earring_ring",
            Accessory::Necklace(NecklaceKind::Chain) => "necklace_chain",
            Accessory::Necklace(NecklaceKind::Pendant) => "necklace_pendant",
        }
    }

    /// Whether this accessory uses a secondary (shadow) color in its SVG.
    pub fn has_shadow(&self) -> bool {
        matches!(self, Accessory::Necklace(NecklaceKind::Pendant))
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

}
