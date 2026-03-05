use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};

use super::settings::CameraSettings;
use crate::editor::workshop::WorkshopState;

#[derive(Component)]
pub struct CameraRig;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct OrbitState {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

const PITCH_MIN: f32 = 0.35; // ~20 degrees
const PITCH_MAX: f32 = 1.40; // ~80 degrees

const WORKSHOP_DEFAULT_YAW: f32 = 0.5;
const WORKSHOP_DEFAULT_PITCH: f32 = 0.6;
const WORKSHOP_DEFAULT_DISTANCE: f32 = 12.0;

const COURSE_DEFAULT_YAW: f32 = 0.0;
const COURSE_DEFAULT_PITCH: f32 = 0.7;
const COURSE_DEFAULT_DISTANCE: f32 = 40.0;

pub(crate) fn orbit_to_transform(orbit: &OrbitState) -> Transform {
    let x = orbit.distance * orbit.pitch.cos() * orbit.yaw.sin();
    let y = orbit.distance * orbit.pitch.sin();
    let z = orbit.distance * orbit.pitch.cos() * orbit.yaw.cos();
    Transform::from_translation(Vec3::new(x, y, z)).looking_at(Vec3::ZERO, Vec3::Y)
}

// --- Lifecycle systems ---

pub fn setup_editor_camera(
    mut commands: Commands,
    camera_query: Query<Entity, With<MainCamera>>,
    settings: Res<CameraSettings>,
) {
    let orbit = OrbitState {
        yaw: COURSE_DEFAULT_YAW,
        pitch: COURSE_DEFAULT_PITCH,
        distance: COURSE_DEFAULT_DISTANCE,
    };

    let rig_id = commands
        .spawn((CameraRig, orbit, Transform::default()))
        .id();

    if let Ok(camera_entity) = camera_query.single() {
        commands.entity(camera_entity).set_parent_in_place(rig_id);
    }

    // Apply FOV from settings
    if let Ok(camera_entity) = camera_query.single() {
        commands.entity(camera_entity).insert(Projection::Perspective(PerspectiveProjection {
            fov: settings.fov_degrees.to_radians(),
            ..default()
        }));
    }
}

pub fn teardown_editor_camera(
    mut commands: Commands,
    camera_query: Query<Entity, With<MainCamera>>,
    rig_query: Query<Entity, With<CameraRig>>,
) {
    // Unparent camera and reset its transform
    if let Ok(camera_entity) = camera_query.single() {
        commands.entity(camera_entity).remove_parent_in_place();
        commands.entity(camera_entity).insert(
            Transform::from_xyz(0.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        );
    }

    // Despawn rig
    for rig in &rig_query {
        commands.entity(rig).despawn();
    }
}

pub fn reset_rig_for_workshop(
    mut rig_query: Query<(&mut Transform, &mut OrbitState), With<CameraRig>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<CameraRig>)>,
) {
    let Ok((mut rig_tf, mut orbit)) = rig_query.single_mut() else {
        return;
    };

    rig_tf.translation = Vec3::ZERO;
    orbit.yaw = WORKSHOP_DEFAULT_YAW;
    orbit.pitch = WORKSHOP_DEFAULT_PITCH;
    orbit.distance = WORKSHOP_DEFAULT_DISTANCE;

    if let Ok(mut cam_tf) = camera_query.single_mut() {
        *cam_tf = orbit_to_transform(&orbit);
    }
}

pub fn reset_rig_for_course_editor(
    mut rig_query: Query<(&mut Transform, &mut OrbitState), With<CameraRig>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<CameraRig>)>,
) {
    let Ok((mut rig_tf, mut orbit)) = rig_query.single_mut() else {
        return;
    };

    rig_tf.translation = Vec3::ZERO;
    orbit.yaw = COURSE_DEFAULT_YAW;
    orbit.pitch = COURSE_DEFAULT_PITCH;
    orbit.distance = COURSE_DEFAULT_DISTANCE;

    if let Ok(mut cam_tf) = camera_query.single_mut() {
        *cam_tf = orbit_to_transform(&orbit);
    }
}

// --- RTS Camera (Course Editor) ---

