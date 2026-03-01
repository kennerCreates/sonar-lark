use bevy::prelude::Color;

use crate::drone::components::DroneIdentity;
use crate::palette;
use crate::pilot::SelectedPilots;

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

/// Resolve display name for a drone, preferring pilot gamertag over drone callsign.
pub fn resolve_drone_name<'a>(
    selected: Option<&'a SelectedPilots>,
    index: usize,
    identity: Option<&'a DroneIdentity>,
) -> &'a str {
    selected
        .and_then(|s| s.pilots.get(index))
        .map(|p| p.gamertag.as_str())
        .or_else(|| identity.map(|id| id.name.as_str()))
        .unwrap_or("???")
}

/// Resolve display color for a drone, preferring pilot color over drone color.
pub fn resolve_drone_color(
    selected: Option<&SelectedPilots>,
    index: usize,
    identity: Option<&DroneIdentity>,
) -> Color {
    selected
        .and_then(|s| s.pilots.get(index))
        .map(|p| p.color)
        .or_else(|| identity.map(|id| id.color))
        .unwrap_or(palette::VANILLA)
}
