use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "assets/dev/portrait_palette.ron";

/// Minimum number of allowed drone colors (one per race slot).
pub const MIN_DRONE_COLORS: usize = 12;

/// Sentinel index used in complementary maps to mean "use the pilot's drone color."
pub const DRONE_COLOR_INDEX: usize = usize::MAX;

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


/// Per-variant override for vetoes and complementary mappings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VariantOverride {
    pub vetoed: HashSet<usize>,
    #[serde(default)]
    pub complementary: HashMap<usize, usize>,
}

/// Persisted portrait palette configuration.
#[derive(Resource, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortraitPaletteConfig {
    #[serde(default)]
    pub vetoed: HashMap<PortraitColorSlot, HashSet<usize>>,
    #[serde(default)]
    pub complementary: HashMap<PortraitColorSlot, HashMap<usize, usize>>,
    /// Per-variant overrides keyed by (slot, variant_index).
    /// When present, the variant uses its own vetoes/complementary instead of the shared slot data.
    #[serde(default)]
    pub variant_overrides: HashMap<PortraitColorSlot, HashMap<usize, VariantOverride>>,
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

    #[cfg_attr(not(test), allow(dead_code))]
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
        self.variant_overrides.remove(&slot);
    }

    pub fn drone_colors_allowed(&self) -> usize {
        self.allowed_indices(PortraitColorSlot::Drone).len()
    }

    pub fn reset_all(&mut self) {
        self.vetoed.clear();
        self.complementary.clear();
        self.variant_overrides.clear();
    }

    // ── Variant override methods ─────────────────────────────────────────

    fn get_override(&self, slot: PortraitColorSlot, variant_idx: usize) -> Option<&VariantOverride> {
        self.variant_overrides.get(&slot)?.get(&variant_idx)
    }

    fn get_override_mut(
        &mut self,
        slot: PortraitColorSlot,
        variant_idx: usize,
    ) -> Option<&mut VariantOverride> {
        self.variant_overrides.get_mut(&slot)?.get_mut(&variant_idx)
    }

    pub fn is_variant_unique(&self, slot: PortraitColorSlot, variant_idx: usize) -> bool {
        self.get_override(slot, variant_idx).is_some()
    }

    /// Copy the shared slot's vetoes+complementary into a new per-variant override.
    pub fn make_variant_unique(&mut self, slot: PortraitColorSlot, variant_idx: usize) {
        let vetoed = self.vetoed.get(&slot).cloned().unwrap_or_default();
        let complementary = self.complementary.get(&slot).cloned().unwrap_or_default();
        self.variant_overrides
            .entry(slot)
            .or_default()
            .insert(variant_idx, VariantOverride { vetoed, complementary });
    }

    pub fn revert_variant_to_default(&mut self, slot: PortraitColorSlot, variant_idx: usize) {
        if let Some(map) = self.variant_overrides.get_mut(&slot) {
            map.remove(&variant_idx);
            if map.is_empty() {
                self.variant_overrides.remove(&slot);
            }
        }
    }

    /// Resolve vetoes: use variant override if present, else shared slot data.
    pub fn allowed_indices_for(
        &self,
        slot: PortraitColorSlot,
        variant_idx: Option<usize>,
    ) -> Vec<usize> {
        if let Some(vi) = variant_idx
            && let Some(ovr) = self.get_override(slot, vi)
        {
            return (0..PALETTE_COLORS.len())
                .filter(|i| !ovr.vetoed.contains(i))
                .collect();
        }
        self.allowed_indices(slot)
    }

    pub fn toggle_veto_for(
        &mut self,
        slot: PortraitColorSlot,
        variant_idx: Option<usize>,
        color_idx: usize,
    ) {
        if let Some(vi) = variant_idx
            && let Some(ovr) = self.get_override_mut(slot, vi)
        {
            if !ovr.vetoed.remove(&color_idx) {
                ovr.vetoed.insert(color_idx);
            }
            return;
        }
        self.toggle_veto(slot, color_idx);
    }

    pub fn get_complementary_for(
        &self,
        slot: PortraitColorSlot,
        variant_idx: Option<usize>,
        primary_index: usize,
    ) -> Option<usize> {
        if let Some(vi) = variant_idx
            && let Some(ovr) = self.get_override(slot, vi)
        {
            return ovr.complementary.get(&primary_index).copied();
        }
        self.get_complementary(slot, primary_index)
    }

    pub fn set_complementary_for(
        &mut self,
        slot: PortraitColorSlot,
        variant_idx: Option<usize>,
        primary_index: usize,
        secondary_index: usize,
    ) {
        if let Some(vi) = variant_idx
            && let Some(ovr) = self.get_override_mut(slot, vi)
        {
            ovr.complementary.insert(primary_index, secondary_index);
            return;
        }
        self.set_complementary(slot, primary_index, secondary_index);
    }

    #[allow(dead_code)]
    pub fn clear_complementary_for(
        &mut self,
        slot: PortraitColorSlot,
        variant_idx: Option<usize>,
        primary_index: usize,
    ) {
        if let Some(vi) = variant_idx
            && let Some(ovr) = self.get_override_mut(slot, vi)
        {
            ovr.complementary.remove(&primary_index);
            return;
        }
        self.clear_complementary(slot, primary_index);
    }

    pub fn auto_assign_all_for(&mut self, slot: PortraitColorSlot, variant_idx: Option<usize>) {
        let allowed = self.allowed_indices_for(slot, variant_idx);
        for &primary_idx in &allowed {
            if self
                .get_complementary_for(slot, variant_idx, primary_idx)
                .is_some()
            {
                continue;
            }
            // Default: use pilot's drone color
            self.set_complementary_for(slot, variant_idx, primary_idx, DRONE_COLOR_INDEX);
        }
    }
}

