mod build;
pub mod systems;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::palette;
use crate::pilot::portrait::loader::PortraitParts;
use crate::pilot::portrait::rasterize::rasterize_portrait;
use crate::pilot::portrait::{
    Accessory, EyeStyle, FaceShape, HairStyle, MouthStyle, PortraitDescriptor, SecondaryColor,
    ShirtStyle,
};
use super::portrait_config::{
    DRONE_COLOR_INDEX, PALETTE_COLORS, PortraitColorSlot, PortraitPaletteConfig,
};

// ── Colors ──────────────────────────────────────────────────────────────────

const PANEL_BG: Color = Color::srgba(0.02, 0.04, 0.08, 0.95);
const TAB_NORMAL: Color = palette::INDIGO;
const TAB_ACTIVE: Color = palette::TEAL;
const RADIO_NORMAL: Color = palette::SMOKY_BLACK;
const RADIO_ACTIVE: Color = palette::SAPPHIRE;
const COLOR_CELL_SIZE: f32 = 24.0;
const COLOR_GRID_COLS: usize = 8;
const SELECTED_BORDER: Color = palette::VANILLA;
const COMPLEMENTARY_BORDER: Color = palette::SUNSHINE;

// ── Editor tab enum ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTab {
    Face,
    Eyes,
    Mouth,
    Hair,
    Shirt,
    Accessory,
    Drone,
}

impl EditorTab {
    const ALL: [EditorTab; 7] = [
        EditorTab::Face,
        EditorTab::Eyes,
        EditorTab::Mouth,
        EditorTab::Hair,
        EditorTab::Shirt,
        EditorTab::Accessory,
        EditorTab::Drone,
    ];

    fn label(self) -> &'static str {
        match self {
            EditorTab::Face => "Face",
            EditorTab::Eyes => "Eyes",
            EditorTab::Mouth => "Mouth",
            EditorTab::Hair => "Hair",
            EditorTab::Shirt => "Shirt",
            EditorTab::Accessory => "Acc",
            EditorTab::Drone => "Drone",
        }
    }

    fn color_slot(self) -> Option<PortraitColorSlot> {
        match self {
            EditorTab::Face => Some(PortraitColorSlot::Skin),
            EditorTab::Eyes => Some(PortraitColorSlot::Eye),
            EditorTab::Mouth => None,
            EditorTab::Hair => Some(PortraitColorSlot::Hair),
            EditorTab::Shirt => Some(PortraitColorSlot::Shirt),
            EditorTab::Accessory => Some(PortraitColorSlot::Accessory),
            EditorTab::Drone => Some(PortraitColorSlot::Drone),
        }
    }
}

// ── Editor state resource ───────────────────────────────────────────────────

#[derive(Resource)]
pub struct PortraitEditorState {
    pub active_tab: EditorTab,
    pub face_shape: FaceShape,
    pub eye_style: EyeStyle,
    pub mouth_style: MouthStyle,
    pub hair_style: HairStyle,
    pub shirt_style: ShirtStyle,
    pub accessory: Option<Accessory>,
    pub primary_colors: HashMap<PortraitColorSlot, usize>,
    pub selected_pairing_primary: Option<usize>,
    pub preview_dirty: bool,
    pub preview_handle: Option<Handle<Image>>,
}

impl Default for PortraitEditorState {
    fn default() -> Self {
        let mut primary_colors = HashMap::new();
        primary_colors.insert(PortraitColorSlot::Skin, 8); // Tan
        primary_colors.insert(PortraitColorSlot::Hair, 3); // Steel
        primary_colors.insert(PortraitColorSlot::Eye, 15); // Sky
        primary_colors.insert(PortraitColorSlot::Shirt, 7); // Sand
        primary_colors.insert(PortraitColorSlot::Accessory, 4); // Stone
        primary_colors.insert(PortraitColorSlot::Drone, 33); // Neon Red

        Self {
            active_tab: EditorTab::Face,
            face_shape: FaceShape::Oval,
            eye_style: EyeStyle::Normal,
            mouth_style: MouthStyle::Neutral,
            hair_style: HairStyle::ShortCrop,
            shirt_style: ShirtStyle::Crew,
            accessory: Some(Accessory::EarringRound),
            primary_colors,
            selected_pairing_primary: None,
            preview_dirty: true,
            preview_handle: None,
        }
    }
}

impl PortraitEditorState {
    /// Whether the current tab should show the paired swatch list.
    fn show_pairing(&self) -> bool {
        let Some(slot) = self.active_tab.color_slot() else {
            return false;
        };
        match slot {
            PortraitColorSlot::Skin => true,
            // Eye only shows pairing for the Visor variant
            PortraitColorSlot::Eye => self.eye_style == EyeStyle::Visor,
            // Accessory only shows pairing when the selected accessory uses a shadow
            PortraitColorSlot::Accessory => self.accessory.is_some_and(|a| a.has_shadow()),
            _ => false,
        }
    }

