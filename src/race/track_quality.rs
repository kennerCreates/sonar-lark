use bevy::prelude::*;
use std::collections::HashSet;

use super::script::RaceScript;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

pub struct RaceSummary {
    pub gate_count: u32,
    pub distinct_obstacle_ids: u32,
    pub turn_tightness_counts: [u32; 3], // [gentle, medium, tight]
    pub elevation_deltas: Vec<f32>,
    pub overtake_count: u32,
    pub crash_count: u32,
    pub photo_finish_gap: f32,
}

#[derive(Resource)]
#[allow(dead_code)] // Sub-scores consumed by results UI in Step 5
pub struct TrackQuality {
    pub gate_count_score: f32,
    pub obstacle_variety_score: f32,
    pub turn_mix_score: f32,
    pub elevation_score: f32,
    pub overtake_score: f32,
    pub crash_score: f32,
    pub photo_finish_score: f32,
    pub overall: f32,
}

// ---------------------------------------------------------------------------
// Harvest
// ---------------------------------------------------------------------------

pub fn harvest_race_summary(
    script: &RaceScript,
    gate_positions: &[Vec3],
    obstacle_ids: &[&str],
    tightness_counts: [u32; 3],
    photo_finish_gap: f32,
) -> RaceSummary {
    let gate_count = gate_positions.len() as u32;

    let distinct_obstacle_ids = {
        let set: HashSet<&str> = obstacle_ids.iter().copied().collect();
        set.len() as u32
    };

    let elevation_deltas: Vec<f32> = gate_positions
        .windows(2)
        .map(|w| w[1].y - w[0].y)
        .collect();

    let overtake_count = script.overtakes.len() as u32;

    let crash_count = script
        .drone_scripts
        .iter()
        .filter(|ds| ds.crash.is_some())
        .count() as u32;

    RaceSummary {
        gate_count,
        distinct_obstacle_ids,
        turn_tightness_counts: tightness_counts,
        elevation_deltas,
        overtake_count,
        crash_count,
        photo_finish_gap,
    }
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

pub fn compute_track_quality(summary: &RaceSummary) -> TrackQuality {
    let gate_count_score = gaussian(summary.gate_count as f32, 11.0, 4.0);

    let obstacle_variety_score = (summary.distinct_obstacle_ids as f32 / 5.0).min(1.0);

    let turn_mix_score = shannon_entropy_normalized(&summary.turn_tightness_counts);

    let elevation_score = if summary.elevation_deltas.is_empty() {
        0.0
    } else {
        let mean_abs: f32 = summary.elevation_deltas.iter().map(|d| d.abs()).sum::<f32>()
            / summary.elevation_deltas.len() as f32;
        1.0 - (-mean_abs / 3.0).exp()
    };

    let overtake_score = (summary.overtake_count as f32 / 8.0).min(1.0);

    let crash_score = gaussian(summary.crash_count as f32, 1.5, 1.0);

    let photo_finish_score = (-summary.photo_finish_gap / 3.0).exp();

    let overall = gate_count_score * 0.10
        + obstacle_variety_score * 0.10
        + turn_mix_score * 0.20
        + elevation_score * 0.10
        + overtake_score * 0.20
        + crash_score * 0.15
        + photo_finish_score * 0.15;

    TrackQuality {
        gate_count_score,
        obstacle_variety_score,
        turn_mix_score,
        elevation_score,
        overtake_score,
        crash_score,
        photo_finish_score,
        overall,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Gaussian bell: e^(-((x - center)^2 / (2 * sigma^2)))
fn gaussian(x: f32, center: f32, sigma_sq: f32) -> f32 {
    let diff = x - center;
    (-(diff * diff) / (2.0 * sigma_sq)).exp()
}

/// Shannon entropy of a distribution normalized to [0, 1].
/// Max entropy for `bins.len()` categories = ln(bins.len()).
fn shannon_entropy_normalized(bins: &[u32]) -> f32 {
    let total: u32 = bins.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f32;
    let mut h: f32 = 0.0;
    for &count in bins {
        if count > 0 {
            let p = count as f32 / total_f;
            h -= p * p.ln();
        }
    }
    let max_h = (bins.len() as f32).ln();
    if max_h == 0.0 {
        0.0
    } else {
        h / max_h
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::script::{DroneScript, RaceScript, ScriptedOvertake};

    fn make_script(drone_count: u32, crash_indices: &[usize], overtake_count: u32) -> RaceScript {
        let drone_scripts = (0..drone_count)
            .map(|i| DroneScript {
                segment_pace: vec![1.0],
                crash: if crash_indices.contains(&(i as usize)) {
                    Some(crate::race::script::CrashScript {
                        gate_index: 2,
                        progress_past_gate: 0.5,
                        crash_type: crate::race::script::ScriptedCrashType::ObstacleCollision,
                    })
                } else {
                    None
                },
                acrobatic_gates: vec![],
                gate_pass_t: vec![],
            })
            .collect();

        let overtakes = (0..overtake_count)
            .map(|i| ScriptedOvertake {
                gate_index: i,
                overtaker_idx: 0,
                overtaken_idx: 1,
            })
            .collect();

        RaceScript {
            drone_scripts,
            overtakes,
        }
    }

    fn make_summary(
        gate_count: u32,
        distinct_obstacle_ids: u32,
        turn_tightness_counts: [u32; 3],
        elevation_deltas: Vec<f32>,
        overtake_count: u32,
        crash_count: u32,
        photo_finish_gap: f32,
    ) -> RaceSummary {
        RaceSummary {
            gate_count,
            distinct_obstacle_ids,
            turn_tightness_counts,
            elevation_deltas,
            overtake_count,
            crash_count,
            photo_finish_gap,
        }
    }

    #[test]
    fn test_gate_count_score_peak() {
        let s = make_summary(11, 3, [4, 4, 4], vec![], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!((q.gate_count_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gate_count_score_low() {
        let s = make_summary(3, 3, [4, 4, 4], vec![], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!(q.gate_count_score < 0.5, "got {}", q.gate_count_score);
    }

    #[test]
    fn test_obstacle_variety_caps_at_one() {
        let s = make_summary(11, 10, [4, 4, 4], vec![], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!((q.obstacle_variety_score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_turn_mix_uniform() {
        let s = make_summary(11, 3, [4, 4, 4], vec![], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!(
            (q.turn_mix_score - 1.0).abs() < 0.01,
            "got {}",
            q.turn_mix_score
        );
    }

    #[test]
    fn test_turn_mix_all_one_type() {
        let s = make_summary(11, 3, [12, 0, 0], vec![], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!(
            q.turn_mix_score.abs() < f32::EPSILON,
            "got {}",
            q.turn_mix_score
        );
    }

    #[test]
    fn test_elevation_zero() {
        let s = make_summary(11, 3, [4, 4, 4], vec![], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!(
            q.elevation_score.abs() < f32::EPSILON,
            "got {}",
            q.elevation_score
        );
    }

    #[test]
    fn test_elevation_high() {
        let s = make_summary(11, 3, [4, 4, 4], vec![10.0, -8.0, 12.0], 0, 0, 1.0);
        let q = compute_track_quality(&s);
        assert!(q.elevation_score > 0.9, "got {}", q.elevation_score);
    }

    #[test]
    fn test_overtake_diminishing() {
        let s8 = make_summary(11, 3, [4, 4, 4], vec![], 8, 0, 1.0);
        let s16 = make_summary(11, 3, [4, 4, 4], vec![], 16, 0, 1.0);
        let q8 = compute_track_quality(&s8);
        let q16 = compute_track_quality(&s16);
        assert!((q8.overtake_score - 1.0).abs() < f32::EPSILON);
        assert!((q16.overtake_score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_crash_sweet_spot() {
        let s1 = make_summary(11, 3, [4, 4, 4], vec![], 0, 1, 1.0);
        let s2 = make_summary(11, 3, [4, 4, 4], vec![], 0, 2, 1.0);
        let q1 = compute_track_quality(&s1);
        let q2 = compute_track_quality(&s2);
        // Both 1 and 2 should score high (near the 1.5 center)
        assert!(q1.crash_score > 0.7, "got {}", q1.crash_score);
        assert!(q2.crash_score > 0.7, "got {}", q2.crash_score);
    }

    #[test]
    fn test_crash_zero_penalized() {
        let s0 = make_summary(11, 3, [4, 4, 4], vec![], 0, 0, 1.0);
        let s1 = make_summary(11, 3, [4, 4, 4], vec![], 0, 1, 1.0);
        let q0 = compute_track_quality(&s0);
        let q1 = compute_track_quality(&s1);
        assert!(
            q0.crash_score < q1.crash_score,
            "0 crashes {} should be less than 1 crash {}",
            q0.crash_score,
            q1.crash_score
        );
    }

    #[test]
    fn test_photo_finish_tight() {
        let s = make_summary(11, 3, [4, 4, 4], vec![], 0, 0, 0.1);
        let q = compute_track_quality(&s);
        assert!(q.photo_finish_score > 0.95, "got {}", q.photo_finish_score);
    }

    #[test]
    fn test_photo_finish_wide() {
        let s = make_summary(11, 3, [4, 4, 4], vec![], 0, 0, 20.0);
        let q = compute_track_quality(&s);
        assert!(q.photo_finish_score < 0.01, "got {}", q.photo_finish_score);
    }

    #[test]
    fn test_overall_weighted_sum() {
        let s = make_summary(11, 5, [4, 4, 4], vec![3.0, -2.0], 4, 1, 1.0);
        let q = compute_track_quality(&s);
        let expected = q.gate_count_score * 0.10
            + q.obstacle_variety_score * 0.10
            + q.turn_mix_score * 0.20
            + q.elevation_score * 0.10
            + q.overtake_score * 0.20
            + q.crash_score * 0.15
            + q.photo_finish_score * 0.15;
        assert!(
            (q.overall - expected).abs() < 1e-6,
            "overall {} != expected {}",
            q.overall,
            expected
        );
    }

    #[test]
    fn test_zero_gates_edge_case() {
        let s = make_summary(0, 0, [0, 0, 0], vec![], 0, 0, 0.0);
        let q = compute_track_quality(&s);
        // Should not panic; gate_count_score should be low
        assert!(q.gate_count_score < 0.01);
    }

    #[test]
    fn test_harvest_basic() {
        let script = make_script(4, &[1, 3], 5);
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 2.0, 0.0),
            Vec3::new(20.0, 5.0, 0.0),
        ];
        let obstacle_ids = vec!["ring", "hoop", "ring", "pillar"];
        let tightness = [3, 2, 1];
        let summary = harvest_race_summary(&script, &positions, &obstacle_ids, tightness, 0.5);

        assert_eq!(summary.gate_count, 3);
        assert_eq!(summary.distinct_obstacle_ids, 3); // ring, hoop, pillar
        assert_eq!(summary.turn_tightness_counts, [3, 2, 1]);
        assert_eq!(summary.elevation_deltas.len(), 2);
        assert!((summary.elevation_deltas[0] - 2.0).abs() < f32::EPSILON);
        assert!((summary.elevation_deltas[1] - 3.0).abs() < f32::EPSILON);
        assert_eq!(summary.overtake_count, 5);
        assert_eq!(summary.crash_count, 2);
        assert!((summary.photo_finish_gap - 0.5).abs() < f32::EPSILON);
    }
}
