use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::palette;

const CONFIG_PATH: &str = "assets/dev/portrait_palette.ron";

/// Minimum number of allowed drone colors (one per race slot).
pub const MIN_DRONE_COLORS: usize = 12;

/// Which color pool a portrait layer draws from.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PortraitColorSlot {
    Skin,
    Hair,
    Eye,
    Shirt,
    Accessory,
    Drone,
}

impl PortraitColorSlot {
    pub fn has_secondary(self) -> bool {
        matches!(
            self,
            PortraitColorSlot::Skin | PortraitColorSlot::Eye | PortraitColorSlot::Accessory
        )
    }
}

/// Persisted portrait palette configuration.
#[derive(Resource, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortraitPaletteConfig {
    #[serde(default)]
    pub vetoed: HashMap<PortraitColorSlot, HashSet<usize>>,
    #[serde(default)]
    pub complementary: HashMap<PortraitColorSlot, HashMap<usize, usize>>,
}

impl PortraitPaletteConfig {
    pub fn is_vetoed(&self, slot: PortraitColorSlot, index: usize) -> bool {
        self.vetoed
            .get(&slot)
            .is_some_and(|set| set.contains(&index))
    }

    pub fn toggle_veto(&mut self, slot: PortraitColorSlot, index: usize) {
        let set = self.vetoed.entry(slot).or_default();
        if !set.remove(&index) {
            set.insert(index);
        }
    }

    pub fn get_complementary(&self, slot: PortraitColorSlot, primary_index: usize) -> Option<usize> {
        self.complementary
            .get(&slot)
            .and_then(|map| map.get(&primary_index).copied())
    }

    pub fn set_complementary(
        &mut self,
        slot: PortraitColorSlot,
        primary_index: usize,
        secondary_index: usize,
    ) {
        self.complementary
            .entry(slot)
            .or_default()
            .insert(primary_index, secondary_index);
    }

    pub fn clear_complementary(&mut self, slot: PortraitColorSlot, primary_index: usize) {
        if let Some(map) = self.complementary.get_mut(&slot) {
            map.remove(&primary_index);
        }
    }

    pub fn allowed_indices(&self, slot: PortraitColorSlot) -> Vec<usize> {
        let vetoed = self.vetoed.get(&slot);
        (0..PALETTE_COLORS.len())
            .filter(|i| !vetoed.is_some_and(|set| set.contains(i)))
            .collect()
    }

    pub fn reset_slot(&mut self, slot: PortraitColorSlot) {
        self.vetoed.remove(&slot);
        self.complementary.remove(&slot);
    }

    pub fn drone_colors_allowed(&self) -> usize {
        self.allowed_indices(PortraitColorSlot::Drone).len()
    }

    pub fn reset_all(&mut self) {
        self.vetoed.clear();
        self.complementary.clear();
    }
}

pub fn load_config() -> PortraitPaletteConfig {
    load_config_from(Path::new(CONFIG_PATH))
}

fn load_config_from(path: &Path) -> PortraitPaletteConfig {
    match std::fs::read_to_string(path) {
        Ok(text) => ron::from_str(&text).unwrap_or_else(|e| {
            warn!("Failed to parse portrait palette config: {e}");
            PortraitPaletteConfig::default()
        }),
        Err(_) => PortraitPaletteConfig::default(),
    }
}

pub fn save_config(config: &PortraitPaletteConfig) {
    save_config_to(config, Path::new(CONFIG_PATH));
}

fn save_config_to(config: &PortraitPaletteConfig, path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let pretty = ron::ser::PrettyConfig::default();
    match ron::ser::to_string_pretty(config, pretty) {
        Ok(text) => {
            if let Err(e) = std::fs::write(path, text) {
                warn!("Failed to write portrait palette config: {e}");
            } else {
                info!("Portrait palette config saved to {}", path.display());
            }
        }
        Err(e) => warn!("Failed to serialize portrait palette config: {e}"),
    }
}

