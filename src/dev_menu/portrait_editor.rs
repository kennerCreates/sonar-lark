use std::collections::HashMap;

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
    PortraitColorSlot, PortraitPaletteConfig, PALETTE_COLORS, save_config,
};

// ── Colors ──────────────────────────────────────────────────────────────────

const PANEL_BG: Color = Color::srgba(0.02, 0.04, 0.08, 0.95);
const TAB_NORMAL: Color = palette::INDIGO;
const TAB_ACTIVE: Color = palette::TEAL;
const RADIO_NORMAL: Color = palette::SMOKY_BLACK;
const RADIO_ACTIVE: Color = palette::SAPPHIRE;
const COLOR_CELL_SIZE: f32 = 24.0;
const COLOR_GRID_COLS: usize = 8;
const VETO_OVERLAY: Color = Color::srgba(0.8, 0.1, 0.1, 0.6);
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
}

impl EditorTab {
    const ALL: [EditorTab; 6] = [
        EditorTab::Face,
        EditorTab::Eyes,
        EditorTab::Mouth,
        EditorTab::Hair,
        EditorTab::Shirt,
        EditorTab::Accessory,
    ];

    fn label(self) -> &'static str {
        match self {
            EditorTab::Face => "Face",
            EditorTab::Eyes => "Eyes",
            EditorTab::Mouth => "Mouth",
            EditorTab::Hair => "Hair",
            EditorTab::Shirt => "Shirt",
            EditorTab::Accessory => "Acc",
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
            preview_dirty: true,
            preview_handle: None,
        }
    }
}

impl PortraitEditorState {
    fn build_descriptor(&self) -> PortraitDescriptor {
        let skin_tone = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Skin]].1;
        let hair_color = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Hair]].1;
        let eye_color = PALETTE_COLORS
            .get(
                self.secondary_colors
                    .get(&PortraitColorSlot::Eye)
                    .copied()
                    .unwrap_or(self.primary_colors[&PortraitColorSlot::Eye]),
            )
            .map(|c| c.1)
            .unwrap_or(hair_color);
        let shirt_color = PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Shirt]].1;
        let accessory_color =
            PALETTE_COLORS[self.primary_colors[&PortraitColorSlot::Accessory]].1;

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

#[derive(Component)]
pub struct VetoOverlay;

#[derive(Component)]
pub struct PreviewImage;

#[derive(Component)]
pub struct SecondaryPanel;

