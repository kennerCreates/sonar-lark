use std::collections::{HashMap, HashSet};

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;

use crate::palette;
use crate::pilot::portrait::loader::PortraitParts;
use crate::pilot::portrait::rasterize::rasterize_portrait;
use crate::pilot::portrait::{
    Accessory, EyeStyle, FaceShape, HairStyle, MouthStyle, PortraitDescriptor, ShirtStyle,
    ALL_ACCESSORIES, ALL_EYE_STYLES, ALL_FACE_SHAPES, ALL_HAIR_STYLES, ALL_MOUTH_STYLES,
    ALL_SHIRT_STYLES,
};
use crate::states::AppState;

use super::portrait_config::{
    MIN_DRONE_COLORS, PALETTE_COLORS, PortraitColorSlot, PortraitPaletteConfig,
    auto_secondary_index, save_config,
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
    pub secondary_colors: HashMap<PortraitColorSlot, usize>,
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

        let mut secondary_colors = HashMap::new();
        secondary_colors.insert(PortraitColorSlot::Skin, 9); // Vanilla (highlight)
        secondary_colors.insert(PortraitColorSlot::Eye, 11); // Teal (iris)
        secondary_colors.insert(PortraitColorSlot::Accessory, 3); // Steel (shadow)

        Self {
            active_tab: EditorTab::Face,
            face_shape: FaceShape::Oval,
            eye_style: EyeStyle::Normal,
            mouth_style: MouthStyle::Neutral,
            hair_style: HairStyle::ShortCrop,
            shirt_style: ShirtStyle::Crew,
            accessory: Some(Accessory::EarringRound),
            primary_colors,
            secondary_colors,
            selected_pairing_primary: None,
            preview_dirty: true,
            preview_handle: None,
        }
    }
}

impl PortraitEditorState {
    /// Whether the current tab+state should show a secondary color grid.
    /// Only Eye/Visor uses the old single-secondary grid.
    fn show_secondary(&self) -> bool {
        matches!(self.active_tab.color_slot(), Some(PortraitColorSlot::Eye))
            && self.eye_style == EyeStyle::Visor
    }

    /// Whether the current tab should show the paired swatch list.
    fn show_pairing(&self) -> bool {
        self.active_tab
            .color_slot()
            .is_some_and(|s| s.needs_pairing())
    }

    /// Returns the variant index for the current tab's selected variant.
    /// None for Mouth (no color slot) and Drone (no variants).
    fn current_variant_index(&self) -> Option<usize> {
        match self.active_tab {
            EditorTab::Face => ALL_FACE_SHAPES.iter().position(|s| *s == self.face_shape),
            EditorTab::Eyes => ALL_EYE_STYLES.iter().position(|s| *s == self.eye_style),
            EditorTab::Mouth => None,
            EditorTab::Hair => ALL_HAIR_STYLES.iter().position(|s| *s == self.hair_style),
            EditorTab::Shirt => ALL_SHIRT_STYLES.iter().position(|s| *s == self.shirt_style),
            EditorTab::Accessory => self
                .accessory
                .and_then(|a| ALL_ACCESSORIES.iter().position(|x| *x == a)),
            EditorTab::Drone => None,
        }
    }

    fn build_descriptor(&self, config: &PortraitPaletteConfig) -> PortraitDescriptor {
        let skin_tone = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Skin]].1;
        let hair_color = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Hair]].1;
        let eye_color = if self.eye_style == EyeStyle::Visor {
            // Visor: use secondary eye color if set, fallback to primary
            PALETTE_COLORS
                .get(
                    self.secondary_colors
                        .get(&PortraitColorSlot::Eye)
                        .copied()
                        .unwrap_or(self.primary_colors[&PortraitColorSlot::Eye]),
                )
                .map(|c| c.1)
                .unwrap_or(hair_color)
        } else {
            // Normal eyes: primary Eye pick = iris color
            PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Eye]].1
        };
        let shirt_color = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Shirt]].1;
        let acc_idx = self.primary_colors[&PortraitColorSlot::Accessory];
        let accessory_color = PALETTE_COLORS[acc_idx].1;

        let skin_idx = self.primary_colors[&PortraitColorSlot::Skin];
        let face_vi = ALL_FACE_SHAPES.iter().position(|s| *s == self.face_shape);
        let skin_highlight = config
            .get_complementary_for(PortraitColorSlot::Skin, face_vi, skin_idx)
            .map(|i| PALETTE_COLORS[i].1);
        let acc_vi = self
            .accessory
            .and_then(|a| ALL_ACCESSORIES.iter().position(|x| *x == a));
        let acc_shadow = config
            .get_complementary_for(PortraitColorSlot::Accessory, acc_vi, acc_idx)
            .map(|i| PALETTE_COLORS[i].1);

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
            skin_highlight,
            acc_shadow,
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

