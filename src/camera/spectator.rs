use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};

use super::orbit::{orbit_to_transform, MainCamera, OrbitState};
use super::settings::CameraSettings;

/// Orbit state for the spectator camera during race.
/// Initialized from the camera's current position when entering spectator mode.
#[derive(Resource)]
pub struct SpectatorOrbitState {
    pub center: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub initialized: bool,
}

impl Default for SpectatorOrbitState {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.7,
            distance: 40.0,
            initialized: false,
        }
    }
}

const PITCH_MIN: f32 = 0.15; // ~9 degrees — allow flatter view than editor
const PITCH_MAX: f32 = 1.40; // ~80 degrees

/// RTS-style orbit camera for spectator mode during race.
/// Middle-mouse orbit, scroll zoom, WASD ground-plane movement.
pub fn spectator_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut orbit: ResMut<SpectatorOrbitState>,
    mut camera: Query<(&mut Transform, &GlobalTransform), With<MainCamera>>,
) {
    let Ok((mut cam_tf, cam_gt)) = camera.single_mut() else {
        return;
    };

    // Initialize orbit state from current camera position on first use
    if !orbit.initialized {
        let pos = cam_tf.translation;
        orbit.distance = pos.length().max(5.0);
        orbit.pitch = (pos.y / orbit.distance).asin().clamp(PITCH_MIN, PITCH_MAX);
        orbit.yaw = pos.x.atan2(pos.z);
        orbit.center = Vec3::ZERO;
        orbit.initialized = true;
    }

    let dt = time.delta_secs();
    let mut orbit_changed = false;

    // Middle mouse drag: orbit
    if mouse_buttons.pressed(MouseButton::Middle) {
        let delta = mouse_motion.delta;
        if delta != Vec2::ZERO {
            orbit.yaw -= delta.x * settings.sensitivity;
            orbit.pitch =
                (orbit.pitch - delta.y * settings.sensitivity).clamp(PITCH_MIN, PITCH_MAX);
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

    // Compute camera-relative ground directions for WASD
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
        orbit.center += movement.normalize() * settings.move_speed * dt;
        orbit_changed = true;
    }

    if orbit_changed {
        let local = orbit_to_transform(&OrbitState {
            yaw: orbit.yaw,
            pitch: orbit.pitch,
            distance: orbit.distance,
        });
        cam_tf.translation = orbit.center + local.translation;
        cam_tf.rotation = local.rotation;
    }
}
