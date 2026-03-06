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
    /// RGBA override color. None = use default gate_color() behavior.
    #[serde(default)]
    pub color_override: Option<[f32; 4]>,
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

/// Cost to place a gate in the course editor. Returns 0 for non-gate obstacles.
/// Looks up the cost from the obstacle library definition.
pub fn gate_cost(obstacle_id: &str, library: &crate::obstacle::library::ObstacleLibrary) -> u32 {
    library
        .get(&crate::obstacle::definition::ObstacleId(obstacle_id.to_string()))
        .filter(|def| def.is_gate)
        .map(|def| def.gate_cost)
        .unwrap_or(0)
}

/// Spectacle weight for fan attraction. Higher = crowds like it more.
pub fn gate_spectacle_weight(obstacle_id: &str) -> f32 {
    match obstacle_id {
        "gate_ground" => 1.0,
        "gate_loop" => 2.0,
        "gate_air" => 4.0,
        _ => 0.0,
    }
}

fn default_location() -> String {
    "Abandoned Warehouse".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize, Resource)]
pub struct CourseData {
    pub name: String,
    pub instances: Vec<ObstacleInstance>,
    #[serde(default)]
    pub props: Vec<PropInstance>,
    #[serde(default)]
    pub cameras: Vec<CameraInstance>,
    #[serde(default = "default_location")]
    pub location: String,
}
