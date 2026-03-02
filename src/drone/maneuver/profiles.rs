use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

use super::{ManeuverKind, ManeuverPhaseTag};

/// Hermite smoothstep: zero derivative at t=0 and t=1, preventing angular velocity spikes
/// at phase transitions.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Build a level (Y-up) quaternion facing the given direction (projected to XZ plane).
fn level_quat(forward: Vec3) -> Quat {
    let horizontal = Vec3::new(forward.x, 0.0, forward.z);
    let fwd = horizontal.normalize_or(Vec3::NEG_Z);
    let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
    let up = right.cross(fwd);
    Quat::from_mat3(&Mat3::from_cols(right, up, -fwd)).normalize()
}

/// Speed-based duration scaling. Faster entry = shorter phases.
/// Reference speed ~30 m/s. Returns multiplier in [0.6, 1.4].
fn speed_scale(entry_speed: f32) -> f32 {
    (30.0 / entry_speed.max(10.0)).clamp(0.6, 1.4)
}

/// Compute the target orientation for a maneuvering drone.
///
/// Returns a world-space quaternion representing the desired body orientation
/// at the given phase and progress within that phase. Uses smoothstep on
/// `progress` to ensure zero angular velocity at phase boundaries.
pub fn maneuver_target_orientation(
    kind: ManeuverKind,
    phase: ManeuverPhaseTag,
    progress: f32,
    entry_yaw_dir: Vec3,
    _entry_velocity: Vec3,
) -> Quat {
    let t = smoothstep(progress);

    match kind {
        ManeuverKind::SplitS => split_s_orientation(phase, t, entry_yaw_dir),
        ManeuverKind::PowerLoop => power_loop_orientation(phase, t, entry_yaw_dir),
        // AggressiveBank uses TiltOverride with the PID, not ActiveManeuver.
        ManeuverKind::AggressiveBank => level_quat(entry_yaw_dir),
    }
}

fn split_s_orientation(phase: ManeuverPhaseTag, t: f32, entry_yaw_dir: Vec3) -> Quat {
    let level_entry = level_quat(entry_yaw_dir);

    match phase {
        ManeuverPhaseTag::Entry => {
            // Roll 180° around the forward (local -Z) axis: level → inverted.
            // Local Z rotation flips body-up from Y to -Y.
            (level_entry * Quat::from_rotation_z(t * PI)).normalize()
        }
        ManeuverPhaseTag::Ballistic => {
            // From inverted, pitch 180° around local X ("pull through").
            // Nose sweeps downward, drone exits level facing the opposite direction.
            let inverted = level_entry * Quat::from_rotation_z(PI);
            (inverted * Quat::from_rotation_x(t * PI)).normalize()
        }
        ManeuverPhaseTag::Recovery => {
            // Hold level flight, heading reversed from entry.
            level_quat(-entry_yaw_dir)
        }
    }
}

fn power_loop_orientation(phase: ManeuverPhaseTag, t: f32, entry_yaw_dir: Vec3) -> Quat {
    let level_entry = level_quat(entry_yaw_dir);

    match phase {
        ManeuverPhaseTag::Entry => {
            // Gentle pitch-up to ~15° to initiate the loop without flinging upward.
            let pitch_up = Quat::from_rotation_x(t * PI / 12.0);
            (level_entry * pitch_up).normalize()
        }
        ManeuverPhaseTag::Ballistic => {
            // Full 360° backward pitch loop.
            // t=0 → level, t=0.5 → inverted (top of loop), t=1.0 → level again.
            let pitch = Quat::from_rotation_x(t * TAU);
            (level_entry * pitch).normalize()
        }
        ManeuverPhaseTag::Recovery => {
            // Hold level flight, same heading as entry.
            level_entry
        }
    }
}