#[derive(Component)]
pub struct SecondaryColorCell(pub usize);

/// Secondary cell in paired grid; stores the *primary* palette index it corresponds to.
#[derive(Component)]
pub struct SecondaryPairingCell(pub usize);

#[derive(Component)]
pub struct PairingPickerPanel;

#[derive(Component)]
pub struct PairingPickerCell(pub usize);

#[derive(Component)]
pub struct AutoAssignAllButton;

#[derive(Component)]
pub struct AllowedGridPanel;

#[derive(Component)]
pub struct VetoedGridPanel;

#[derive(Component)]
pub struct PreviewImage;

#[derive(Component)]
pub struct SecondaryPanel;

#[derive(Component)]
pub struct VariantPanel;

#[derive(Component)]
pub struct PrimarySection;

#[derive(Component)]
pub struct PrimaryGridPanel;

#[derive(Component)]
pub struct SecondaryGridPanel;

#[derive(Component)]
pub struct SaveButton;

#[derive(Component)]
pub struct ResetSlotButton;

#[derive(Component)]
pub struct ResetAllButton;

#[derive(Component)]
pub struct AutoSecondaryButton;

#[derive(Component)]
pub struct ColorNameLabel;

#[derive(Component)]
pub struct DroneWarningLabel;

#[derive(Component)]
pub struct MakeUniqueButton;

#[derive(Component)]
pub struct UniqueStatusRow;

// ── Setup ───────────────────────────────────────────────────────────────────

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

        build_ui(&mut commands, &state, &config, Some(handle));
    } else {
        build_ui(&mut commands, &state, &config, None);
    }

    commands.insert_resource(state);
    commands.insert_resource(config);
}

