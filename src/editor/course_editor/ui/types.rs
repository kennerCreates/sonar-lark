use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::editor::course_editor::TransformMode;
use crate::obstacle::definition::ObstacleId;
use crate::palette;
use crate::rendering::CelMaterial;

pub const PANEL_BG: Color = palette::SMOKY_BLACK;
pub const BUTTON_NORMAL: Color = palette::INDIGO;
pub const BUTTON_HOVERED: Color = palette::SAPPHIRE;
pub const BUTTON_PRESSED: Color = palette::GREEN;
pub const BUTTON_SELECTED: Color = palette::TEAL;
pub const TOGGLE_ON: Color = palette::FROG;
pub const TOGGLE_OFF: Color = palette::BURGUNDY;

// --- Marker components ---

#[derive(Component)]
pub struct PaletteButton(pub ObstacleId);

#[derive(Component)]
pub struct ExistingCourseButton(pub String);

#[derive(Component)]
pub struct BackToWorkshopButton;

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
pub struct CourseNameField;

#[derive(Component)]
pub struct CourseNameDisplayText;

#[derive(Component)]
pub struct PaletteContainer;

#[derive(Component)]
pub struct ExistingCoursesContainer;

#[derive(Component)]
pub struct DeleteCourseButton(pub String);

#[derive(Component)]
pub struct ConfirmDeleteYesButton;

#[derive(Component)]
pub struct ConfirmDeleteCancelButton;

#[derive(Resource)]
pub struct PendingCourseDelete {
    pub path: String,
    pub display_name: String,
}

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

pub const COLOR_CYCLE: &[(&str, Option<[f32; 4]>)] = &[
    ("Auto", None),
    ("Gold", Some([1.0, 0.725, 0.220, 1.0])),
    ("Red", Some([0.961, 0.192, 0.255, 1.0])),
    ("Blue", Some([0.090, 0.576, 0.902, 1.0])),
    ("Green", Some([0.090, 0.612, 0.263, 1.0])),
    ("Purple", Some([0.792, 0.494, 0.949, 1.0])),
    ("White", Some([0.949, 0.949, 0.855, 1.0])),
];

pub struct CourseEntry {
    pub display_name: String,
    pub path: String,
}