/// Compute the thrust fraction (0.0–1.0) for a maneuvering drone.
pub fn maneuver_thrust_fraction(
    kind: ManeuverKind,
    phase: ManeuverPhaseTag,
    progress: f32,
) -> f32 {
    match kind {
        ManeuverKind::SplitS => match phase {
            ManeuverPhaseTag::Entry => 0.55,
            // Linear ramp from 0.4 (inverted, moderate thrust) to 1.0 (exiting dive, full thrust).
            // Front-loaded compared to old quadratic ramp so the drone pulls out of the dive earlier.
            ManeuverPhaseTag::Ballistic => 0.4 + 0.6 * progress,
            ManeuverPhaseTag::Recovery => 1.0,
        },
        ManeuverKind::PowerLoop => match phase {
            ManeuverPhaseTag::Entry => 0.9,
            ManeuverPhaseTag::Ballistic => {
                // U-shaped: 0.7 at start/end, 0.3 at midpoint (top of loop).
                let centered = 2.0 * progress - 1.0;
                0.3 + 0.4 * centered * centered
            }
            ManeuverPhaseTag::Recovery => 1.0,
        },
        ManeuverKind::AggressiveBank => 0.5,
    }
}

/// Get the duration (seconds) for a given maneuver phase.
/// Durations scale with entry speed: faster drones execute shorter phases.
pub fn phase_duration(kind: ManeuverKind, phase: ManeuverPhaseTag, entry_speed: f32) -> f32 {
    let scale = speed_scale(entry_speed);

    match kind {
        ManeuverKind::SplitS => match phase {
            ManeuverPhaseTag::Entry => 0.08 * scale,
            ManeuverPhaseTag::Ballistic => 0.28 * scale,
            ManeuverPhaseTag::Recovery => 0.15 * scale,
        },
        ManeuverKind::PowerLoop => match phase {
            ManeuverPhaseTag::Entry => 0.10 * scale,
            ManeuverPhaseTag::Ballistic => 0.35 * scale,
            ManeuverPhaseTag::Recovery => 0.10 * scale,
        },
        ManeuverKind::AggressiveBank => match phase {
            ManeuverPhaseTag::Entry => 0.10,
            ManeuverPhaseTag::Ballistic => 0.30,
            ManeuverPhaseTag::Recovery => 0.10,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAW_DIR: Vec3 = Vec3::NEG_Z;
    const VELOCITY: Vec3 = Vec3::new(0.0, 0.0, -30.0);
    const TOLERANCE: f32 = 0.01;

    // --- smoothstep ---

    #[test]
    fn smoothstep_endpoints_and_midpoint() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert_eq!(smoothstep(0.5), 0.5);
    }

    #[test]
    fn smoothstep_clamps_outside_zero_one() {
        assert_eq!(smoothstep(-0.5), 0.0);
        assert_eq!(smoothstep(1.5), 1.0);
    }

    // --- Split-S orientation ---

    #[test]
    fn split_s_entry_inverted_at_completion() {
        let q = maneuver_target_orientation(
            ManeuverKind::SplitS,
            ManeuverPhaseTag::Entry,
            1.0,
            YAW_DIR,
            VELOCITY,
        );
        let body_up = q.mul_vec3(Vec3::Y);
        assert!(
            (body_up - Vec3::NEG_Y).length() < TOLERANCE,
            "body-up should point down, got {body_up}"
        );
    }

    #[test]
    fn split_s_entry_level_at_start() {
        let q = maneuver_target_orientation(
            ManeuverKind::SplitS,
            ManeuverPhaseTag::Entry,
            0.0,
            YAW_DIR,
            VELOCITY,
        );
        let body_up = q.mul_vec3(Vec3::Y);
        assert!(
            (body_up - Vec3::Y).length() < TOLERANCE,
            "body-up should point up at start, got {body_up}"
        );
    }

    #[test]
    fn split_s_ballistic_level_reversed_at_completion() {
        let q = maneuver_target_orientation(
            ManeuverKind::SplitS,
            ManeuverPhaseTag::Ballistic,
            1.0,
            YAW_DIR,
            VELOCITY,
        );
        let body_up = q.mul_vec3(Vec3::Y);
        let forward = q.mul_vec3(Vec3::NEG_Z);

        assert!(
            (body_up - Vec3::Y).length() < TOLERANCE,
            "body-up should point up (level), got {body_up}"
        );
        assert!(
            (forward - (-YAW_DIR)).length() < TOLERANCE,
            "heading should be reversed, got {forward}"
        );
    }

    #[test]
    fn split_s_recovery_holds_reversed_heading() {
        let q = maneuver_target_orientation(
            ManeuverKind::SplitS,
            ManeuverPhaseTag::Recovery,
            0.5,
            YAW_DIR,
            VELOCITY,
        );
        let forward = q.mul_vec3(Vec3::NEG_Z);
        assert!(
            (forward - (-YAW_DIR)).length() < TOLERANCE,
            "recovery heading should be reversed, got {forward}"
        );
    }

    // --- Power Loop orientation ---

    #[test]
    fn power_loop_entry_pitched_up_at_completion() {
        let q = maneuver_target_orientation(
            ManeuverKind::PowerLoop,
            ManeuverPhaseTag::Entry,
            1.0,
            YAW_DIR,
            VELOCITY,
        );
        let body_up = q.mul_vec3(Vec3::Y);
        // Pitched up 15° (PI/12): body-up tilts backward (toward +Z since facing -Z).
        let angle = PI / 12.0;
        let expected_up = Vec3::new(0.0, angle.cos(), angle.sin());
        assert!(
            (body_up - expected_up).length() < TOLERANCE,
            "body-up should be tilted 15° back, got {body_up}"
        );
    }

    #[test]
    fn power_loop_ballistic_inverted_at_midpoint() {
        let q = maneuver_target_orientation(
            ManeuverKind::PowerLoop,
            ManeuverPhaseTag::Ballistic,
            0.5,
            YAW_DIR,
            VELOCITY,
        );
        let body_up = q.mul_vec3(Vec3::Y);
        assert!(
            (body_up - Vec3::NEG_Y).length() < TOLERANCE,
            "body-up should point down at top of loop, got {body_up}"
        );
    }

    #[test]
    fn power_loop_ballistic_level_at_completion() {
        let q = maneuver_target_orientation(
            ManeuverKind::PowerLoop,
            ManeuverPhaseTag::Ballistic,
            1.0,
            YAW_DIR,
            VELOCITY,
        );
        let body_up = q.mul_vec3(Vec3::Y);
        assert!(
            (body_up - Vec3::Y).length() < TOLERANCE,
            "body-up should point up at end of loop, got {body_up}"
        );
    }

    #[test]
    fn power_loop_recovery_holds_original_heading() {
        let q = maneuver_target_orientation(
            ManeuverKind::PowerLoop,
            ManeuverPhaseTag::Recovery,
            0.5,
            YAW_DIR,
            VELOCITY,
        );
        let forward = q.mul_vec3(Vec3::NEG_Z);
        assert!(
            (forward - YAW_DIR).length() < TOLERANCE,
            "recovery heading should match entry, got {forward}"
        );
    }

    // --- Thrust fractions ---

    #[test]
    fn all_thrust_fractions_in_valid_range() {
        for kind in [ManeuverKind::SplitS, ManeuverKind::PowerLoop] {
            for phase in [
                ManeuverPhaseTag::Entry,
                ManeuverPhaseTag::Ballistic,
                ManeuverPhaseTag::Recovery,
            ] {
                for i in 0..=20 {
                    let progress = i as f32 / 20.0;
                    let thrust = maneuver_thrust_fraction(kind, phase, progress);
                    assert!(
                        (0.0..=1.0).contains(&thrust),
                        "{kind:?}/{phase:?} at progress={progress}: thrust={thrust} out of range"
                    );
                }
            }
        }
    }

    #[test]
    fn split_s_thrust_curve_shape() {
        let entry = maneuver_thrust_fraction(ManeuverKind::SplitS, ManeuverPhaseTag::Entry, 0.5);
        assert!((entry - 0.55).abs() < f32::EPSILON, "entry should be constant 0.55");

        let ballistic_start =
            maneuver_thrust_fraction(ManeuverKind::SplitS, ManeuverPhaseTag::Ballistic, 0.0);
        let ballistic_end =
            maneuver_thrust_fraction(ManeuverKind::SplitS, ManeuverPhaseTag::Ballistic, 1.0);
        assert!(
            (ballistic_start - 0.4).abs() < f32::EPSILON,
            "ballistic start should be 0.4"
        );
        assert!(
            (ballistic_end - 1.0).abs() < f32::EPSILON,
            "ballistic end should be 1.0"
        );

        let recovery =
            maneuver_thrust_fraction(ManeuverKind::SplitS, ManeuverPhaseTag::Recovery, 0.5);
        assert!((recovery - 1.0).abs() < f32::EPSILON, "recovery should be 1.0");
    }

    #[test]
    fn power_loop_thrust_u_shaped() {
        let at_start =
            maneuver_thrust_fraction(ManeuverKind::PowerLoop, ManeuverPhaseTag::Ballistic, 0.0);
        let at_mid =
            maneuver_thrust_fraction(ManeuverKind::PowerLoop, ManeuverPhaseTag::Ballistic, 0.5);
        let at_end =
            maneuver_thrust_fraction(ManeuverKind::PowerLoop, ManeuverPhaseTag::Ballistic, 1.0);

        assert!((at_start - 0.7).abs() < f32::EPSILON, "start should be 0.7");
        assert!((at_mid - 0.3).abs() < f32::EPSILON, "midpoint should be 0.3");
        assert!((at_end - 0.7).abs() < f32::EPSILON, "end should be 0.7");
    }

    // --- Phase durations ---

    #[test]
    fn phase_durations_scale_with_speed() {
        let slow = phase_duration(ManeuverKind::SplitS, ManeuverPhaseTag::Entry, 15.0);
        let fast = phase_duration(ManeuverKind::SplitS, ManeuverPhaseTag::Entry, 60.0);
        assert!(
            slow > fast,
            "slower entry speed should produce longer phase: slow={slow}, fast={fast}"
        );
    }

    #[test]
    fn phase_durations_clamped_at_extremes() {
        // Very slow (1 m/s) should clamp at 1.4x scale
        let very_slow = phase_duration(ManeuverKind::SplitS, ManeuverPhaseTag::Entry, 1.0);
        assert!(
            (very_slow - 0.08 * 1.4).abs() < f32::EPSILON,
            "very slow should clamp to 1.4x: got {very_slow}"
        );

        // Very fast (100 m/s) should clamp at 0.6x scale
        let very_fast = phase_duration(ManeuverKind::SplitS, ManeuverPhaseTag::Entry, 100.0);
        assert!(
            (very_fast - 0.08 * 0.6).abs() < f32::EPSILON,
            "very fast should clamp to 0.6x: got {very_fast}"
        );
    }

    // --- Non-trivial yaw direction ---

    #[test]
    fn split_s_works_with_arbitrary_yaw_dir() {
        let yaw = Vec3::new(1.0, 0.0, 1.0).normalize();
        let vel = yaw * 30.0;

        let q_entry_end = maneuver_target_orientation(
            ManeuverKind::SplitS,
            ManeuverPhaseTag::Entry,
            1.0,
            yaw,
            vel,
        );
        let body_up = q_entry_end.mul_vec3(Vec3::Y);
        assert!(
            (body_up - Vec3::NEG_Y).length() < TOLERANCE,
            "inverted with diagonal yaw, body-up got {body_up}"
        );

        let q_ballistic_end = maneuver_target_orientation(
            ManeuverKind::SplitS,
            ManeuverPhaseTag::Ballistic,
            1.0,
            yaw,
            vel,
        );
        let forward = q_ballistic_end.mul_vec3(Vec3::NEG_Z);
        let expected_exit = -yaw;
        assert!(
            (forward - expected_exit).length() < TOLERANCE,
            "exit heading should be reversed: got {forward}, expected {expected_exit}"
        );
    }
}