fn build_ui(
    commands: &mut Commands,
    state: &PortraitEditorState,
    config: &PortraitPaletteConfig,
    preview_handle: Option<Handle<Image>>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DespawnOnExit(AppState::DevMenu),
        ))
        .with_children(|root| {
            // Header row
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("PORTRAIT PALETTE EDITOR"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(palette::VANILLA),
                ));
                header
                    .spawn((
                        Button,
                        BackButton,
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(palette::INDIGO),
                        BorderColor::all(palette::STEEL),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("BACK"),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(palette::VANILLA),
                        ));
                    });
            });

            // Main content: preview (left) + controls (right)
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|main| {
                // Left column: preview
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    width: Val::Px(520.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|left| {
                    left.spawn((
                        Text::new("Preview"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(palette::SIDEWALK),
                    ));

                    // Preview image container
                    let mut preview_cmd = left.spawn((
                        PreviewImage,
                        Node {
                            width: Val::Px(512.0),
                            height: Val::Px(512.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor::all(palette::STEEL),
                        BackgroundColor(palette::SMOKY_BLACK),
                    ));
                    if let Some(handle) = preview_handle {
                        preview_cmd.insert(ImageNode::new(handle));
                    }

                    // Color name label
                    left.spawn((
                        ColorNameLabel,
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(palette::SIDEWALK),
                    ));
                });

                // Right column: controls
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|right| {
                    // Tab row
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|tab_row| {
                            for tab in EditorTab::ALL {
                                let is_active = tab == state.active_tab;
                                tab_row
                                    .spawn((
                                        Button,
                                        PartTab(tab),
                                        Node {
                                            width: Val::Px(56.0),
                                            height: Val::Px(28.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(if is_active {
                                            TAB_ACTIVE
                                        } else {
                                            TAB_NORMAL
                                        }),
                                        BorderColor::all(if is_active {
                                            palette::SKY
                                        } else {
                                            palette::STEEL
                                        }),
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(tab.label()),
                                            TextFont {
                                                font_size: 13.0,
                                                ..default()
                                            },
                                            TextColor(palette::VANILLA),
                                        ));
                                    });
                            }
                        });

                    // Variant radio buttons
                    right
                        .spawn((
                            VariantPanel,
                            Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(4.0),
                                row_gap: Val::Px(4.0),
                                ..default()
                            },
                        ))
                        .with_children(|variant_area| {
                            spawn_variant_buttons(variant_area, state, config);
                        });

                    // Unique variant toggle row
                    right.spawn((
                        UniqueStatusRow,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Visibility::Hidden,
                    ));

                    // Color grids row: primary (left) + pairing (right)
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(16.0),
                            align_items: AlignItems::Start,
                            ..default()
                        })
                        .with_children(|color_row| {
                            // Primary color grid (two-column: allowed / vetoed)
                            color_row
                                .spawn((
                                    PrimarySection,
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(4.0),
                                        ..default()
                                    },
                                    if state.active_tab.color_slot().is_some() {
                                        Visibility::Visible
                                    } else {
                                        Visibility::Hidden
                                    },
                                ))
                                .with_children(|section| {
                                    // Header row: label + auto-assign (for pairing tabs)
                                    section
                                        .spawn(Node {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(8.0),
                                            align_items: AlignItems::Center,
                                            ..default()
                                        })
                                        .with_children(|hdr| {
                                            hdr.spawn((
                                                Text::new(
                                                    "Primary Color (left-click = select, right-click = move)",
                                                ),
                                                TextFont {
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(palette::SIDEWALK),
                                            ));
                                            hdr.spawn((
                                                Button,
                                                AutoAssignAllButton,
                                                Node {
                                                    height: Val::Px(20.0),
                                                    padding: UiRect::horizontal(Val::Px(6.0)),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(palette::INDIGO),
                                                BorderColor::all(palette::STEEL),
                                                if state.show_pairing() {
                                                    Visibility::Visible
                                                } else {
                                                    Visibility::Hidden
                                                },
                                            ))
                                            .with_children(|btn| {
                                                btn.spawn((
                                                    Text::new("Auto-pair"),
                                                    TextFont {
                                                        font_size: 11.0,
                                                        ..default()
                                                    },
                                                    TextColor(palette::VANILLA),
                                                ));
                                            });
                                        });
                                    section
                                        .spawn((
                                            PrimaryGridPanel,
                                            Node {
                                                flex_direction: FlexDirection::Row,
                                                column_gap: Val::Px(16.0),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|grid_parent| {
                                            // Left: Allowed colors
                                            grid_parent
                                                .spawn(Node {
                                                    flex_direction: FlexDirection::Column,
                                                    row_gap: Val::Px(4.0),
                                                    ..default()
                                                })
                                                .with_children(|col| {
                                                    col.spawn((
                                                        Text::new("Allowed"),
                                                        TextFont {
                                                            font_size: 11.0,
                                                            ..default()
                                                        },
                                                        TextColor(palette::SIDEWALK),
                                                    ));
                                                    col.spawn((
                                                        AllowedGridPanel,
                                                        Node {
                                                            flex_direction: FlexDirection::Row,
                                                            flex_wrap: FlexWrap::Wrap,
                                                            column_gap: Val::Px(2.0),
                                                            row_gap: Val::Px(2.0),
                                                            max_width: Val::Px(
                                                                (COLOR_CELL_SIZE + 2.0)
                                                                    * COLOR_GRID_COLS as f32,
                                                            ),
                                                            ..default()
                                                        },
                                                    ));
                                                });
                                            // Right: Vetoed colors
                                            grid_parent
                                                .spawn(Node {
                                                    flex_direction: FlexDirection::Column,
                                                    row_gap: Val::Px(4.0),
                                                    ..default()
                                                })
                                                .with_children(|col| {
                                                    col.spawn((
                                                        Text::new("Vetoed"),
                                                        TextFont {
                                                            font_size: 11.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgba(
                                                            0.8, 0.3, 0.3, 1.0,
                                                        )),
                                                    ));
                                                    col.spawn((
                                                        VetoedGridPanel,
                                                        Node {
                                                            flex_direction: FlexDirection::Row,
                                                            flex_wrap: FlexWrap::Wrap,
                                                            column_gap: Val::Px(2.0),
                                                            row_gap: Val::Px(2.0),
                                                            max_width: Val::Px(
                                                                (COLOR_CELL_SIZE + 2.0)
                                                                    * COLOR_GRID_COLS as f32,
                                                            ),
                                                            ..default()
                                                        },
                                                    ));
                                                });
                                        });

                                    // Inline picker (hidden until a secondary cell is clicked)
                                    section.spawn((
                                        PairingPickerPanel,
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            flex_wrap: FlexWrap::Wrap,
                                            column_gap: Val::Px(2.0),
                                            row_gap: Val::Px(2.0),
                                            max_width: Val::Px(
                                                (COLOR_CELL_SIZE + 2.0)
                                                    * COLOR_GRID_COLS as f32,
                                            ),
                                            padding: UiRect::all(Val::Px(4.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
                                        BorderColor::all(palette::SUNSHINE),
                                        Visibility::Hidden,
                                    ));
                                });
                        });

                    // Drone color minimum warning (hidden by default)
                    right.spawn((
                        DroneWarningLabel,
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
                        Visibility::Hidden,
                    ));

                    // Secondary color grid (Eye/Visor only)
                    right
                        .spawn((
                            SecondaryPanel,
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                ..default()
                            },
                            if state.show_secondary() {
                                Visibility::Visible
                            } else {
                                Visibility::Hidden
                            },
                        ))
                        .with_children(|sec| {
                            sec.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn((
                                    Text::new("Secondary Color"),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(palette::SIDEWALK),
                                ));
                                row.spawn((
                                    Button,
                                    AutoSecondaryButton,
                                    Node {
                                        width: Val::Px(48.0),
                                        height: Val::Px(20.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(palette::INDIGO),
                                    BorderColor::all(palette::STEEL),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new("Auto"),
                                        TextFont {
                                            font_size: 11.0,
                                            ..default()
                                        },
                                        TextColor(palette::VANILLA),
                                    ));
                                });
                            });
                            sec.spawn((
                                SecondaryGridPanel,
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    column_gap: Val::Px(2.0),
                                    row_gap: Val::Px(2.0),
                                    max_width: Val::Px(
                                        (COLOR_CELL_SIZE + 2.0) * COLOR_GRID_COLS as f32,
                                    ),
                                    ..default()
                                },
                            ));
                        });

                    // Action buttons
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|actions| {
                            spawn_action_button(actions, "SAVE", SaveButton);
                            spawn_action_button(actions, "RESET SLOT", ResetSlotButton);
                            spawn_action_button(actions, "RESET ALL", ResetAllButton);
                        });
                });
            });
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Component) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(palette::INDIGO),
            BorderColor::all(palette::STEEL),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::VANILLA),
            ));
        });
}