    /// Returns the variant index for the current tab's selected variant.
    /// None for Mouth (no color slot) and Drone (no variants).
    fn current_variant_index(&self) -> Option<usize> {
        match self.active_tab {
            EditorTab::Face => Some(self.face_shape.index()),
            EditorTab::Eyes => Some(self.eye_style.index()),
            EditorTab::Mouth => None,
            EditorTab::Hair => Some(self.hair_style.index()),
            EditorTab::Shirt => Some(self.shirt_style.index()),
            EditorTab::Accessory => self.accessory.map(|a| a.index()),
            EditorTab::Drone => None,
        }
    }

    fn build_descriptor(&self, config: &PortraitPaletteConfig) -> PortraitDescriptor {
        let skin_tone = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Skin]].1;
        let hair_color = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Hair]].1;
        let eye_idx = self.primary_colors[&PortraitColorSlot::Eye];
        let eye_color = PALETTE_COLORS[eye_idx].1;
        let shirt_color = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Shirt]].1;
        let acc_idx = self.primary_colors[&PortraitColorSlot::Accessory];
        let accessory_color = PALETTE_COLORS[acc_idx].1;

        let resolve_comp = |comp: Option<usize>| -> SecondaryColor {
            match comp {
                Some(i) if i == DRONE_COLOR_INDEX => SecondaryColor::DroneColor,
                Some(i) => SecondaryColor::Chosen(PALETTE_COLORS[i].1),
                None => SecondaryColor::DroneColor,
            }
        };

        let skin_idx = self.primary_colors[&PortraitColorSlot::Skin];
        let face_vi = Some(self.face_shape.index());
        let skin_comp = config.get_complementary_for(PortraitColorSlot::Skin, face_vi, skin_idx);
        let skin_secondary = resolve_comp(skin_comp);

        let eye_vi = Some(self.eye_style.index());
        let eye_comp = config.get_complementary_for(PortraitColorSlot::Eye, eye_vi, eye_idx);
        let eye_secondary = resolve_comp(eye_comp);

        let acc_vi = self.accessory.map(|a| a.index());
        let acc_comp = config.get_complementary_for(PortraitColorSlot::Accessory, acc_vi, acc_idx);
        let acc_secondary = resolve_comp(acc_comp);

        PortraitDescriptor {
            face_shape: self.face_shape,
            eyes: self.eye_style,
            mouth: self.mouth_style,
            hair: self.hair_style,
            shirt: self.shirt_style,
            accessory: self.accessory,
            skin_tone,
            hair_color,
            eye_color,
            accessory_color,
            shirt_color,
            skin_secondary,
            acc_secondary,
            eye_secondary,
            generated: true,
        }
    }
}

// ── Component markers ───────────────────────────────────────────────────────

#[derive(Component)]
pub struct BackButton;

#[derive(Component)]
pub struct PartTab(EditorTab);

#[derive(Component)]
pub struct VariantButton {
    tab: EditorTab,
    index: usize,
}

#[derive(Component)]
pub struct PrimaryColorCell(pub usize);

/// Secondary cell in paired grid; stores the *primary* palette index it corresponds to.
#[derive(Component)]
pub struct SecondaryPairingCell(pub usize);

#[derive(Component)]
pub struct PairingPickerPanel;

#[derive(Component)]
pub struct PairingPickerCell(pub usize);

/// The special "drone color" cell in the pairing picker (rainbow).
#[derive(Component)]
pub struct DroneColorPickerCell;

#[derive(Component)]
pub struct AutoAssignAllButton;

#[derive(Component)]
pub struct AllowedGridPanel;

#[derive(Component)]
pub struct VetoedGridPanel;

#[derive(Component)]
pub struct PreviewImage;

#[derive(Component)]
pub struct VariantPanel;

#[derive(Component)]
pub struct PrimarySection;

#[derive(Component)]
pub struct PrimaryGridPanel;

#[derive(Component)]
pub struct SaveButton;

#[derive(Component)]
pub struct ResetSlotButton;

#[derive(Component)]
pub struct ResetAllButton;

#[derive(Component)]
pub struct ColorNameLabel;

#[derive(Component)]
pub struct DroneWarningLabel;

#[derive(Component)]
pub struct MakeUniqueButton;

#[derive(Component)]
pub struct UniqueStatusRow;

// ── Setup / Cleanup ─────────────────────────────────────────────────────────

pub fn setup_portrait_editor(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    portrait_parts: Option<Res<PortraitParts>>,
) {
    let config = super::portrait_config::load_config();
    let mut state = PortraitEditorState::default();

    // If we have portrait parts, render initial preview
    if let Some(ref parts) = portrait_parts {
        let descriptor = state.build_descriptor(&config);
        let bg = PALETTE_COLORS[state.primary_colors[&PortraitColorSlot::Drone]].1;
        let image = rasterize_portrait(&descriptor, bg, 512, parts);
        let handle = images.add(image);
        state.preview_handle = Some(handle.clone());

        build::build_ui(&mut commands, &state, &config, Some(handle));
    } else {
        build::build_ui(&mut commands, &state, &config, None);
    }

    commands.insert_resource(state);
    commands.insert_resource(config);
}

pub fn cleanup_portrait_editor(mut commands: Commands) {
    commands.remove_resource::<PortraitEditorState>();
    commands.remove_resource::<PortraitPaletteConfig>();
}

// Re-export all public systems so dev_menu/mod.rs doesn't need to change paths.
pub use systems::*;
