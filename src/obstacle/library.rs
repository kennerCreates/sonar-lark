use std::collections::HashMap;

use bevy::prelude::*;

use super::definition::{ObstacleDef, ObstacleId};

#[derive(Resource, Default)]
pub struct ObstacleLibrary {
    pub definitions: HashMap<ObstacleId, ObstacleDef>,
}