fn spawn_variant_buttons(
    parent: &mut ChildSpawnerCommands,
    state: &PortraitEditorState,
    config: &PortraitPaletteConfig,
) {
    let slot = state.active_tab.color_slot();
    match state.active_tab {
        EditorTab::Face => {
            for (i, shape) in ALL_FACE_SHAPES.iter().enumerate() {
                let is_active = *shape == state.face_shape;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(parent, EditorTab::Face, i, &format!("{shape:?}"), is_active, is_unique);
            }
        }
        EditorTab::Eyes => {
            for (i, style) in ALL_EYE_STYLES.iter().enumerate() {
                let is_active = *style == state.eye_style;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(parent, EditorTab::Eyes, i, &format!("{style:?}"), is_active, is_unique);
            }
        }
        EditorTab::Mouth => {
            for (i, style) in ALL_MOUTH_STYLES.iter().enumerate() {
                let is_active = *style == state.mouth_style;
                spawn_radio_button(
                    parent,
                    EditorTab::Mouth,
                    i,
                    &format!("{style:?}"),
                    is_active,
                    false,
                );
            }
        }
        EditorTab::Hair => {
            for (i, style) in ALL_HAIR_STYLES.iter().enumerate() {
                let is_active = *style == state.hair_style;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(parent, EditorTab::Hair, i, &format!("{style:?}"), is_active, is_unique);
            }
        }
        EditorTab::Shirt => {
            for (i, style) in ALL_SHIRT_STYLES.iter().enumerate() {
                let is_active = *style == state.shirt_style;
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(
                    parent,
                    EditorTab::Shirt,
                    i,
                    &format!("{style:?}"),
                    is_active,
                    is_unique,
                );
            }
        }
        EditorTab::Accessory => {
            // "None" option (never unique)
            spawn_radio_button(
                parent,
                EditorTab::Accessory,
                ALL_ACCESSORIES.len(),
                "None",
                state.accessory.is_none(),
                false,
            );
            for (i, acc) in ALL_ACCESSORIES.iter().enumerate() {
                let is_active = state.accessory == Some(*acc);
                let is_unique = slot.is_some_and(|s| config.is_variant_unique(s, i));
                spawn_radio_button(
                    parent,
                    EditorTab::Accessory,
                    i,
                    &format!("{acc:?}"),
                    is_active,
                    is_unique,
                );
            }
        }
        EditorTab::Drone => {
            parent.spawn((
                Text::new("Portrait background color"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(palette::SIDEWALK),
            ));
        }
    }
}

fn spawn_radio_button(
    parent: &mut ChildSpawnerCommands,
    tab: EditorTab,
    index: usize,
    label: &str,
    active: bool,
    is_unique: bool,
) {
    let border_color = if active {
        palette::SKY
    } else if is_unique {
        palette::SUNSHINE
    } else {
        palette::STEEL
    };
    parent
        .spawn((
            Button,
            VariantButton { tab, index },
            Node {
                height: Val::Px(24.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(if is_unique { 2.0 } else { 1.0 })),
                ..default()
            },
            BackgroundColor(if active { RADIO_ACTIVE } else { RADIO_NORMAL }),
            BorderColor::all(border_color),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(if active {
                    palette::VANILLA
                } else if is_unique {
                    palette::SUNSHINE
                } else {
                    palette::SIDEWALK
                }),
            ));
        });
}

// ── Cleanup ─────────────────────────────────────────────────────────────────

pub fn cleanup_portrait_editor(mut commands: Commands) {
    commands.remove_resource::<PortraitEditorState>();
    commands.remove_resource::<PortraitPaletteConfig>();
}