// 64 palette colors: (name, sRGB [0..1])
pub const PALETTE_COLORS: [(&str, [f32; 3]); 64] = [
    ("Black", color_to_rgb(palette::BLACK)),
    ("Smoky Black", color_to_rgb(palette::SMOKY_BLACK)),
    ("Indigo", color_to_rgb(palette::INDIGO)),
    ("Steel", color_to_rgb(palette::STEEL)),
    ("Stone", color_to_rgb(palette::STONE)),
    ("Chainmail", color_to_rgb(palette::CHAINMAIL)),
    ("Sidewalk", color_to_rgb(palette::SIDEWALK)),
    ("Sand", color_to_rgb(palette::SAND)),
    ("Tan", color_to_rgb(palette::TAN)),
    ("Vanilla", color_to_rgb(palette::VANILLA)),
    // Blues
    ("Sapphire", color_to_rgb(palette::SAPPHIRE)),
    ("Teal", color_to_rgb(palette::TEAL)),
    ("Homeworld", color_to_rgb(palette::HOMEWORLD)),
    ("Carolina", color_to_rgb(palette::CAROLINA)),
    ("Cerulean", color_to_rgb(palette::CERULEAN)),
    ("Sky", color_to_rgb(palette::SKY)),
    // Purples
    ("Violet", color_to_rgb(palette::VIOLET)),
    ("Royal", color_to_rgb(palette::ROYAL)),
    ("Ultramarine", color_to_rgb(palette::ULTRAMARINE)),
    ("Slate", color_to_rgb(palette::SLATE)),
    ("Periwinkle", color_to_rgb(palette::PERIWINKLE)),
    ("Amethyst", color_to_rgb(palette::AMETHYST)),
    ("Orchid", color_to_rgb(palette::ORCHID)),
    ("Lilac", color_to_rgb(palette::LILAC)),
    ("Lavender", color_to_rgb(palette::LAVENDER)),
    // Pinks & reds
    ("Burgundy", color_to_rgb(palette::BURGUNDY)),
    ("Eggplant", color_to_rgb(palette::EGGPLANT)),
    ("Grape", color_to_rgb(palette::GRAPE)),
    ("Magenta", color_to_rgb(palette::MAGENTA)),
    ("Pink", color_to_rgb(palette::PINK)),
    ("Bubblegum", color_to_rgb(palette::BUBBLEGUM)),
    ("Pale Pink", color_to_rgb(palette::PALE_PINK)),
    ("Cherry", color_to_rgb(palette::CHERRY)),
    ("Neon Red", color_to_rgb(palette::NEON_RED)),
    ("Salmon", color_to_rgb(palette::SALMON)),
    ("Peach", color_to_rgb(palette::PEACH)),
    ("Maroon", color_to_rgb(palette::MAROON)),
    // Oranges
    ("Tangerine", color_to_rgb(palette::TANGERINE)),
    ("Pumpkin", color_to_rgb(palette::PUMPKIN)),
    ("Sunflower", color_to_rgb(palette::SUNFLOWER)),
    ("Dandelion", color_to_rgb(palette::DANDELION)),
    // Yellows
    ("Limon", color_to_rgb(palette::LIMON)),
    ("Sunshine", color_to_rgb(palette::SUNSHINE)),
    ("Goldenrod", color_to_rgb(palette::GOLDENROD)),
    ("Bronze", color_to_rgb(palette::BRONZE)),
    ("Lime", color_to_rgb(palette::LIME)),
    ("Acid", color_to_rgb(palette::ACID)),
    ("Avocado", color_to_rgb(palette::AVOCADO)),
    ("Olive", color_to_rgb(palette::OLIVE)),
    // Greens
    ("Grass", color_to_rgb(palette::GRASS)),
    ("Green", color_to_rgb(palette::GREEN)),
    ("Frog", color_to_rgb(palette::FROG)),
    ("Jungle", color_to_rgb(palette::JUNGLE)),
    ("Spruce", color_to_rgb(palette::SPRUCE)),
    ("Sea Foam", color_to_rgb(palette::SEA_FOAM)),
    ("Mint", color_to_rgb(palette::MINT)),
    ("Granny Smith", color_to_rgb(palette::GRANNY_SMITH)),
    ("Jade", color_to_rgb(palette::JADE)),
    ("Seagreen", color_to_rgb(palette::SEAGREEN)),
    // Browns
    ("Clay", color_to_rgb(palette::CLAY)),
    ("Dirt", color_to_rgb(palette::DIRT)),
    ("Hazelnut", color_to_rgb(palette::HAZELNUT)),
    ("Toast", color_to_rgb(palette::TOAST)),
    ("Clove", color_to_rgb(palette::CLOVE)),
];