pub fn load_config() -> PortraitPaletteConfig {
    load_config_from(Path::new(CONFIG_PATH))
}

fn load_config_from(path: &Path) -> PortraitPaletteConfig {
    crate::persistence::load_ron_or_default(path)
}

pub fn save_config(config: &PortraitPaletteConfig) {
    save_config_to(config, Path::new(CONFIG_PATH));
}

fn save_config_to(config: &PortraitPaletteConfig, path: &Path) {
    match crate::persistence::save_ron(config, path) {
        Ok(()) => info!("Portrait palette config saved to {}", path.display()),
        Err(e) => warn!("Failed to save portrait palette config: {e}"),
    }
}

pub use super::color_picker_data::PALETTE_COLORS;

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn make_variant_unique_copies_shared_vetoes() {
        let mut config = PortraitPaletteConfig::default();
        config.toggle_veto(PortraitColorSlot::Eye, 5);
        config.toggle_veto(PortraitColorSlot::Eye, 10);
        config.set_complementary(PortraitColorSlot::Eye, 3, 20);

        assert!(!config.is_variant_unique(PortraitColorSlot::Eye, 3));
        config.make_variant_unique(PortraitColorSlot::Eye, 3); // Visor
        assert!(config.is_variant_unique(PortraitColorSlot::Eye, 3));

        // Override starts with a copy of shared vetoes
        let allowed = config.allowed_indices_for(PortraitColorSlot::Eye, Some(3));
        assert!(!allowed.contains(&5));
        assert!(!allowed.contains(&10));
        assert_eq!(allowed.len(), 62);

        // Override starts with a copy of shared complementary
        assert_eq!(
            config.get_complementary_for(PortraitColorSlot::Eye, Some(3), 3),
            Some(20)
        );
    }

    #[test]
    fn variant_override_is_independent() {
        let mut config = PortraitPaletteConfig::default();
        config.toggle_veto(PortraitColorSlot::Eye, 5);
        config.make_variant_unique(PortraitColorSlot::Eye, 3);

        // Veto color 20 only in the override
        config.toggle_veto_for(PortraitColorSlot::Eye, Some(3), 20);
        assert!(!config.allowed_indices_for(PortraitColorSlot::Eye, Some(3)).contains(&20));
        // Shared slot still allows 20
        assert!(config.allowed_indices(PortraitColorSlot::Eye).contains(&20));

        // Non-unique variant (index 0) still uses shared
        assert!(config.allowed_indices_for(PortraitColorSlot::Eye, Some(0)).contains(&20));
        assert!(!config.allowed_indices_for(PortraitColorSlot::Eye, Some(0)).contains(&5));
    }

    #[test]
    fn revert_variant_removes_override() {
        let mut config = PortraitPaletteConfig::default();
        config.make_variant_unique(PortraitColorSlot::Eye, 3);
        config.toggle_veto_for(PortraitColorSlot::Eye, Some(3), 20);

        config.revert_variant_to_default(PortraitColorSlot::Eye, 3);
        assert!(!config.is_variant_unique(PortraitColorSlot::Eye, 3));
        // Falls back to shared (which has no vetoes)
        assert!(config.allowed_indices_for(PortraitColorSlot::Eye, Some(3)).contains(&20));
    }

    #[test]
    fn reset_slot_clears_variant_overrides() {
        let mut config = PortraitPaletteConfig::default();
        config.make_variant_unique(PortraitColorSlot::Eye, 3);
        config.toggle_veto_for(PortraitColorSlot::Eye, Some(3), 5);

        config.reset_slot(PortraitColorSlot::Eye);
        assert!(!config.is_variant_unique(PortraitColorSlot::Eye, 3));
    }

    #[test]
    fn variant_override_serde_roundtrip() {
        let mut config = PortraitPaletteConfig::default();
        config.make_variant_unique(PortraitColorSlot::Eye, 3);
        config.toggle_veto_for(PortraitColorSlot::Eye, Some(3), 7);
        config.set_complementary_for(PortraitColorSlot::Eye, Some(3), 10, 30);

        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&config, pretty).unwrap();
        let deserialized: PortraitPaletteConfig = ron::from_str(&serialized).unwrap();

        assert!(deserialized.is_variant_unique(PortraitColorSlot::Eye, 3));
        assert!(!deserialized.allowed_indices_for(PortraitColorSlot::Eye, Some(3)).contains(&7));
        assert_eq!(
            deserialized.get_complementary_for(PortraitColorSlot::Eye, Some(3), 10),
            Some(30)
        );
    }

    #[test]
    fn variant_override_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_variant.ron");

        let mut config = PortraitPaletteConfig::default();
        config.make_variant_unique(PortraitColorSlot::Accessory, 1);
        config.toggle_veto_for(PortraitColorSlot::Accessory, Some(1), 15);

        save_config_to(&config, &path);
        let loaded = load_config_from(&path);

        assert!(loaded.is_variant_unique(PortraitColorSlot::Accessory, 1));
        assert!(!loaded.allowed_indices_for(PortraitColorSlot::Accessory, Some(1)).contains(&15));
    }
}