// ── Interaction systems ─────────────────────────────────────────────────────

pub fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

pub fn handle_part_tabs(
    query: Query<(&Interaction, &PartTab), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, tab) in &query {
        if *interaction == Interaction::Pressed && state.active_tab != tab.0 {
            state.active_tab = tab.0;
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
        }
    }
}

pub fn handle_variant_selection(
    query: Query<(&Interaction, &VariantButton), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, vb) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match vb.tab {
            EditorTab::Face => {
                state.face_shape = ALL_FACE_SHAPES[vb.index];
            }
            EditorTab::Eyes => {
                state.eye_style = ALL_EYE_STYLES[vb.index];
            }
            EditorTab::Mouth => {
                state.mouth_style = ALL_MOUTH_STYLES[vb.index];
            }
            EditorTab::Hair => {
                state.hair_style = ALL_HAIR_STYLES[vb.index];
            }
            EditorTab::Shirt => {
                state.shirt_style = ALL_SHIRT_STYLES[vb.index];
            }
            EditorTab::Accessory => {
                if vb.index >= ALL_ACCESSORIES.len() {
                    state.accessory = None;
                } else {
                    state.accessory = Some(ALL_ACCESSORIES[vb.index]);
                }
            }
            EditorTab::Drone => {}
        }
        state.preview_dirty = true;
    }
}

pub fn handle_primary_color_click(
    query: Query<(&Interaction, &PrimaryColorCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            state.primary_colors.insert(slot, cell.0);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_primary_color_veto(
    query: Query<(&Interaction, &PrimaryColorCell)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut config: ResMut<PortraitPaletteConfig>,
    state: Res<PortraitEditorState>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(slot) = state.active_tab.color_slot() else {
        return;
    };
    let vi = state.current_variant_index();
    for (interaction, cell) in &query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            // Block vetoing if it would drop drone colors below minimum
            if slot == PortraitColorSlot::Drone
                && !config.is_vetoed(slot, cell.0)
                && config.drone_colors_allowed() <= MIN_DRONE_COLORS
            {
                return;
            }
            config.toggle_veto_for(slot, vi, cell.0);
        }
    }
}

pub fn handle_secondary_color_click(
    query: Query<(&Interaction, &SecondaryColorCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
    mut config: ResMut<PortraitPaletteConfig>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && state.show_secondary()
            && let Some(slot) = state.active_tab.color_slot()
        {
            state.secondary_colors.insert(slot, cell.0);
            let vi = state.current_variant_index();
            if let Some(&primary_idx) = state.primary_colors.get(&slot) {
                config.set_complementary_for(slot, vi, primary_idx, cell.0);
            }
            state.preview_dirty = true;
        }
    }
}

pub fn handle_auto_secondary(
    query: Query<&Interaction, (Changed<Interaction>, With<AutoSecondaryButton>)>,
    mut state: ResMut<PortraitEditorState>,
    mut config: ResMut<PortraitPaletteConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
            && let Some(&primary_idx) = state.primary_colors.get(&slot)
        {
            let vi = state.current_variant_index();
            config.clear_complementary_for(slot, vi, primary_idx);
            state.secondary_colors.remove(&slot);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_save_button(
    query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    config: Res<PortraitPaletteConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            if config.drone_colors_allowed() < MIN_DRONE_COLORS {
                warn!(
                    "Cannot save: need at least {} drone colors, have {}",
                    MIN_DRONE_COLORS,
                    config.drone_colors_allowed()
                );
                return;
            }
            save_config(&config);
        }
    }
}

pub fn handle_reset_slot_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ResetSlotButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            config.reset_slot(slot);
            state.preview_dirty = true;
        }
    }
}

pub fn handle_reset_all_button(
    query: Query<&Interaction, (Changed<Interaction>, With<ResetAllButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            config.reset_all();
            state.preview_dirty = true;
        }
    }
}

pub fn handle_make_unique_button(
    query: Query<&Interaction, (Changed<Interaction>, With<MakeUniqueButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
            && let Some(vi) = state.current_variant_index()
        {
            if config.is_variant_unique(slot, vi) {
                config.revert_variant_to_default(slot, vi);
            } else {
                config.make_variant_unique(slot, vi);
            }
            state.preview_dirty = true;
        }
    }
}

// ── Pairing interaction handlers ─────────────────────────────────────────────

/// Clicking a secondary pairing cell opens the picker for that primary color.
pub fn handle_secondary_pairing_click(
    query: Query<(&Interaction, &SecondaryPairingCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
) {
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed {
            if state.selected_pairing_primary == Some(cell.0) {
                // Toggle off if clicking the same cell
                state.selected_pairing_primary = None;
            } else {
                state.selected_pairing_primary = Some(cell.0);
            }
        }
    }
}

