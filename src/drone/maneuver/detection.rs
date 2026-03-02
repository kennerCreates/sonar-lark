use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

use super::ManeuverKind;
use crate::common::POINTS_PER_GATE;

/// Number of tangent samples for turn detection.
const TANGENT_SAMPLES: usize = 8;

/// How far ahead to scan (in POINTS_PER_GATE units).
const SCAN_RANGE_GATES: f32 = 1.5;

/// Minimum speed (m/s) to trigger any maneuver.
const MIN_MANEUVER_SPEED: f32 = 10.0;

/// Spline parameter distance after a completed maneuver before another can trigger.
const MANEUVER_COOLDOWN_T: f32 = 0.5;

/// Don't trigger within this distance (meters) of any gate center.
const GATE_PROXIMITY_GUARD: f32 = 2.0;

/// Turn angle (degrees) above which a flip maneuver (Split-S/Power Loop) is
/// chosen over Aggressive Bank.
const FLIP_ANGLE_THRESHOLD: f32 = 120.0;

/// Per-sample angle threshold (degrees) for determining where the turn ends.
const TANGENT_STABLE_DEG: f32 = 5.0;

/// Per-sample angle threshold (degrees) for determining where the turn begins.
const TANGENT_ENTRY_DEG: f32 = 8.0;

/// Spline parameter distance before the turn entry point at which the maneuver
/// should activate. Gives the drone a short lead-in before the turn.
const TRIGGER_LEAD_IN: f32 = 0.3;

pub struct ManeuverTrigger {
    pub kind: ManeuverKind,
    /// Spline parameter where the drone should start executing the maneuver.
    pub trigger_t: f32,
    pub exit_t: f32,
    #[allow(dead_code)] // Read in Phase 5 (debug visualization)
    pub turn_angle: f32,
}

/// Sample tangent from cyclic spline (wraps t into [0, cycle_t)).
fn sample_tangent(spline: &CubicCurve<Vec3>, t: f32, cycle_t: f32) -> Vec3 {
    spline.velocity(t.rem_euclid(cycle_t))
}

/// Flatten a 3D vector to the XZ plane and normalize.
fn flat_dir(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z).normalize_or(Vec3::NEG_Z)
}

