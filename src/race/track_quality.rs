use bevy::prelude::*;
use std::collections::HashSet;

use crate::course::data::gate_spectacle_weight;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

pub struct RaceSummary {
    pub gate_count: u32,
    pub distinct_obstacle_ids: u32,
    pub turn_tightness_counts: [u32; 3], // [gentle, medium, tight]
    pub elevation_deltas: Vec<f32>,
    /// Obstacle IDs of gates only (e.g. "gate_ground", "gate_air").
    pub gate_obstacle_ids: Vec<String>,
}

#[derive(Resource)]
pub struct TrackQuality {
    pub gate_count_score: f32,
    pub obstacle_variety_score: f32,
    pub turn_mix_score: f32,
    pub elevation_score: f32,
    pub gate_spectacle_score: f32,
    pub overall: f32,
}

// ---------------------------------------------------------------------------
// Harvest
// ---------------------------------------------------------------------------

pub fn harvest_race_summary(
    gate_positions: &[Vec3],
    obstacle_ids: &[&str],
    gate_obstacle_ids: &[&str],
    tightness_counts: [u32; 3],
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

    RaceSummary {
        gate_count,
        distinct_obstacle_ids,
        turn_tightness_counts: tightness_counts,
        elevation_deltas,
        gate_obstacle_ids: gate_obstacle_ids.iter().map(|s| s.to_string()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// `venue_capacity` scales the ideal gate count: small venues want fewer gates,
/// large venues want more. The ideal is linearly interpolated from 6 (capacity 40)
/// to 14 (capacity 200+).
pub fn compute_track_quality(summary: &RaceSummary, venue_capacity: u32) -> TrackQuality {
    let ideal_gates = ideal_gate_count(venue_capacity);
    let gate_count_score = gaussian(summary.gate_count as f32, ideal_gates, 4.0);

    let obstacle_variety_score = (summary.distinct_obstacle_ids as f32 / 5.0).min(1.0);

    let turn_mix_score = shannon_entropy_normalized(&summary.turn_tightness_counts);

    let elevation_score = if summary.elevation_deltas.is_empty() {
        0.0
    } else {
        let mean_abs: f32 = summary.elevation_deltas.iter().map(|d| d.abs()).sum::<f32>()
            / summary.elevation_deltas.len() as f32;
        1.0 - (-mean_abs / 3.0).exp()
    };

    // Gate spectacle: average weight / max weight (4.0). Air gates score highest.
    let gate_spectacle_score = if summary.gate_obstacle_ids.is_empty() {
        0.0
    } else {
        let total_weight: f32 = summary
            .gate_obstacle_ids
            .iter()
            .map(|id| gate_spectacle_weight(id))
            .sum();
        let avg = total_weight / summary.gate_obstacle_ids.len() as f32;
        (avg / 4.0).min(1.0)
    };

    let overall = gate_count_score * 0.20
        + obstacle_variety_score * 0.20
        + turn_mix_score * 0.20
        + elevation_score * 0.20
        + gate_spectacle_score * 0.20;

    TrackQuality {
        gate_count_score,
        obstacle_variety_score,
        turn_mix_score,
        elevation_score,
        gate_spectacle_score,
        overall,
    }
}

/// Ideal gate count scales linearly with venue capacity:
/// capacity 40 → 6 gates, capacity 200+ → 14 gates.
fn ideal_gate_count(capacity: u32) -> f32 {
    let t = ((capacity as f32 - 40.0) / 160.0).clamp(0.0, 1.0);
    6.0 + t * 8.0
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

    const DEFAULT_CAPACITY: u32 = 100;

    fn make_summary(
        gate_count: u32,
        distinct_obstacle_ids: u32,
        turn_tightness_counts: [u32; 3],
        elevation_deltas: Vec<f32>,
    ) -> RaceSummary {
        RaceSummary {
            gate_count,
            distinct_obstacle_ids,
            turn_tightness_counts,
            elevation_deltas,
            gate_obstacle_ids: vec![],
        }
    }

    #[test]
    fn test_ideal_gate_count_scales_with_capacity() {
        assert!((ideal_gate_count(40) - 6.0).abs() < 0.01);
        assert!((ideal_gate_count(120) - 10.0).abs() < 0.01);
        assert!((ideal_gate_count(200) - 14.0).abs() < 0.01);
        // Clamps above 200
        assert!((ideal_gate_count(500) - 14.0).abs() < 0.01);
    }

    #[test]
    fn test_gate_count_score_peak_small_venue() {
        let s = make_summary(6, 3, [4, 4, 4], vec![]);
        let q = compute_track_quality(&s, 40);
        assert!((q.gate_count_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gate_count_score_peak_large_venue() {
        let s = make_summary(14, 3, [4, 4, 4], vec![]);
        let q = compute_track_quality(&s, 200);
        assert!((q.gate_count_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gate_count_score_low() {
        let s = make_summary(3, 3, [4, 4, 4], vec![]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!(q.gate_count_score < 0.5, "got {}", q.gate_count_score);
    }

    #[test]
    fn test_obstacle_variety_caps_at_one() {
        let s = make_summary(11, 10, [4, 4, 4], vec![]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!((q.obstacle_variety_score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_turn_mix_uniform() {
        let s = make_summary(11, 3, [4, 4, 4], vec![]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!(
            (q.turn_mix_score - 1.0).abs() < 0.01,
            "got {}",
            q.turn_mix_score
        );
    }

    #[test]
    fn test_turn_mix_all_one_type() {
        let s = make_summary(11, 3, [12, 0, 0], vec![]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!(
            q.turn_mix_score.abs() < f32::EPSILON,
            "got {}",
            q.turn_mix_score
        );
    }

    #[test]
    fn test_elevation_zero() {
        let s = make_summary(11, 3, [4, 4, 4], vec![]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!(
            q.elevation_score.abs() < f32::EPSILON,
            "got {}",
            q.elevation_score
        );
    }

    #[test]
    fn test_elevation_high() {
        let s = make_summary(11, 3, [4, 4, 4], vec![10.0, -8.0, 12.0]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!(q.elevation_score > 0.9, "got {}", q.elevation_score);
    }

    #[test]
    fn test_overall_weighted_sum() {
        let s = make_summary(10, 5, [4, 4, 4], vec![3.0, -2.0]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        let expected = q.gate_count_score * 0.20
            + q.obstacle_variety_score * 0.20
            + q.turn_mix_score * 0.20
            + q.elevation_score * 0.20
            + q.gate_spectacle_score * 0.20;
        assert!(
            (q.overall - expected).abs() < 1e-6,
            "overall {} != expected {}",
            q.overall,
            expected
        );
    }

    #[test]
    fn test_zero_gates_edge_case() {
        let s = make_summary(0, 0, [0, 0, 0], vec![]);
        let q = compute_track_quality(&s, DEFAULT_CAPACITY);
        assert!(q.gate_count_score < 0.01);
    }

    #[test]
    fn test_harvest_basic() {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 2.0, 0.0),
            Vec3::new(20.0, 5.0, 0.0),
        ];
        let obstacle_ids = vec!["ring", "hoop", "ring", "pillar"];
        let tightness = [3, 2, 1];
        let gate_ids = vec!["gate_air", "gate_ground", "gate_loop"];
        let summary = harvest_race_summary(&positions, &obstacle_ids, &gate_ids, tightness);

        assert_eq!(summary.gate_count, 3);
        assert_eq!(summary.distinct_obstacle_ids, 3); // ring, hoop, pillar
        assert_eq!(summary.turn_tightness_counts, [3, 2, 1]);
        assert_eq!(summary.elevation_deltas.len(), 2);
        assert!((summary.elevation_deltas[0] - 2.0).abs() < f32::EPSILON);
        assert!((summary.elevation_deltas[1] - 3.0).abs() < f32::EPSILON);
        assert_eq!(summary.gate_obstacle_ids.len(), 3);
    }
}