/// Clicking a color in the picker sets the complementary and closes the picker.
pub fn handle_pairing_picker_click(
    query: Query<(&Interaction, &PairingPickerCell), Changed<Interaction>>,
    mut state: ResMut<PortraitEditorState>,
    mut config: ResMut<PortraitPaletteConfig>,
) {
    let Some(primary_idx) = state.selected_pairing_primary else {
        return;
    };
    for (interaction, cell) in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
        {
            let vi = state.current_variant_index();
            config.set_complementary_for(slot, vi, primary_idx, cell.0);
            state.selected_pairing_primary = None;
            state.preview_dirty = true;
        }
    }
}

/// Dismiss the pairing picker when clicking outside it.
pub fn dismiss_pairing_picker(
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<PortraitEditorState>,
    picker_cells: Query<&Interaction, With<PairingPickerCell>>,
    secondary_cells: Query<&Interaction, With<SecondaryPairingCell>>,
) {
    if state.selected_pairing_primary.is_none() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Don't dismiss if hovering over picker or secondary cells
    for interaction in &picker_cells {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    for interaction in &secondary_cells {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            return;
        }
    }
    state.selected_pairing_primary = None;
}

pub fn handle_auto_assign_all(
    query: Query<&Interaction, (Changed<Interaction>, With<AutoAssignAllButton>)>,
    mut config: ResMut<PortraitPaletteConfig>,
    mut state: ResMut<PortraitEditorState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed
            && let Some(slot) = state.active_tab.color_slot()
            && slot.needs_pairing()
        {
            let vi = state.current_variant_index();
            config.auto_assign_all_for(slot, vi);
            state.preview_dirty = true;
        }
    }
}

// ── Preview update ──────────────────────────────────────────────────────────

pub fn update_preview(
    mut state: ResMut<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    portrait_parts: Option<Res<PortraitParts>>,
    mut images: ResMut<Assets<Image>>,
    mut preview_query: Query<(Entity, Option<&mut ImageNode>), With<PreviewImage>>,
    mut commands: Commands,
) {
    if !state.preview_dirty {
        return;
    }
    state.preview_dirty = false;

    let Some(parts) = portrait_parts else {
        return;
    };

    let descriptor = state.build_descriptor(&config);
    let bg = PALETTE_COLORS[state.primary_colors[&PortraitColorSlot::Drone]].1;
    let image = rasterize_portrait(&descriptor, bg, 512, &parts);
    let handle = images.add(image);
    state.preview_handle = Some(handle.clone());

    for (entity, existing_image) in &mut preview_query {
        if let Some(mut img) = existing_image {
            img.image = handle.clone();
        } else {
            commands.entity(entity).insert(ImageNode::new(handle.clone()));
        }
    }
}

// ── Visual updates (tab highlights, variant highlights, grid updates) ───────

pub fn update_tab_visuals(
    state: Res<PortraitEditorState>,
    mut query: Query<(&PartTab, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !state.is_changed() {
        return;
    }
    for (tab, mut bg, mut border) in &mut query {
        let is_active = tab.0 == state.active_tab;
        *bg = BackgroundColor(if is_active { TAB_ACTIVE } else { TAB_NORMAL });
        *border = BorderColor::all(if is_active {
            palette::SKY
        } else {
            palette::STEEL
        });
    }
}

pub fn rebuild_variant_panel(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    variant_panel: Query<Entity, With<VariantPanel>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    for entity in &variant_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            spawn_variant_buttons(parent, &state, &config);
        });
    }
}

pub fn rebuild_unique_status_row(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    row_query: Query<Entity, With<UniqueStatusRow>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    let slot = state.active_tab.color_slot();
    let vi = state.current_variant_index();
    // Only show for tabs with both a color slot and a variant
    let show = slot.is_some() && vi.is_some();

    for entity in &row_query {
        commands.entity(entity).insert(if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        commands.entity(entity).despawn_children();
        if let (Some(slot), Some(vi)) = (slot, vi) {
            let is_unique = config.is_variant_unique(slot, vi);
            commands.entity(entity).with_children(|row| {
                let (label, bg_color, border_color) = if is_unique {
                    ("REVERT TO DEFAULT", palette::SAPPHIRE, palette::SUNSHINE)
                } else {
                    ("MAKE UNIQUE", palette::INDIGO, palette::STEEL)
                };
                row.spawn((
                    Button,
                    MakeUniqueButton,
                    Node {
                        height: Val::Px(24.0),
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(bg_color),
                    BorderColor::all(border_color),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::VANILLA),
                    ));
                });
                if is_unique {
                    row.spawn((
                        Text::new("(variant has its own veto set)"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(palette::SUNSHINE),
                    ));
                }
            });
        }
    }
}