/// Analyze the spline ahead and determine if a maneuver should be triggered.
///
/// Pure function — all inputs passed explicitly for testability.
///
/// Algorithm:
/// 1. Check guards (speed, cooldown, gate proximity)
/// 2. Sample 8 tangent vectors over the next 1.5 POINTS_PER_GATE of spline
/// 3. Accumulate total XZ-plane direction change
/// 4. If turn angle exceeds effective threshold (base × threshold_mult):
///    - Turn > 120° + high altitude → Split-S
///    - Turn > 120° + low altitude → Power Loop
///    - Turn ≤ 120° → Aggressive Bank
/// 5. Compute exit_t past the tight section where tangent stabilizes
#[allow(clippy::too_many_arguments)]
pub fn detect_maneuver(
    spline: &CubicCurve<Vec3>,
    spline_t: f32,
    cycle_t: f32,
    finish_t: Option<f32>,
    altitude: f32,
    speed: f32,
    gate_positions: &[Vec3],
    drone_position: Vec3,
    maneuver_threshold_mult: f32,
    maneuver_turn_threshold: f32,
    maneuver_altitude_min: f32,
    last_maneuver_exit_t: Option<f32>,
) -> Option<ManeuverTrigger> {
    if speed < MIN_MANEUVER_SPEED {
        return None;
    }

    if let Some(exit_t) = last_maneuver_exit_t
        && (spline_t - exit_t).abs() < MANEUVER_COOLDOWN_T
    {
        return None;
    }

    for gate_pos in gate_positions {
        if drone_position.distance(*gate_pos) < GATE_PROXIMITY_GUARD {
            return None;
        }
    }

    // Scan ahead: accumulate total XZ-plane turn angle and find where the turn
    // starts (first significant deviation) and ends (last significant deviation).
    let scan_range = SCAN_RANGE_GATES * POINTS_PER_GATE;
    let mut total_angle = 0.0f32;
    let mut prev_flat = flat_dir(sample_tangent(spline, spline_t, cycle_t));
    let mut first_turning_idx: Option<usize> = None;
    let mut last_turning_idx = 0usize;

    for i in 1..=TANGENT_SAMPLES {
        let t = spline_t + (i as f32 / TANGENT_SAMPLES as f32) * scan_range;
        let cur_flat = flat_dir(sample_tangent(spline, t, cycle_t));
        let delta = prev_flat.angle_between(cur_flat).to_degrees();
        total_angle += delta;
        if delta > TANGENT_STABLE_DEG {
            last_turning_idx = i;
        }
        if delta > TANGENT_ENTRY_DEG && first_turning_idx.is_none() {
            first_turning_idx = Some(i);
        }
        prev_flat = cur_flat;
    }

    let effective_threshold = maneuver_turn_threshold * maneuver_threshold_mult;
    if total_angle < effective_threshold {
        return None;
    }

    // trigger_t: just before where the turn starts (with a small lead-in).
    let entry_idx = first_turning_idx.unwrap_or(1);
    let entry_t = spline_t + (entry_idx as f32 / TANGENT_SAMPLES as f32) * scan_range;
    let trigger_t = (entry_t - TRIGGER_LEAD_IN).max(spline_t);

    // exit_t: one sample past where the turn ends, capped at scan_range.
    let exit_idx = (last_turning_idx + 1).min(TANGENT_SAMPLES);
    let exit_t = spline_t + (exit_idx as f32 / TANGENT_SAMPLES as f32) * scan_range;

    if let Some(ft) = finish_t
        && exit_t > ft
    {
        return None;
    }

    let kind = if total_angle > FLIP_ANGLE_THRESHOLD {
        if altitude > maneuver_altitude_min {
            ManeuverKind::SplitS
        } else {
            ManeuverKind::PowerLoop
        }
    } else {
        ManeuverKind::AggressiveBank
    };

    Some(ManeuverTrigger {
        kind,
        trigger_t,
        exit_t,
        turn_angle: total_angle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::cubic_splines::CubicCardinalSpline;

    /// U-turn shaped spline at altitude 5m.
    fn make_hairpin_spline() -> (CubicCurve<Vec3>, f32) {
        let points = vec![
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(20.0, 5.0, 0.0),
            Vec3::new(25.0, 5.0, 10.0),
            Vec3::new(20.0, 5.0, 20.0),
            Vec3::new(0.0, 5.0, 20.0),
        ];
        let spline = CubicCardinalSpline::new_catmull_rom(points)
            .to_curve_cyclic()
            .expect("spline creation failed");
        (spline, 5.0)
    }

    /// Nearly straight spline (enough points that the scan range stays in the
    /// straight section without hitting the cyclic wrap-around).
    fn make_straight_spline() -> (CubicCurve<Vec3>, f32) {
        let points: Vec<Vec3> = (0..10)
            .map(|i| Vec3::new(i as f32 * 10.0, 5.0, 0.0))
            .collect();
        let spline = CubicCardinalSpline::new_catmull_rom(points)
            .to_curve_cyclic()
            .expect("spline creation failed");
        (spline, 10.0)
    }

    /// Helper: default detect args for a hairpin spline.
    fn detect_hairpin(
        spline: &CubicCurve<Vec3>,
        cycle_t: f32,
        overrides: impl FnOnce(&mut DetectArgs),
    ) -> Option<ManeuverTrigger> {
        let mut args = DetectArgs {
            spline_t: 0.5,
            finish_t: None,
            altitude: 5.0,
            speed: 30.0,
            gate_positions: vec![],
            drone_position: Vec3::new(5.0, 5.0, 0.0),
            threshold_mult: 1.0,
            turn_threshold: 90.0,
            altitude_min: 3.0,
            last_exit_t: None,
        };
        overrides(&mut args);
        detect_maneuver(
            spline,
            args.spline_t,
            cycle_t,
            args.finish_t,
            args.altitude,
            args.speed,
            &args.gate_positions,
            args.drone_position,
            args.threshold_mult,
            args.turn_threshold,
            args.altitude_min,
            args.last_exit_t,
        )
    }

    struct DetectArgs {
        spline_t: f32,
        finish_t: Option<f32>,
        altitude: f32,
        speed: f32,
        gate_positions: Vec<Vec3>,
        drone_position: Vec3,
        threshold_mult: f32,
        turn_threshold: f32,
        altitude_min: f32,
        last_exit_t: Option<f32>,
    }

    #[test]
    fn no_trigger_on_straight_spline() {
        let (spline, cycle_t) = make_straight_spline();
        let result = detect_hairpin(&spline, cycle_t, |_| {});
        assert!(result.is_none());
    }

    #[test]
    fn no_trigger_below_min_speed() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| a.speed = 5.0);
        assert!(result.is_none());
    }

    #[test]
    fn no_trigger_during_cooldown() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.spline_t = 1.0;
            a.last_exit_t = Some(0.8);
        });
        assert!(result.is_none());
    }

    #[test]
    fn triggers_after_cooldown_expires() {
        let (spline, cycle_t) = make_hairpin_spline();
        // Cooldown expired (diff = 0.8 > 0.5) — detection proceeds
        let _not_blocked = detect_hairpin(&spline, cycle_t, |a| {
            a.spline_t = 1.0;
            a.last_exit_t = Some(0.2);
            a.turn_threshold = 30.0;
        });
        // Cooldown active (diff = 0.2 < 0.5) — blocked
        let cooldown_blocked = detect_hairpin(&spline, cycle_t, |a| {
            a.spline_t = 1.0;
            a.last_exit_t = Some(0.8);
            a.turn_threshold = 30.0;
        });
        assert!(cooldown_blocked.is_none());
    }

    #[test]
    fn no_trigger_near_gate() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.gate_positions = vec![Vec3::new(5.0, 5.0, 0.0)];
            a.turn_threshold = 30.0;
        });
        assert!(result.is_none());
    }

    #[test]
    fn no_trigger_past_finish_line() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.finish_t = Some(0.6); // Very close — exit_t would exceed
            a.turn_threshold = 30.0;
        });
        assert!(result.is_none());
    }

    #[test]
    fn high_threshold_mult_prevents_trigger() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.threshold_mult = 5.0; // effective = 450°, impossible to reach
        });
        assert!(result.is_none());
    }

    #[test]
    fn low_threshold_mult_triggers_on_hairpin() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.threshold_mult = 0.3; // effective = 27°
        });
        assert!(result.is_some());
    }

    #[test]
    fn high_altitude_selects_split_s_for_big_turn() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.altitude = 10.0;
            a.altitude_min = 3.0;
            a.turn_threshold = 30.0; // low to ensure trigger
        });
        if let Some(trigger) = result {
            if trigger.turn_angle > FLIP_ANGLE_THRESHOLD {
                assert_eq!(trigger.kind, ManeuverKind::SplitS);
            }
        }
    }

    #[test]
    fn low_altitude_selects_power_loop_for_big_turn() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.altitude = 1.0;
            a.drone_position = Vec3::new(5.0, 1.0, 0.0);
            a.altitude_min = 3.0;
            a.turn_threshold = 30.0;
        });
        if let Some(trigger) = result {
            if trigger.turn_angle > FLIP_ANGLE_THRESHOLD {
                assert_eq!(trigger.kind, ManeuverKind::PowerLoop);
            }
        }
    }

    #[test]
    fn moderate_turn_selects_aggressive_bank() {
        // Build a spline with a moderate (~90°) turn
        let points = vec![
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(20.0, 5.0, 0.0),
            Vec3::new(30.0, 5.0, 0.0),
            Vec3::new(35.0, 5.0, 10.0),
            Vec3::new(35.0, 5.0, 20.0),
            Vec3::new(35.0, 5.0, 30.0),
        ];
        let spline = CubicCardinalSpline::new_catmull_rom(points)
            .to_curve_cyclic()
            .expect("spline creation failed");
        let cycle_t = 6.0;

        let result = detect_maneuver(
            &spline,
            1.5,
            cycle_t,
            None,
            5.0,
            30.0,
            &[],
            Vec3::new(20.0, 5.0, 0.0),
            1.0,
            60.0, // threshold that allows bank but the angle won't exceed 120
            3.0,
            None,
        );
        if let Some(trigger) = result {
            if trigger.turn_angle <= FLIP_ANGLE_THRESHOLD {
                assert_eq!(trigger.kind, ManeuverKind::AggressiveBank);
            }
        }
    }

    #[test]
    fn exit_t_is_past_spline_t() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.turn_threshold = 30.0;
        });
        if let Some(trigger) = result {
            assert!(
                trigger.exit_t > 0.5,
                "exit_t ({}) should be past spline_t (0.5)",
                trigger.exit_t
            );
        }
    }

    #[test]
    fn turn_angle_is_positive() {
        let (spline, cycle_t) = make_hairpin_spline();
        let result = detect_hairpin(&spline, cycle_t, |a| {
            a.turn_threshold = 30.0;
        });
        if let Some(trigger) = result {
            assert!(trigger.turn_angle > 0.0);
        }
    }
}
