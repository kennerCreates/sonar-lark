use std::collections::HashMap;
use std::fs;
use std::path::Path;

use bevy::prelude::*;

use super::definition::{ObstacleDef, ObstacleId};

#[derive(Resource, Default)]
pub struct ObstacleLibrary {
    pub definitions: HashMap<ObstacleId, ObstacleDef>,
}

impl ObstacleLibrary {
    pub fn get(&self, id: &ObstacleId) -> Option<&ObstacleDef> {
        self.definitions.get(id)
    }

    pub fn insert(&mut self, def: ObstacleDef) {
        self.definitions.insert(def.id.clone(), def);
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let defs: Vec<ObstacleDef> =
            ron::from_str(&contents).map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

        let mut library = Self::default();
        for def in defs {
            library.insert(def);
        }
        Ok(library)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let defs: Vec<&ObstacleDef> = self.definitions.values().collect();
        let pretty = ron::ser::PrettyConfig::default();
        let contents = ron::ser::to_string_pretty(&defs, pretty)
            .map_err(|e| format!("Failed to serialize library: {e}"))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }
        fs::write(path, contents)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        Ok(())
    }
}

const LIBRARY_PATH: &str = "assets/library/default.obstacles.ron";

pub fn load_obstacle_library(mut library: ResMut<ObstacleLibrary>) {
    let path = Path::new(LIBRARY_PATH);
    if path.exists() {
        match ObstacleLibrary::load_from_file(path) {
            Ok(loaded) => {
                *library = loaded;
                info!("Loaded obstacle library with {} definitions", library.definitions.len());
            }
            Err(e) => warn!("Could not load obstacle library: {e}"),
        }
    } else {
        info!("No obstacle library found at {LIBRARY_PATH}, starting empty");
    }
}

pub fn save_obstacle_library(library: &ObstacleLibrary) {
    let path = Path::new(LIBRARY_PATH);
    match library.save_to_file(path) {
        Ok(()) => info!("Saved obstacle library to {LIBRARY_PATH}"),
        Err(e) => error!("Failed to save obstacle library: {e}"),
    }
}