pub fn rebuild_primary_grid(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    allowed_panel: Query<Entity, With<AllowedGridPanel>>,
    vetoed_panel: Query<Entity, With<VetoedGridPanel>>,
    section_panel: Query<Entity, With<PrimarySection>>,
    auto_btn: Query<Entity, With<AutoAssignAllButton>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    let slot = state.active_tab.color_slot();
    let vi = state.current_variant_index();
    let selected = slot.and_then(|s| state.primary_colors.get(&s).copied());
    let needs_pairing = slot.is_some_and(|s| s.needs_pairing());

    // Hide entire primary color section when tab has no color slot (e.g. Mouth)
    for entity in &section_panel {
        commands.entity(entity).insert(if slot.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }

    // Show/hide auto-pair button
    for entity in &auto_btn {
        commands.entity(entity).insert(if needs_pairing {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }

    // Use variant-aware allowed check
    let allowed_set: HashSet<usize> = slot
        .map(|s| config.allowed_indices_for(s, vi).into_iter().collect())
        .unwrap_or_default();

    // Collect allowed indices in palette order
    let allowed_ordered: Vec<usize> = PALETTE_COLORS
        .iter()
        .enumerate()
        .filter(|(i, _)| allowed_set.contains(i))
        .map(|(i, _)| i)
        .collect();

    for entity in &allowed_panel {
        commands.entity(entity).despawn_children();

        if needs_pairing {
            // Paired layout: chunk into rows with secondary row below each primary row
            commands.entity(entity).insert(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            });
            let slot = slot.unwrap();
            commands.entity(entity).with_children(|parent| {
                for chunk in allowed_ordered.chunks(COLOR_GRID_COLS) {
                    // Row pair container
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        })
                        .with_children(|pair| {
                            // Primary row
                            pair.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|row| {
                                for &i in chunk {
                                    let rgb = &PALETTE_COLORS[i].1;
                                    spawn_color_cell(row, i, rgb, selected == Some(i));
                                }
                            });
                            // Secondary row
                            pair.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|row| {
                                for &i in chunk {
                                    let explicit = config.get_complementary_for(slot, vi, i);
                                    let sec_idx = explicit
                                        .unwrap_or_else(|| auto_secondary_index(slot, i));
                                    let rgb = &PALETTE_COLORS[sec_idx].1;
                                    let is_active =
                                        state.selected_pairing_primary == Some(i);
                                    spawn_secondary_pairing_cell(
                                        row,
                                        i,
                                        rgb,
                                        explicit.is_some(),
                                        is_active,
                                    );
                                }
                            });
                        });
                }
            });
        } else {
            // Flat wrapping layout for non-pairing tabs
            commands.entity(entity).insert(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(2.0),
                row_gap: Val::Px(2.0),
                max_width: Val::Px((COLOR_CELL_SIZE + 2.0) * COLOR_GRID_COLS as f32),
                ..default()
            });
            commands.entity(entity).with_children(|parent| {
                for &i in &allowed_ordered {
                    let rgb = &PALETTE_COLORS[i].1;
                    spawn_color_cell(parent, i, rgb, selected == Some(i));
                }
            });
        }
    }

    for entity in &vetoed_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
                if slot.is_none() || allowed_set.contains(&i) {
                    continue;
                }
                spawn_color_cell(parent, i, rgb, selected == Some(i));
            }
        });
    }
}

fn spawn_color_cell(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    rgb: &[f32; 3],
    is_selected: bool,
) {
    parent.spawn((
        Button,
        PrimaryColorCell(index),
        Node {
            width: Val::Px(COLOR_CELL_SIZE),
            height: Val::Px(COLOR_CELL_SIZE),
            border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
            ..default()
        },
        BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
        BorderColor::all(if is_selected {
            SELECTED_BORDER
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.3)
        }),
    ));
}

fn spawn_secondary_pairing_cell(
    parent: &mut ChildSpawnerCommands,
    primary_index: usize,
    rgb: &[f32; 3],
    is_explicit: bool,
    is_active: bool,
) {
    let border = if is_active {
        SELECTED_BORDER
    } else if is_explicit {
        COMPLEMENTARY_BORDER
    } else {
        Color::srgba(0.3, 0.3, 0.3, 0.5)
    };
    parent.spawn((
        Button,
        SecondaryPairingCell(primary_index),
        Node {
            width: Val::Px(COLOR_CELL_SIZE),
            height: Val::Px(COLOR_CELL_SIZE),
            border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
            ..default()
        },
        BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
        BorderColor::all(border),
    ));
}

