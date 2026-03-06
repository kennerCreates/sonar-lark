use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObstacleId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerVolumeConfig {
    pub offset: Vec3,
    pub half_extents: Vec3,
    /// Local-space forward direction of the gate (the expected approach direction).
    #[serde(default = "default_forward")]
    pub forward: Vec3,
    #[serde(default = "default_rotation")]
    pub rotation: Quat,
}

fn default_forward() -> Vec3 {
    Vec3::NEG_Z
}

fn default_rotation() -> Quat {
    Quat::IDENTITY
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionVolumeConfig {
    pub offset: Vec3,
    pub half_extents: Vec3,
    #[serde(default = "default_rotation")]
    pub rotation: Quat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraPlacement {
    pub offset: Vec3,
    #[serde(default = "default_rotation")]
    pub rotation: Quat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObstacleDef {
    pub id: ObstacleId,
    pub glb_node_name: String,
    pub trigger_volume: Option<TriggerVolumeConfig>,
    pub is_gate: bool,
    /// Cost to place this gate in the course editor. Only meaningful when `is_gate` is true.
    #[serde(default)]
    pub gate_cost: u32,
    #[serde(default)]
    pub model_offset: Vec3,
    #[serde(default = "default_rotation")]
    pub model_rotation: Quat,
    #[serde(default)]
    pub collision_volumes: Vec<CollisionVolumeConfig>,
    #[serde(default)]
    pub default_camera: Option<CameraPlacement>,
}
