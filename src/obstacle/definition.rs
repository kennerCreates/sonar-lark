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
}

fn default_forward() -> Vec3 {
    Vec3::NEG_Z
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObstacleDef {
    pub id: ObstacleId,
    pub glb_node_name: String,
    pub trigger_volume: Option<TriggerVolumeConfig>,
    pub is_gate: bool,
    #[serde(default)]
    pub model_offset: Vec3,
}
