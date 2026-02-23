use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::obstacle::definition::ObstacleId;

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
}

#[derive(Clone, Debug, Serialize, Deserialize, Resource)]
pub struct CourseData {
    pub name: String,
    pub instances: Vec<ObstacleInstance>,
}