pub fn rebuild_pairing_picker(
    state: Res<PortraitEditorState>,
    mut commands: Commands,
    picker_panel: Query<Entity, With<PairingPickerPanel>>,
) {
    if !state.is_changed() {
        return;
    }
    let show = state.selected_pairing_primary.is_some() && state.show_pairing();
    for entity in &picker_panel {
        commands.entity(entity).insert(if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        commands.entity(entity).despawn_children();
        if show {
            commands.entity(entity).with_children(|parent| {
                for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
                    parent.spawn((
                        Button,
                        PairingPickerCell(i),
                        Node {
                            width: Val::Px(COLOR_CELL_SIZE),
                            height: Val::Px(COLOR_CELL_SIZE),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
                        BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                    ));
                }
            });
        }
    }
}

pub fn rebuild_secondary_grid(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    grid_panel: Query<Entity, With<SecondaryGridPanel>>,
    sec_panel: Query<Entity, With<SecondaryPanel>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }

    // Show/hide secondary panel
    let has_secondary = state.show_secondary();
    for entity in &sec_panel {
        commands.entity(entity).insert(if has_secondary {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }

    for entity in &grid_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            let slot = state.active_tab.color_slot();
            let selected = slot.and_then(|s| state.secondary_colors.get(&s).copied());
            spawn_secondary_color_grid(parent, selected);
        });
    }
}

fn spawn_secondary_color_grid(
    parent: &mut ChildSpawnerCommands,
    selected: Option<usize>,
) {
    for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
        let is_selected = selected == Some(i);
        parent.spawn((
            Button,
            SecondaryColorCell(i),
            Node {
                width: Val::Px(COLOR_CELL_SIZE),
                height: Val::Px(COLOR_CELL_SIZE),
                border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
                ..default()
            },
            BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
            BorderColor::all(if is_selected {
                COMPLEMENTARY_BORDER
            } else {
                Color::srgba(0.0, 0.0, 0.0, 0.3)
            }),
        ));
    }
}

pub fn update_drone_warning(
    config: Res<PortraitPaletteConfig>,
    state: Res<PortraitEditorState>,
    mut query: Query<(&mut Text, &mut Visibility), With<DroneWarningLabel>>,
) {
    if !config.is_changed() && !state.is_changed() {
        return;
    }
    let allowed = config.drone_colors_allowed();
    let show = state.active_tab == EditorTab::Drone && allowed <= MIN_DRONE_COLORS;
    for (mut text, mut vis) in &mut query {
        if show {
            *vis = Visibility::Visible;
            let msg = format!(
                "Need at least {} drone colors ({} allowed) \u{2014} cannot save",
                MIN_DRONE_COLORS, allowed,
            );
            if text.0 != msg {
                text.0 = msg;
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

pub fn update_color_name_on_hover(
    primary_query: Query<(&Interaction, &PrimaryColorCell)>,
    secondary_query: Query<(&Interaction, &SecondaryColorCell)>,
    pairing_cell_query: Query<(&Interaction, &SecondaryPairingCell)>,
    pairing_picker_query: Query<(&Interaction, &PairingPickerCell)>,
    mut label_query: Query<&mut Text, With<ColorNameLabel>>,
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
) {
    let mut name = "";
    for (interaction, cell) in &primary_query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            name = PALETTE_COLORS[cell.0].0;
            break;
        }
    }
    if name.is_empty() {
        for (interaction, cell) in &secondary_query {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                name = PALETTE_COLORS[cell.0].0;
                break;
            }
        }
    }
    if name.is_empty()
        && let Some(slot) = state.active_tab.color_slot()
    {
        let vi = state.current_variant_index();
        for (interaction, cell) in &pairing_cell_query {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                let sec_idx = config
                    .get_complementary_for(slot, vi, cell.0)
                    .unwrap_or_else(|| auto_secondary_index(slot, cell.0));
                name = PALETTE_COLORS[sec_idx].0;
                break;
            }
        }
    }
    if name.is_empty() {
        for (interaction, cell) in &pairing_picker_query {
            if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
                name = PALETTE_COLORS[cell.0].0;
                break;
            }
        }
    }
    for mut text in &mut label_query {
        if text.0 != name {
            text.0 = name.to_string();
        }
    }
}

pub fn handle_button_hover_visuals(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (
            Changed<Interaction>,
            Or<(
                With<BackButton>,
                With<SaveButton>,
                With<ResetSlotButton>,
                With<ResetAllButton>,
                With<AutoSecondaryButton>,
                With<AutoAssignAllButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg, mut border) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(palette::GREEN);
                *border = BorderColor::all(palette::VANILLA);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(palette::SAPPHIRE);
                *border = BorderColor::all(palette::SIDEWALK);
            }
            Interaction::None => {
                *bg = BackgroundColor(palette::INDIGO);
                *border = BorderColor::all(palette::STEEL);
            }
        }
    }
}
