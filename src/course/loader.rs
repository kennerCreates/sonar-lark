use std::fs;
use std::path::Path;

use bevy::prelude::*;

use crate::drone::fireworks::FireworkEmitter;
use crate::obstacle::library::ObstacleLibrary;
use crate::obstacle::spawning::{spawn_obstacle, ObstacleMarker, ObstaclesGltfHandle};
use crate::rendering::{CelLightDir, CelMaterial};
use crate::states::AppState;
use super::data::CourseData;

#[derive(Resource)]
pub struct SelectedCourse {
    pub path: String,
}

/// Marker inserted after obstacles have been spawned, prevents re-running.
#[derive(Resource)]
pub struct CourseSpawned;

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

    match load_course_from_file(path) {
        Ok(course) => {
            info!("Loaded course '{}' with {} obstacles", course.name, course.instances.len());
            commands.insert_resource(course);
        }
        Err(e) => error!("{e}"),
    }
}

pub fn spawn_course(
    mut commands: Commands,
    course: Option<Res<CourseData>>,
    library: Res<ObstacleLibrary>,
    gltf_handle: Option<Res<ObstaclesGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    node_assets: Res<Assets<bevy::gltf::GltfNode>>,
    mesh_assets: Res<Assets<bevy::gltf::GltfMesh>>,
    mut cel_materials: ResMut<Assets<CelMaterial>>,
    std_materials: Res<Assets<StandardMaterial>>,
    light_dir: Res<CelLightDir>,
    already_spawned: Option<Res<CourseSpawned>>,
) {
    if already_spawned.is_some() {
        return;
    }
    let Some(course) = course else { return };
    let Some(gltf_handle) = gltf_handle else { return };
    // Poll until the glTF asset is actually loaded (async)
    if gltf_assets.get(&gltf_handle.0).is_none() {
        return;
    }

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
            &node_assets,
            &mesh_assets,
            &mut cel_materials,
            &std_materials,
            light_dir.0,
            &gltf_handle,
            &def.id,
            &def.glb_node_name,
            transform,
            def.model_offset,
            def.trigger_volume.as_ref(),
            instance.gate_order,
            instance.gate_forward_flipped,
        );

        if spawned.is_none() {
            warn!(
                "Failed to spawn obstacle '{}' (node '{}')",
                instance.obstacle_id.0, def.glb_node_name
            );
        }
    }

    // Spawn firework emitter markers from course props
    for prop in &course.props {
        let color_override = prop.color_override.map(|rgba| Color::srgb(rgba[0], rgba[1], rgba[2]));
        let transform =
            Transform::from_translation(prop.translation).with_rotation(prop.rotation);
        commands.spawn((
            transform,
            FireworkEmitter {
                kind: prop.kind,
                color_override,
            },
            DespawnOnExit(AppState::Race),
        ));
    }

    commands.insert_resource(CourseSpawned);
}

pub fn cleanup_course_spawned(mut commands: Commands) {
    commands.remove_resource::<CourseSpawned>();
}

/// Despawns all course obstacle entities. Used when leaving Race for a non-Results
/// state (e.g. Editor), since obstacles use `DespawnOnExit(Results)` and would
/// otherwise leak.
pub fn despawn_course_obstacles(
    mut commands: Commands,
    obstacles: Query<Entity, With<ObstacleMarker>>,
) {
    for entity in &obstacles {
        commands.entity(entity).despawn();
    }
}

pub fn load_course_from_file(path: &Path) -> Result<CourseData, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    ron::from_str(&contents)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))
}

