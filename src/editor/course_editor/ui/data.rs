use bevy::prelude::*;

use crate::course::data::{CameraInstance, CourseData, ObstacleInstance, PropInstance};
use crate::editor::course_editor::{PlacedCamera, PlacedObstacle, PlacedProp};

/// Build a `CourseData` from placed obstacle and prop data.
/// Pure function — no ECS dependencies.
pub fn build_course_data<'a>(
    name: String,
    obstacles: impl IntoIterator<Item = (&'a PlacedObstacle, &'a Transform)>,
    props: impl IntoIterator<Item = (&'a PlacedProp, &'a Transform)>,
    cameras: impl IntoIterator<Item = (&'a PlacedCamera, &'a Transform)>,
) -> CourseData {
    let instances = obstacles
        .into_iter()
        .map(|(placed, transform)| ObstacleInstance {
            obstacle_id: placed.obstacle_id.clone(),
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
            gate_order: placed.gate_order,
            gate_forward_flipped: placed.gate_forward_flipped,
        })
        .collect();

    let props = props
        .into_iter()
        .map(|(prop, transform)| PropInstance {
            kind: prop.kind,
            translation: transform.translation,
            rotation: transform.rotation,
            color_override: prop.color_override,
        })
        .collect();

    let cameras = cameras
        .into_iter()
        .map(|(cam, transform)| CameraInstance {
            translation: transform.translation,
            rotation: transform.rotation,
            is_primary: cam.is_primary,
            label: cam.label.clone(),
        })
        .collect();

    CourseData {
        name,
        instances,
        props,
        cameras,
    }
}

/// Compute the next gate order value from a course's obstacle instances.
/// Returns one past the maximum existing gate order, or 0 if none.
pub fn next_gate_order_from_instances(instances: &[ObstacleInstance]) -> u32 {
    instances
        .iter()
        .filter_map(|i| i.gate_order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::data::PropKind;
    use crate::obstacle::definition::ObstacleId;
    use bevy::math::{Quat, Vec3};

    // --- build_course_data ---

    #[test]
    fn build_course_data_empty() {
        let course = build_course_data(
            "empty".to_string(),
            Vec::<(&PlacedObstacle, &Transform)>::new(),
            Vec::<(&PlacedProp, &Transform)>::new(),
            Vec::<(&PlacedCamera, &Transform)>::new(),
        );
        assert_eq!(course.name, "empty");
        assert!(course.instances.is_empty());
        assert!(course.props.is_empty());
    }

    #[test]
    fn build_course_data_maps_obstacle_fields() {
        let placed = PlacedObstacle {
            obstacle_id: ObstacleId("gate_air".to_string()),
            gate_order: Some(2),
            gate_forward_flipped: true,
        };
        let transform = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(1.0),
            scale: Vec3::new(0.5, 1.0, 1.5),
        };

        let course = build_course_data(
            "test".to_string(),
            vec![(&placed, &transform)],
            Vec::<(&PlacedProp, &Transform)>::new(),
            Vec::<(&PlacedCamera, &Transform)>::new(),
        );

        assert_eq!(course.instances.len(), 1);
        let inst = &course.instances[0];
        assert_eq!(inst.obstacle_id.0, "gate_air");
        assert_eq!(inst.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(inst.scale, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(inst.gate_order, Some(2));
        assert!(inst.gate_forward_flipped);
    }

    #[test]
    fn build_course_data_maps_prop_fields() {
        let prop = PlacedProp {
            kind: PropKind::ConfettiEmitter,
            color_override: Some([1.0, 0.0, 0.0, 1.0]),
        };
        let transform = Transform::from_translation(Vec3::new(5.0, 0.0, -10.0));

        let course = build_course_data(
            "props_test".to_string(),
            Vec::<(&PlacedObstacle, &Transform)>::new(),
            vec![(&prop, &transform)],
            Vec::<(&PlacedCamera, &Transform)>::new(),
        );

        assert_eq!(course.props.len(), 1);
        let p = &course.props[0];
        assert_eq!(p.kind, PropKind::ConfettiEmitter);
        assert_eq!(p.translation, Vec3::new(5.0, 0.0, -10.0));
        assert_eq!(p.color_override, Some([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn build_course_data_multiple_obstacles_and_props() {
        let obs1 = PlacedObstacle {
            obstacle_id: ObstacleId("gate1".to_string()),
            gate_order: Some(0),
            gate_forward_flipped: false,
        };
        let obs2 = PlacedObstacle {
            obstacle_id: ObstacleId("wall".to_string()),
            gate_order: None,
            gate_forward_flipped: false,
        };
        let t1 = Transform::from_translation(Vec3::ZERO);
        let t2 = Transform::from_translation(Vec3::X);

        let prop = PlacedProp {
            kind: PropKind::ShellBurstEmitter,
            color_override: None,
        };
        let tp = Transform::from_translation(Vec3::Y);

        let course = build_course_data(
            "mixed".to_string(),
            vec![(&obs1, &t1), (&obs2, &t2)],
            vec![(&prop, &tp)],
            Vec::<(&PlacedCamera, &Transform)>::new(),
        );

        assert_eq!(course.instances.len(), 2);
        assert_eq!(course.props.len(), 1);
        assert_eq!(course.instances[0].obstacle_id.0, "gate1");
        assert_eq!(course.instances[1].obstacle_id.0, "wall");
        assert_eq!(course.props[0].kind, PropKind::ShellBurstEmitter);
        assert!(course.props[0].color_override.is_none());
    }

    // --- next_gate_order_from_instances ---

    #[test]
    fn next_gate_order_empty() {
        assert_eq!(next_gate_order_from_instances(&[]), 0);
    }

    #[test]
    fn next_gate_order_no_gates() {
        let instances = vec![ObstacleInstance {
            obstacle_id: ObstacleId("wall".to_string()),
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gate_order: None,
            gate_forward_flipped: false,
        }];
        assert_eq!(next_gate_order_from_instances(&instances), 0);
    }

    #[test]
    fn next_gate_order_sequential() {
        let instances = vec![
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(0),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(1),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(2),
                gate_forward_flipped: false,
            },
        ];
        assert_eq!(next_gate_order_from_instances(&instances), 3);
    }

    #[test]
    fn next_gate_order_with_gaps() {
        let instances = vec![
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(0),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("g".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(5),
                gate_forward_flipped: false,
            },
        ];
        assert_eq!(next_gate_order_from_instances(&instances), 6);
    }

    #[test]
    fn next_gate_order_mixed_gates_and_walls() {
        let instances = vec![
            ObstacleInstance {
                obstacle_id: ObstacleId("gate".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(3),
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("wall".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: None,
                gate_forward_flipped: false,
            },
            ObstacleInstance {
                obstacle_id: ObstacleId("gate".to_string()),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                gate_order: Some(1),
                gate_forward_flipped: false,
            },
        ];
        assert_eq!(next_gate_order_from_instances(&instances), 4);
    }
}
