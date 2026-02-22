use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};

#[derive(Resource)]
pub struct SpectatorSettings {
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for SpectatorSettings {
    fn default() -> Self {
        Self {
            speed: 20.0,
            sensitivity: 0.003,
        }
    }
}

pub fn spectator_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut settings: ResMut<SpectatorSettings>,
    mut query: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Right-click drag for rotation
    if mouse_buttons.pressed(MouseButton::Right) {
        let delta = mouse_motion.delta;
        if delta != Vec2::ZERO {
            let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
            let new_yaw = yaw - delta.x * settings.sensitivity;
            let new_pitch = (pitch - delta.y * settings.sensitivity).clamp(-1.5, 1.5);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);
        }
    }

    // Scroll wheel for speed adjustment
    let scroll_y = mouse_scroll.delta.y;
    if scroll_y != 0.0 {
        settings.speed = (settings.speed * (1.0 + scroll_y * 0.1)).clamp(1.0, 200.0);
    }

    // WASD movement
    let speed = settings.speed;
    let dt = time.delta_secs();

    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();

    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        movement += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement += right;
    }
    if keyboard.pressed(KeyCode::Space) {
        movement += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        movement -= Vec3::Y;
    }

    if movement != Vec3::ZERO {
        transform.translation += movement.normalize() * speed * dt;
    }
}