#[derive(Component)]
pub struct VariantPanel;

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
        let descriptor = state.build_descriptor();
        let bg = PALETTE_COLORS[state.primary_colors[&PortraitColorSlot::Skin]].1;
        let image = rasterize_portrait(&descriptor, bg, 128, parts);
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
    _config: &PortraitPaletteConfig,
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
                    width: Val::Px(160.0),
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
                            width: Val::Px(128.0),
                            height: Val::Px(128.0),
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
                            spawn_variant_buttons(variant_area, state);
                        });

                    // Primary color grid
                    right.spawn((
                        Text::new("Primary Color (left-click = select, right-click = veto)"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(palette::SIDEWALK),
                    ));
                    right.spawn((
                        PrimaryGridPanel,
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

                    // Secondary color grid (conditionally shown)
                    right
                        .spawn((
                            SecondaryPanel,
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                ..default()
                            },
                            if state
                                .active_tab
                                .color_slot()
                                .is_some_and(|s| s.has_secondary())
                            {
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

fn spawn_variant_buttons(parent: &mut ChildSpawnerCommands, state: &PortraitEditorState) {
    match state.active_tab {
        EditorTab::Face => {
            for (i, shape) in ALL_FACE_SHAPES.iter().enumerate() {
                let is_active = *shape == state.face_shape;
                spawn_radio_button(parent, EditorTab::Face, i, &format!("{shape:?}"), is_active);
            }
        }
        EditorTab::Eyes => {
            for (i, style) in ALL_EYE_STYLES.iter().enumerate() {
                let is_active = *style == state.eye_style;
                spawn_radio_button(parent, EditorTab::Eyes, i, &format!("{style:?}"), is_active);
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
                );
            }
        }
        EditorTab::Hair => {
            for (i, style) in ALL_HAIR_STYLES.iter().enumerate() {
                let is_active = *style == state.hair_style;
                spawn_radio_button(parent, EditorTab::Hair, i, &format!("{style:?}"), is_active);
            }
        }
        EditorTab::Shirt => {
            for (i, style) in ALL_SHIRT_STYLES.iter().enumerate() {
                let is_active = *style == state.shirt_style;
                spawn_radio_button(
                    parent,
                    EditorTab::Shirt,
                    i,
                    &format!("{style:?}"),
                    is_active,
                );
            }
        }
        EditorTab::Accessory => {
            // "None" option
            spawn_radio_button(
                parent,
                EditorTab::Accessory,
                ALL_ACCESSORIES.len(),
                "None",
                state.accessory.is_none(),
            );
            for (i, acc) in ALL_ACCESSORIES.iter().enumerate() {
                let is_active = state.accessory == Some(*acc);
                spawn_radio_button(
                    parent,
                    EditorTab::Accessory,
                    i,
                    &format!("{acc:?}"),
                    is_active,
                );
            }
        }
    }
}

fn spawn_radio_button(
    parent: &mut ChildSpawnerCommands,
    tab: EditorTab,
    index: usize,
    label: &str,
    active: bool,
) {
    parent
        .spawn((
            Button,
            VariantButton { tab, index },
            Node {
                height: Val::Px(24.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(if active { RADIO_ACTIVE } else { RADIO_NORMAL }),
            BorderColor::all(if active {
                palette::SKY
            } else {
                palette::STEEL
            }),
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
    for (interaction, cell) in &query {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            config.toggle_veto(slot, cell.0);
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
            && let Some(slot) = state.active_tab.color_slot()
            && slot.has_secondary()
        {
            state.secondary_colors.insert(slot, cell.0);
            if let Some(&primary_idx) = state.primary_colors.get(&slot) {
                config.set_complementary(slot, primary_idx, cell.0);
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
            config.clear_complementary(slot, primary_idx);
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

// ── Preview update ──────────────────────────────────────────────────────────

pub fn update_preview(
    mut state: ResMut<PortraitEditorState>,
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

    let descriptor = state.build_descriptor();
    let bg = PALETTE_COLORS[state.primary_colors[&PortraitColorSlot::Skin]].1;
    let image = rasterize_portrait(&descriptor, bg, 128, &parts);
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
    mut commands: Commands,
    variant_panel: Query<Entity, With<VariantPanel>>,
) {
    if !state.is_changed() {
        return;
    }
    for entity in &variant_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            spawn_variant_buttons(parent, &state);
        });
    }
}

pub fn rebuild_primary_grid(
    state: Res<PortraitEditorState>,
    config: Res<PortraitPaletteConfig>,
    mut commands: Commands,
    grid_panel: Query<Entity, With<PrimaryGridPanel>>,
) {
    if !state.is_changed() && !config.is_changed() {
        return;
    }
    for entity in &grid_panel {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            let slot = state.active_tab.color_slot();
            let selected = slot.and_then(|s| state.primary_colors.get(&s).copied());
            spawn_primary_color_grid(parent, &config, slot, selected);
        });
    }
}

fn spawn_primary_color_grid(
    parent: &mut ChildSpawnerCommands,
    config: &PortraitPaletteConfig,
    slot: Option<PortraitColorSlot>,
    selected: Option<usize>,
) {
    for (i, (_, rgb)) in PALETTE_COLORS.iter().enumerate() {
        let is_selected = selected == Some(i);
        let is_vetoed = slot.is_some_and(|s| config.is_vetoed(s, i));

        parent
            .spawn((
                Button,
                PrimaryColorCell(i),
                Node {
                    width: Val::Px(COLOR_CELL_SIZE),
                    height: Val::Px(COLOR_CELL_SIZE),
                    border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(rgb[0], rgb[1], rgb[2])),
                BorderColor::all(if is_selected {
                    SELECTED_BORDER
                } else {
                    Color::srgba(0.0, 0.0, 0.0, 0.3)
                }),
            ))
            .with_children(|cell| {
                if is_vetoed {
                    cell.spawn((
                        VetoOverlay,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        BackgroundColor(VETO_OVERLAY),
                    ));
                }
            });
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
    let has_secondary = state
        .active_tab
        .color_slot()
        .is_some_and(|s| s.has_secondary());
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

pub fn update_color_name_on_hover(
    primary_query: Query<(&Interaction, &PrimaryColorCell)>,
    secondary_query: Query<(&Interaction, &SecondaryColorCell)>,
    mut label_query: Query<&mut Text, With<ColorNameLabel>>,
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
