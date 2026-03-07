use std::collections::HashMap;

use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::editor::course_editor::TransformMode;
use crate::obstacle::definition::ObstacleId;
use crate::rendering::CelMaterial;

// --- Marker components ---

#[derive(Component)]
pub struct PaletteButton(pub ObstacleId);

#[derive(Component)]
pub struct BackToMenuButton;

#[derive(Component)]
pub struct SaveCourseButton;

#[derive(Component)]
pub struct GateOrderModeButton;

#[derive(Component)]
pub struct GateOrderModeText;

#[derive(Component)]
pub struct ClearGateOrdersButton;

#[derive(Component)]
pub struct GateCountText;

#[derive(Component)]
pub struct PaletteContainer;

#[derive(Component)]
pub struct InventoryContainer;

#[derive(Component)]
pub struct InventoryPaletteButton(pub ObstacleId);

#[derive(Component)]
pub struct TransformModeButton(pub TransformMode);

#[derive(Component)]
pub struct ObstacleTabButton;

#[derive(Component)]
pub struct PropsTabButton;

#[derive(Component)]
pub struct ObstaclePaletteContent;

#[derive(Component)]
pub struct PropPaletteContent;

#[derive(Component)]
pub struct PropPaletteButton(pub PropKind);

#[derive(Component)]
pub struct PropColorButton;

#[derive(Component)]
pub struct PropColorLabel;

#[derive(Resource)]
pub struct PropEditorMeshes {
    pub confetti_mesh: Handle<Mesh>,
    pub shell_mesh: Handle<Mesh>,
    pub confetti_material: Handle<CelMaterial>,
    pub shell_material: Handle<CelMaterial>,
}

#[derive(Resource)]
pub struct CameraEditorMeshes {
    pub mesh: Handle<Mesh>,
    pub material: Handle<CelMaterial>,
    pub primary_material: Handle<CelMaterial>,
}

// --- Thumbnail camera ---

#[derive(Component)]
pub struct ThumbnailCamera;

#[derive(Resource)]
pub struct ThumbnailRenderTarget {
    pub image_handle: Handle<Image>,
    pub camera_entity: Entity,
}

// --- Gate color picker ---

#[derive(Component)]
pub struct GateColorLabel;

#[derive(Component)]
pub struct GateColorCell(pub usize);

#[derive(Component)]
pub struct GateColorDefaultButton;

#[derive(Component)]
pub struct StartRaceButton;

#[derive(Component)]
pub struct MoneyText;

pub const DEFAULT_GATE_COLOR: [f32; 4] = [0.949, 0.949, 0.855, 1.0];

pub const GATE_COLOR_CELL_SIZE: f32 = 24.0;
pub const GATE_COLOR_GRID_COLS: usize = 8;

// --- Obstacle thumbnails ---

#[derive(Resource, Default)]
pub struct ObstacleThumbnails {
    pub images: HashMap<ObstacleId, Handle<Image>>,
}

// --- Prop color cycle ---

pub const COLOR_CYCLE: &[(&str, Option<[f32; 4]>)] = &[
    ("Auto", None),
    ("Gold", Some([1.0, 0.725, 0.220, 1.0])),
    ("Red", Some([0.961, 0.192, 0.255, 1.0])),
    ("Blue", Some([0.090, 0.576, 0.902, 1.0])),
    ("Green", Some([0.090, 0.612, 0.263, 1.0])),
    ("Purple", Some([0.792, 0.494, 0.949, 1.0])),
    ("White", Some([0.949, 0.949, 0.855, 1.0])),
];
