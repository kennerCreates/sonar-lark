use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::obstacle::definition::ObstacleId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateCamera {
    pub offset: Vec3,
    pub rotation: Quat,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObstacleInstance {
    pub obstacle_id: ObstacleId,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub gate_order: Option<u32>,
    /// When true, the gate's forward direction is flipped 180 degrees.
    #[serde(default)]
    pub gate_forward_flipped: bool,
    /// Optional camera attached to this gate, with local offset/rotation.
    #[serde(default)]
    pub camera: Option<GateCamera>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropKind {
    ConfettiEmitter,
    ShellBurstEmitter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropInstance {
    pub kind: PropKind,
    pub translation: Vec3,
    pub rotation: Quat,
    /// RGBA override color. None = use winner's drone color at race time.
    pub color_override: Option<[f32; 4]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraInstance {
    pub translation: Vec3,
    pub rotation: Quat,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Resource)]
pub struct CourseData {
    pub name: String,
    pub instances: Vec<ObstacleInstance>,
    #[serde(default)]
    pub props: Vec<PropInstance>,
    #[serde(default)]
    pub cameras: Vec<CameraInstance>,
}