pub fn delete_course(path: &Path) -> Result<(), String> {
    fs::remove_file(path)
        .map_err(|e| format!("Failed to delete {}: {e}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obstacle::definition::ObstacleId;
    use crate::course::data::ObstacleInstance;
    use bevy::math::{Quat, Vec3};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_course() -> CourseData {
        CourseData {
            name: "Test Circuit".to_string(),
            instances: vec![
                ObstacleInstance {
                    obstacle_id: ObstacleId("gate_air".to_string()),
                    translation: Vec3::new(0.0, 0.0, -20.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                    gate_order: Some(0),
                    gate_forward_flipped: false,
                },
                ObstacleInstance {
                    obstacle_id: ObstacleId("gate_air".to_string()),
                    translation: Vec3::new(10.0, 2.0, -40.0),
                    rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
                    scale: Vec3::splat(1.5),
                    gate_order: Some(1),
                    gate_forward_flipped: false,
                },
                ObstacleInstance {
                    obstacle_id: ObstacleId("wall_short".to_string()),
                    translation: Vec3::new(5.0, 0.0, -30.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                    gate_order: None,
                    gate_forward_flipped: false,
                },
            ],
            props: vec![],
        }
    }

    #[test]
    fn save_load_roundtrip() {
        let course = sample_course();
        let tmp = NamedTempFile::new().unwrap();

        save_course(&course, tmp.path()).unwrap();
        let loaded = load_course_from_file(tmp.path()).unwrap();

        assert_eq!(loaded.name, "Test Circuit");
        assert_eq!(loaded.instances.len(), 3);

        assert_eq!(loaded.instances[0].obstacle_id.0, "gate_air");
        assert_eq!(loaded.instances[0].gate_order, Some(0));
        assert_eq!(loaded.instances[0].translation, Vec3::new(0.0, 0.0, -20.0));

        assert_eq!(loaded.instances[2].obstacle_id.0, "wall_short");
        assert_eq!(loaded.instances[2].gate_order, None);
    }

    #[test]
    fn empty_course_roundtrip() {
        let course = CourseData {
            name: "Empty".to_string(),
            instances: vec![],
            props: vec![],
        };
        let tmp = NamedTempFile::new().unwrap();

        save_course(&course, tmp.path()).unwrap();
        let loaded = load_course_from_file(tmp.path()).unwrap();

        assert_eq!(loaded.name, "Empty");
        assert!(loaded.instances.is_empty());
    }

    #[test]
    fn load_existing_course_format() {
        let ron_content = r#"CourseData(
    name: "Example Circuit",
    instances: [],
)"#;
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{ron_content}").unwrap();

        let loaded = load_course_from_file(tmp.path()).unwrap();
        assert_eq!(loaded.name, "Example Circuit");
        assert!(loaded.instances.is_empty());
    }

    #[test]
    fn load_missing_file_returns_error() {
        assert!(load_course_from_file(Path::new("no_such_course.ron")).is_err());
    }

    #[test]
    fn load_invalid_ron_returns_error() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "not valid ron {{}}").unwrap();
        assert!(load_course_from_file(tmp.path()).is_err());
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("course.ron");

        let course = CourseData {
            name: "Nested".to_string(),
            instances: vec![],
            props: vec![],
        };
        save_course(&course, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn transform_values_preserved() {
        let rotation = Quat::from_rotation_y(1.234);
        let course = CourseData {
            name: "Transform Test".to_string(),
            instances: vec![ObstacleInstance {
                obstacle_id: ObstacleId("test".to_string()),
                translation: Vec3::new(1.5, 2.5, 3.5),
                rotation,
                scale: Vec3::new(0.5, 1.0, 2.0),
                gate_order: Some(7),
                gate_forward_flipped: false,
            }],
            props: vec![],
        };
        let tmp = NamedTempFile::new().unwrap();

        save_course(&course, tmp.path()).unwrap();
        let loaded = load_course_from_file(tmp.path()).unwrap();
        let inst = &loaded.instances[0];

        assert_eq!(inst.translation, Vec3::new(1.5, 2.5, 3.5));
        assert_eq!(inst.scale, Vec3::new(0.5, 1.0, 2.0));
        assert_eq!(inst.gate_order, Some(7));
        // Quaternion comparison with tolerance for float serialization
        let diff = (inst.rotation.dot(rotation)).abs();
        assert!(diff > 0.999, "rotation not preserved: dot product = {diff}");
    }

    #[test]
    fn delete_course_removes_file() {
        let tmp = NamedTempFile::new().unwrap();
        let (_, persisted_path) = tmp.keep().unwrap();
        assert!(persisted_path.exists());

        delete_course(&persisted_path).unwrap();
        assert!(!persisted_path.exists());
    }

    #[test]
    fn delete_nonexistent_course_returns_error() {
        assert!(delete_course(Path::new("no_such_course.ron")).is_err());
    }
}
