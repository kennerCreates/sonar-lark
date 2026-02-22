use std::fs;
use std::path::Path;

use bevy::prelude::*;

use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{spawn_obstacle, ObstaclesGltfHandle};
use super::data::CourseData;

#[derive(Resource)]
pub struct SelectedCourse {
    pub path: String,
}

pub fn load_course(mut commands: Commands, selected: Option<Res<SelectedCourse>>) {
    let Some(selected) = selected else {
        warn!("No course selected, cannot load");
        return;
    };

    let path = Path::new(&selected.path);
    if !path.exists() {
        warn!("Course file not found: {}", path.display());
        return;
    }

    match fs::read_to_string(path) {
        Ok(contents) => match ron::from_str::<CourseData>(&contents) {
            Ok(course) => {
                info!("Loaded course '{}' with {} obstacles", course.name, course.instances.len());
                commands.insert_resource(course);
            }
            Err(e) => error!("Failed to parse course {}: {e}", path.display()),
        },
        Err(e) => error!("Failed to read course {}: {e}", path.display()),
    }
}

pub fn spawn_course(
    mut commands: Commands,
    course: Option<Res<CourseData>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
) {
    let Some(course) = course else { return };
    let Some(gltf_handle) = gltf_handle else {
        warn!("Obstacles glTF not loaded yet");
        return;
    };

    for instance in &course.instances {
        let Some(def) = library.get(&instance.obstacle_id) else {
            warn!("Unknown obstacle '{}', skipping", instance.obstacle_id.0);
            continue;
        };

        let transform = Transform {
            translation: instance.translation,
            rotation: instance.rotation,
            scale: instance.scale,
        };

        let spawned = spawn_obstacle(
            &mut commands,
            &gltf_assets,
            &gltf_handle,
            &def.id,
            &def.glb_scene_name,
            transform,
            def.trigger_volume.as_ref(),
            instance.gate_order,
        );

        if spawned.is_none() {
            warn!(
                "Failed to spawn obstacle '{}' (scene '{}')",
                instance.obstacle_id.0, def.glb_scene_name
            );
        }
    }
}

pub fn save_course(course: &CourseData, path: &Path) -> Result<(), String> {
    let pretty = ron::ser::PrettyConfig::default();
    let contents = ron::ser::to_string_pretty(course, pretty)
        .map_err(|e| format!("Failed to serialize course: {e}"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }
    fs::write(path, contents)
        .map_err(|e| format!("Failed to write course to {}: {e}", path.display()))?;

    Ok(())
}