pub fn rts_camera_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    windows: Query<&Window>,
    settings: Res<CameraSettings>,
    workshop_state: Option<Res<WorkshopState>>,
    mut rig_query: Query<(&mut Transform, &mut OrbitState), With<CameraRig>>,
    mut camera_query: Query<(&mut Transform, &GlobalTransform), (With<MainCamera>, Without<CameraRig>)>,
) {
    let Ok((mut rig_tf, mut orbit)) = rig_query.single_mut() else {
        return;
    };
    let Ok((mut cam_tf, cam_gt)) = camera_query.single_mut() else {
        return;
    };

    let editing_name = workshop_state
        .as_ref()
        .is_some_and(|s| s.editing_name);

    let dt = time.delta_secs();
    let mut orbit_changed = false;

    // Middle mouse drag: orbit
    if mouse_buttons.pressed(MouseButton::Middle) {
        let delta = mouse_motion.delta;
        if delta != Vec2::ZERO {
            orbit.yaw -= delta.x * settings.sensitivity;
            orbit.pitch = (orbit.pitch - delta.y * settings.sensitivity).clamp(PITCH_MIN, PITCH_MAX);
            orbit_changed = true;
        }
    }

    // Scroll wheel: zoom
    let scroll_y = mouse_scroll.delta.y;
    if scroll_y != 0.0 {
        orbit.distance = (orbit.distance - scroll_y * settings.zoom_speed)
            .clamp(settings.zoom_min, settings.zoom_max);
        orbit_changed = true;
    }

    if orbit_changed {
        *cam_tf = orbit_to_transform(&orbit);
    }

    // Skip movement inputs when editing text
    if editing_name {
        return;
    }

    // WASD: ground-plane rig movement
    let cam_forward = cam_gt.forward().as_vec3();
    let cam_right = cam_gt.right().as_vec3();
    let ground_forward = Vec3::new(cam_forward.x, 0.0, cam_forward.z).normalize_or_zero();
    let ground_right = Vec3::new(cam_right.x, 0.0, cam_right.z).normalize_or_zero();

    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        movement += ground_forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement -= ground_forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement -= ground_right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement += ground_right;
    }

    if movement != Vec3::ZERO {
        rig_tf.translation += movement.normalize() * settings.move_speed * dt;
    }

    // Edge scroll
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let w = window.width();
    let h = window.height();
    let margin = settings.edge_scroll_margin;

    let mut edge_dir = Vec2::ZERO;
    if cursor_pos.x < margin {
        edge_dir.x -= 1.0;
    }
    if cursor_pos.x > w - margin {
        edge_dir.x += 1.0;
    }
    // Screen Y: top=0, bottom=height. Top of screen = forward in world.
    if cursor_pos.y < margin {
        edge_dir.y += 1.0;
    }
    if cursor_pos.y > h - margin {
        edge_dir.y -= 1.0;
    }

    if edge_dir != Vec2::ZERO {
        let scroll_movement = (ground_right * edge_dir.x + ground_forward * edge_dir.y)
            * settings.edge_scroll_speed
            * dt;
        rig_tf.translation += scroll_movement;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    #[test]
    fn orbit_to_transform_at_zero_yaw() {
        let orbit = OrbitState {
            yaw: 0.0,
            pitch: FRAC_PI_4,
            distance: 10.0,
        };
        let tf = orbit_to_transform(&orbit);
        // At yaw=0: x=0, z=distance*cos(pitch), y=distance*sin(pitch)
        let expected_y = 10.0 * FRAC_PI_4.sin();
        let expected_z = 10.0 * FRAC_PI_4.cos();
        assert!((tf.translation.x).abs() < 0.001);
        assert!((tf.translation.y - expected_y).abs() < 0.001);
        assert!((tf.translation.z - expected_z).abs() < 0.001);
    }

    #[test]
    fn orbit_to_transform_looks_at_origin() {
        let orbit = OrbitState {
            yaw: 1.0,
            pitch: 0.5,
            distance: 20.0,
        };
        let tf = orbit_to_transform(&orbit);
        let forward = tf.forward().as_vec3();
        let to_origin = (Vec3::ZERO - tf.translation).normalize();
        let dot = forward.dot(to_origin);
        assert!(dot > 0.99, "Camera should face origin, dot={dot}");
    }

    #[test]
    fn orbit_distance_matches() {
        let orbit = OrbitState {
            yaw: 2.0,
            pitch: 1.0,
            distance: 15.0,
        };
        let tf = orbit_to_transform(&orbit);
        let dist = tf.translation.length();
        assert!((dist - 15.0).abs() < 0.01);
    }
}
