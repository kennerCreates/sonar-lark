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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obstacle::definition::TriggerVolumeConfig;
    use bevy::math::Vec3;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_def(id: &str, node: &str, gate: bool) -> ObstacleDef {
        ObstacleDef {
            id: ObstacleId(id.to_string()),
            glb_node_name: node.to_string(),
            trigger_volume: if gate {
                Some(TriggerVolumeConfig {
                    offset: Vec3::new(0.0, 1.0, 0.0),
                    half_extents: Vec3::new(2.0, 2.0, 0.5),
                    forward: Vec3::NEG_Z,
                })
            } else {
                None
            },
            is_gate: gate,
            model_offset: Vec3::ZERO,
        }
    }

    #[test]
    fn insert_and_get() {
        let mut lib = ObstacleLibrary::default();
        lib.insert(sample_def("gate_air", "gate_air", true));

        let id = ObstacleId("gate_air".to_string());
        let fetched = lib.get(&id).unwrap();
        assert_eq!(fetched.id, id);
        assert_eq!(fetched.glb_node_name, "gate_air");
        assert!(fetched.is_gate);
        assert!(fetched.trigger_volume.is_some());
    }

    #[test]
    fn get_missing_returns_none() {
        let lib = ObstacleLibrary::default();
        assert!(lib.get(&ObstacleId("nonexistent".to_string())).is_none());
    }

    #[test]
    fn insert_overwrites_same_id() {
        let mut lib = ObstacleLibrary::default();
        lib.insert(sample_def("wall", "wall_mesh", false));
        lib.insert(sample_def("wall", "wall_mesh_v2", false));

        let id = ObstacleId("wall".to_string());
        assert_eq!(lib.get(&id).unwrap().glb_node_name, "wall_mesh_v2");
        assert_eq!(lib.definitions.len(), 1);
    }

    #[test]
    fn save_load_roundtrip() {
        let mut lib = ObstacleLibrary::default();
        lib.insert(sample_def("gate_air", "gate_air", true));
        lib.insert(sample_def("wall_short", "wall_short", false));

        let tmp = NamedTempFile::new().unwrap();
        lib.save_to_file(tmp.path()).unwrap();
        let loaded = ObstacleLibrary::load_from_file(tmp.path()).unwrap();

        assert_eq!(loaded.definitions.len(), 2);

        let gate = loaded.get(&ObstacleId("gate_air".to_string())).unwrap();
        assert!(gate.is_gate);
        let tv = gate.trigger_volume.as_ref().unwrap();
        assert_eq!(tv.offset, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(tv.half_extents, Vec3::new(2.0, 2.0, 0.5));

        let wall = loaded.get(&ObstacleId("wall_short".to_string())).unwrap();
        assert!(!wall.is_gate);
        assert!(wall.trigger_volume.is_none());
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("lib.ron");

        let mut lib = ObstacleLibrary::default();
        lib.insert(sample_def("test", "test_mesh", false));
        lib.save_to_file(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn load_missing_file_returns_error() {
        let result = ObstacleLibrary::load_from_file(Path::new("nonexistent_path.ron"));
        assert!(result.is_err());
    }

    #[test]
    fn load_invalid_ron_returns_error() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "this is not valid RON").unwrap();
        assert!(ObstacleLibrary::load_from_file(tmp.path()).is_err());
    }

    #[test]
    fn empty_library_roundtrip() {
        let lib = ObstacleLibrary::default();
        let tmp = NamedTempFile::new().unwrap();

        lib.save_to_file(tmp.path()).unwrap();
        let loaded = ObstacleLibrary::load_from_file(tmp.path()).unwrap();
        assert!(loaded.definitions.is_empty());
    }

    #[test]
    fn load_existing_ron_format() {
        let ron_content = r#"[
    (
        id: ("gate_air"),
        glb_node_name: "gate_air",
        trigger_volume: Some((
            offset: (0.0, 1.0, 0.0),
            half_extents: (2.0, 2.0, 0.5),
        )),
        is_gate: true,
    ),
]"#;
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{ron_content}").unwrap();

        let loaded = ObstacleLibrary::load_from_file(tmp.path()).unwrap();
        assert_eq!(loaded.definitions.len(), 1);
        let gate = loaded.get(&ObstacleId("gate_air".to_string())).unwrap();
        assert_eq!(gate.glb_node_name, "gate_air");
        assert!(gate.is_gate);
    }
}
