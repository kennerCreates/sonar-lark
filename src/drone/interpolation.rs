use bevy::prelude::*;
use bevy::time::Fixed;

use super::components::Drone;

/// Stores the drone's translation from the previous FixedUpdate tick.
/// Camera systems interpolate between this and the current `Transform` using
/// `Time<Fixed>::overstep_fraction()` for smooth rendering between physics steps.
#[derive(Component)]
pub struct PreviousTranslation(pub Vec3);

/// Stores the drone's rotation from the previous FixedUpdate tick.
#[derive(Component)]
pub struct PreviousRotation(pub Quat);

/// Authoritative physics translation, saved after each FixedUpdate tick.
/// Visual interpolation blends between `PreviousTranslation` and this value.
#[derive(Component)]
pub struct PhysicsTranslation(pub Vec3);

/// Authoritative physics rotation, saved after each FixedUpdate tick.
#[derive(Component)]
pub struct PhysicsRotation(pub Quat);

/// Restores the authoritative physics transform before each fixed tick.
/// Undoes the visual interpolation applied in PostUpdate so that physics
/// and `save_previous_transforms` see the real physics state.
/// Runs in `FixedFirst`.
pub fn restore_physics_transforms(
    mut query: Query<
        (&mut Transform, &PhysicsTranslation, &PhysicsRotation),
        With<Drone>,
    >,
) {
    for (mut tf, phys_pos, phys_rot) in &mut query {
        tf.translation = phys_pos.0;
        tf.rotation = phys_rot.0;
    }
}

/// Snapshots current transform into Previous* components at the start of each
/// fixed tick. Runs in `FixedPreUpdate` so it captures state before the physics
/// chain modifies it.
pub fn save_previous_transforms(
    mut query: Query<
        (&Transform, &mut PreviousTranslation, &mut PreviousRotation),
        With<Drone>,
    >,
) {
    for (tf, mut prev_pos, mut prev_rot) in &mut query {
        prev_pos.0 = tf.translation;
        prev_rot.0 = tf.rotation;
    }
}

/// Saves the authoritative physics state after each FixedUpdate tick.
/// Runs in `FixedPostUpdate`.
pub fn save_physics_transforms(
    mut query: Query<
        (&Transform, &mut PhysicsTranslation, &mut PhysicsRotation),
        With<Drone>,
    >,
) {
    for (tf, mut phys_pos, mut phys_rot) in &mut query {
        phys_pos.0 = tf.translation;
        phys_rot.0 = tf.rotation;
    }
}

/// Interpolates drone transforms for smooth rendering between physics ticks.
/// Blends between the previous and current authoritative physics state using
/// `overstep_fraction`. Runs in `PostUpdate` (after camera systems, before render).
pub fn interpolate_visual_transforms(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<
        (
            &mut Transform,
            &PreviousTranslation,
            &PreviousRotation,
            &PhysicsTranslation,
            &PhysicsRotation,
        ),
        With<Drone>,
    >,
) {
    let alpha = fixed_time.overstep_fraction();
    for (mut tf, prev_pos, prev_rot, phys_pos, phys_rot) in &mut query {
        tf.translation = prev_pos.0.lerp(phys_pos.0, alpha);
        tf.rotation = prev_rot.0.slerp(phys_rot.0, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn lerp_translation_at_zero_returns_prev() {
        let prev = Vec3::new(1.0, 2.0, 3.0);
        let current = Vec3::new(4.0, 5.0, 6.0);
        let result = prev.lerp(current, 0.0);
        assert!((result - prev).length() < 1e-6);
    }

    #[test]
    fn lerp_translation_at_one_returns_current() {
        let prev = Vec3::new(1.0, 2.0, 3.0);
        let current = Vec3::new(4.0, 5.0, 6.0);
        let result = prev.lerp(current, 1.0);
        assert!((result - current).length() < 1e-6);
    }

    #[test]
    fn lerp_translation_at_half() {
        let prev = Vec3::ZERO;
        let current = Vec3::new(10.0, 0.0, 0.0);
        let result = prev.lerp(current, 0.5);
        assert!((result - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn slerp_rotation_at_zero_returns_prev() {
        let prev = Quat::IDENTITY;
        let current = Quat::from_rotation_y(FRAC_PI_2);
        let result = prev.slerp(current, 0.0);
        assert!(result.angle_between(prev) < 0.001);
    }

    #[test]
    fn slerp_rotation_at_one_returns_current() {
        let prev = Quat::IDENTITY;
        let current = Quat::from_rotation_y(FRAC_PI_2);
        let result = prev.slerp(current, 1.0);
        assert!(result.angle_between(current) < 0.001);
    }
}