/// Extract sRGB components from a `Color::srgb()` constant at compile time.
const fn color_to_rgb(c: Color) -> [f32; 3] {
    match c {
        Color::Srgba(srgba) => [srgba.red, srgba.green, srgba.blue],
        _ => [0.0, 0.0, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_colors_count() {
        assert_eq!(PALETTE_COLORS.len(), 64);
    }

    #[test]
    fn palette_colors_in_range() {
        for (name, rgb) in &PALETTE_COLORS {
            for (i, ch) in rgb.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(ch),
                    "{name} channel {i} out of range: {ch}"
                );
            }
        }
    }

    #[test]
    fn veto_toggle() {
        let mut config = PortraitPaletteConfig::default();
        assert!(!config.is_vetoed(PortraitColorSlot::Hair, 5));
        config.toggle_veto(PortraitColorSlot::Hair, 5);
        assert!(config.is_vetoed(PortraitColorSlot::Hair, 5));
        config.toggle_veto(PortraitColorSlot::Hair, 5);
        assert!(!config.is_vetoed(PortraitColorSlot::Hair, 5));
    }

    #[test]
    fn allowed_indices_excludes_vetoed() {
        let mut config = PortraitPaletteConfig::default();
        config.toggle_veto(PortraitColorSlot::Skin, 0);
        config.toggle_veto(PortraitColorSlot::Skin, 63);
        let allowed = config.allowed_indices(PortraitColorSlot::Skin);
        assert_eq!(allowed.len(), 62);
        assert!(!allowed.contains(&0));
        assert!(!allowed.contains(&63));
    }

    #[test]
    fn complementary_roundtrip() {
        let mut config = PortraitPaletteConfig::default();
        assert_eq!(config.get_complementary(PortraitColorSlot::Eye, 10), None);
        config.set_complementary(PortraitColorSlot::Eye, 10, 25);
        assert_eq!(config.get_complementary(PortraitColorSlot::Eye, 10), Some(25));
        config.clear_complementary(PortraitColorSlot::Eye, 10);
        assert_eq!(config.get_complementary(PortraitColorSlot::Eye, 10), None);
    }

    #[test]
    fn reset_slot_clears_only_that_slot() {
        let mut config = PortraitPaletteConfig::default();
        config.toggle_veto(PortraitColorSlot::Hair, 5);
        config.toggle_veto(PortraitColorSlot::Skin, 3);
        config.set_complementary(PortraitColorSlot::Hair, 5, 10);
        config.reset_slot(PortraitColorSlot::Hair);
        assert!(!config.is_vetoed(PortraitColorSlot::Hair, 5));
        assert!(config.is_vetoed(PortraitColorSlot::Skin, 3));
    }

    #[test]
    fn serde_roundtrip() {
        let mut config = PortraitPaletteConfig::default();
        config.toggle_veto(PortraitColorSlot::Hair, 2);
        config.toggle_veto(PortraitColorSlot::Hair, 14);
        config.set_complementary(PortraitColorSlot::Eye, 10, 25);

        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&config, pretty).unwrap();
        let deserialized: PortraitPaletteConfig = ron::from_str(&serialized).unwrap();

        assert!(deserialized.is_vetoed(PortraitColorSlot::Hair, 2));
        assert!(deserialized.is_vetoed(PortraitColorSlot::Hair, 14));
        assert_eq!(
            deserialized.get_complementary(PortraitColorSlot::Eye, 10),
            Some(25)
        );
    }

    #[test]
    fn load_missing_file_returns_default() {
        let config = load_config_from(Path::new("nonexistent/path.ron"));
        assert!(config.vetoed.is_empty());
        assert!(config.complementary.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_palette.ron");

        let mut config = PortraitPaletteConfig::default();
        config.toggle_veto(PortraitColorSlot::Skin, 0);
        config.set_complementary(PortraitColorSlot::Accessory, 12, 34);

        save_config_to(&config, &path);
        let loaded = load_config_from(&path);

        assert!(loaded.is_vetoed(PortraitColorSlot::Skin, 0));
        assert_eq!(
            loaded.get_complementary(PortraitColorSlot::Accessory, 12),
            Some(34)
        );
    }
}
