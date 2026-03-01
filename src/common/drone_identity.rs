use bevy::prelude::Color;

use crate::palette;

pub const DRONE_COUNT: u8 = 12;

/// Per-drone colors — 12 palette colors chosen for maximum hue spread.
pub const DRONE_COLORS: [Color; 12] = [
    palette::NEON_RED,
    palette::SUNFLOWER,
    palette::LIMON,
    palette::GRASS,
    palette::FROG,
    palette::JADE,
    palette::SKY,
    palette::HOMEWORLD,
    palette::PERIWINKLE,
    palette::AMETHYST,
    palette::PINK,
    palette::VANILLA,
];

/// Callsigns for each of the 12 drones, matching `DRONE_COLORS` indices.
pub const DRONE_NAMES: [&str; 12] = [
    "FALCON", "VIPER", "HAWK", "PHANTOM",
    "SPARK", "BLITZ", "NOVA", "DRIFT",
    "SURGE", "BOLT", "ECHO", "FURY",
];

const _: () = assert!(DRONE_COLORS.len() == DRONE_COUNT as usize);
const _: () = assert!(DRONE_NAMES.len() == DRONE_COUNT as usize);
