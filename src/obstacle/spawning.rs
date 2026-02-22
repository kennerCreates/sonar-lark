use bevy::prelude::*;

use super::definition::ObstacleId;

#[derive(Component)]
pub struct ObstacleMarker {
    pub id: ObstacleId,
}

#[derive(Component)]
pub struct TriggerVolume {
    pub half_extents: Vec3,
}
