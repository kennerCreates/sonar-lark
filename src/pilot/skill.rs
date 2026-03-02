use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::drone::components::DroneConfig;

use super::personality::PersonalityTrait;

/// Skill profile: overall level + per-axis variation.
/// All values in 0.0 (novice) to 1.0 (elite).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillProfile {
    pub level: f32,
    pub speed: f32,
    pub cornering: f32,
    pub consistency: f32,
}

/// Generate a DroneConfig from skill profile + personality traits.
///
/// The approach: each DroneConfig field has an "optimal" center value and a base range.
/// Skill compresses the randomization range toward optimal. Personality traits then shift
/// the result. Final values are clamped to the same bounds as `randomize_drone_config()`.
pub fn generate_drone_config(
    skill: &SkillProfile,
    traits: &[PersonalityTrait],
    rng: &mut impl Rng,
) -> DroneConfig {
    // Effective skill factors per axis (blend overall level with axis-specific)
    let s_speed = (skill.level * 0.4 + skill.speed * 0.6).clamp(0.0, 1.0);
    let s_corner = (skill.level * 0.4 + skill.cornering * 0.6).clamp(0.0, 1.0);
    let s_consist = (skill.level * 0.4 + skill.consistency * 0.6).clamp(0.0, 1.0);

    // Higher skill → tighter range around optimal (range shrinks to 30% at skill=1.0)
    let range_factor = |skill_val: f32| -> f32 { 1.0 - skill_val * 0.7 };

    // --- Generate base values ---

    // PID variation: optimal = 0, range ±0.15, governed by consistency
    let pid_range = 0.15 * range_factor(s_consist);
    let pid_variation = Vec3::new(
        rng.gen_range(-pid_range..=pid_range),
        rng.gen_range(-pid_range..=pid_range),
        rng.gen_range(-pid_range..=pid_range),
    );

    // Cornering aggression: optimal ≈ 1.05, range 0.8..1.2, governed by cornering
    let corner_range = 0.2 * range_factor(s_corner);
    let cornering_aggression = 1.05 + rng.gen_range(-corner_range..=corner_range);

    // Braking distance: optimal ≈ 0.95 (slightly late braker), range 0.8..1.2
    let brake_range = 0.2 * range_factor(s_corner);
    let braking_distance = 0.95 + rng.gen_range(-brake_range..=brake_range);

    // Racing line bias: derived from cornering_aggression (as in original)
    let raw_bias: f32 = rng.gen_range(-1.0..=1.0);
    let racing_line_bias = raw_bias * (2.0 + cornering_aggression * 2.0);

    // Approach offset scale: derived from cornering_aggression (as in original)
    let approach_offset_scale = 1.0 - (cornering_aggression - 1.0) * 0.5;

    // Line offset: optimal = 0, range ±1.5, governed by speed
    let line_range = 1.5 * range_factor(s_speed);
    let line_offset = rng.gen_range(-line_range..=line_range);

    // Noise amplitude: optimal ≈ 0.3 (minimum realistic), range 0.3..1.5
    let noise_range = 1.2 * range_factor(s_consist);
    let noise_amplitude = 0.3 + rng.gen_range(0.0..=noise_range);

    // Noise frequency: optimal ≈ 1.0, range 0.5..2.0
    let freq_range = 0.75 * range_factor(s_consist);
    let noise_frequency = 1.0 + rng.gen_range(-freq_range..=freq_range);

    // Attitude gains: optimal = 1.0, range 0.9..1.1
    let att_range = 0.1 * range_factor(s_consist);
    let attitude_kp_mult = 1.0 + rng.gen_range(-att_range..=att_range);
    let attitude_kd_mult = 1.0 + rng.gen_range(-att_range..=att_range);

    // Gate pass offset: optimal ≈ 0.35, range 0.19..0.61
    let gate_base = 0.2 + rng.gen_range(0.0..=0.2 * range_factor(s_corner));
    let gate_pass_offset = gate_base + (cornering_aggression - 0.8) * 0.5;

    // Maneuver threshold mult: optimal = 1.0, range 0.85..1.15, governed by cornering
    let maneuver_range = 0.15 * range_factor(s_corner);
    let maneuver_threshold_mult = 1.0 + rng.gen_range(-maneuver_range..=maneuver_range);

    // Hover noise: optimal = low, governed by consistency
    let hover_factor = range_factor(s_consist);
    let hover_noise_amp = Vec3::new(
        rng.gen_range(0.05..=0.05 + 0.10 * hover_factor),
        rng.gen_range(0.02..=0.02 + 0.04 * hover_factor),
        rng.gen_range(0.05..=0.05 + 0.07 * hover_factor),
    );
    let hover_noise_freq = Vec3::new(
        rng.gen_range(0.1..=0.1 + 0.4 * hover_factor),
        rng.gen_range(0.15..=0.15 + 0.35 * hover_factor),
        rng.gen_range(0.1..=0.1 + 0.3 * hover_factor),
    );

    // --- Apply personality trait modifiers ---
    let mut total_cornering_adj = 0.0f32;
    let mut total_braking_adj = 0.0f32;
    let mut total_noise_adj = 0.0f32;
    let mut total_gate_adj = 0.0f32;
    let mut total_approach_adj = 0.0f32;
    let mut line_bias_scale = 1.0f32;
    let mut pid_scale = 1.0f32;
    let mut total_maneuver_adj = 0.0f32;

    for trait_ in traits {
        let m = trait_.modifiers();
        total_cornering_adj += m.cornering_aggression;
        total_braking_adj += m.braking_distance;
        total_noise_adj += m.noise_amplitude;
        total_gate_adj += m.gate_pass_offset;
        total_approach_adj += m.approach_offset_scale;
        line_bias_scale *= m.racing_line_bias_scale;
        pid_scale *= m.pid_variation_scale;
        total_maneuver_adj += m.maneuver_threshold;
    }

    // --- Clamp to the same bounds as randomize_drone_config() ---
    DroneConfig {
        pid_variation: (pid_variation * pid_scale).clamp(Vec3::splat(-0.15), Vec3::splat(0.15)),
        line_offset: line_offset.clamp(-1.5, 1.5),
        noise_amplitude: (noise_amplitude + total_noise_adj).clamp(0.3, 1.5),
        noise_frequency: noise_frequency.clamp(0.5, 2.0),
        hover_noise_amp: hover_noise_amp.clamp(
            Vec3::new(0.05, 0.02, 0.05),
            Vec3::new(0.15, 0.06, 0.12),
        ),
        hover_noise_freq: hover_noise_freq.clamp(
            Vec3::new(0.1, 0.15, 0.1),
            Vec3::new(0.5, 0.5, 0.4),
        ),
        cornering_aggression: (cornering_aggression + total_cornering_adj).clamp(0.8, 1.2),
        braking_distance: (braking_distance + total_braking_adj).clamp(0.8, 1.2),
        attitude_kp_mult: attitude_kp_mult.clamp(0.9, 1.1),
        attitude_kd_mult: attitude_kd_mult.clamp(0.9, 1.1),
        racing_line_bias: (racing_line_bias * line_bias_scale).clamp(-4.4, 4.4),
        approach_offset_scale: (approach_offset_scale + total_approach_adj).clamp(0.89, 1.11),
        gate_pass_offset: (gate_pass_offset + total_gate_adj).clamp(0.19, 0.61),
        maneuver_threshold_mult: (maneuver_threshold_mult + total_maneuver_adj).clamp(0.7, 1.3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn medium_skill() -> SkillProfile {
        SkillProfile {
            level: 0.5,
            speed: 0.5,
            cornering: 0.5,
            consistency: 0.5,
        }
    }

    #[test]
    fn generate_config_within_bounds() {
        let mut rng = rand::thread_rng();
        let skill = medium_skill();
        for _ in 0..100 {
            let config = generate_drone_config(&skill, &[], &mut rng);
            assert!(config.pid_variation.x.abs() <= 0.15);
            assert!(config.pid_variation.y.abs() <= 0.15);
            assert!(config.pid_variation.z.abs() <= 0.15);
            assert!(config.line_offset.abs() <= 1.5);
            assert!((0.3..=1.5).contains(&config.noise_amplitude));
            assert!((0.5..=2.0).contains(&config.noise_frequency));
            assert!((0.05..=0.15).contains(&config.hover_noise_amp.x));
            assert!((0.02..=0.06).contains(&config.hover_noise_amp.y));
            assert!((0.05..=0.12).contains(&config.hover_noise_amp.z));
            assert!((0.1..=0.5).contains(&config.hover_noise_freq.x));
            assert!((0.15..=0.5).contains(&config.hover_noise_freq.y));
            assert!((0.1..=0.4).contains(&config.hover_noise_freq.z));
            assert!((0.8..=1.2).contains(&config.cornering_aggression));
            assert!((0.8..=1.2).contains(&config.braking_distance));
            assert!((0.9..=1.1).contains(&config.attitude_kp_mult));
            assert!((0.9..=1.1).contains(&config.attitude_kd_mult));
            assert!(config.racing_line_bias.abs() <= 4.5);
            assert!(
                (0.88..=1.12).contains(&config.approach_offset_scale),
                "approach_offset_scale {} out of range",
                config.approach_offset_scale
            );
            assert!(
                (0.18..=0.62).contains(&config.gate_pass_offset),
                "gate_pass_offset {} out of range",
                config.gate_pass_offset
            );
            assert!(
                (0.69..=1.31).contains(&config.maneuver_threshold_mult),
                "maneuver_threshold_mult {} out of range",
                config.maneuver_threshold_mult
            );
        }
    }

    #[test]
    fn high_skill_tighter_ranges() {
        let mut rng = rand::thread_rng();
        let low = SkillProfile {
            level: 0.1,
            speed: 0.1,
            cornering: 0.1,
            consistency: 0.1,
        };
        let high = SkillProfile {
            level: 0.95,
            speed: 0.95,
            cornering: 0.95,
            consistency: 0.95,
        };

        let iterations = 200;
        let mut low_noise_sum = 0.0f32;
        let mut high_noise_sum = 0.0f32;
        let mut low_pid_sum = 0.0f32;
        let mut high_pid_sum = 0.0f32;

        for _ in 0..iterations {
            let lc = generate_drone_config(&low, &[], &mut rng);
            let hc = generate_drone_config(&high, &[], &mut rng);
            low_noise_sum += lc.noise_amplitude;
            high_noise_sum += hc.noise_amplitude;
            low_pid_sum += lc.pid_variation.length();
            high_pid_sum += hc.pid_variation.length();
        }

        // High-skill pilots should have lower average noise
        let low_avg_noise = low_noise_sum / iterations as f32;
        let high_avg_noise = high_noise_sum / iterations as f32;
        assert!(
            high_avg_noise < low_avg_noise,
            "High skill avg noise ({high_avg_noise}) should be less than low skill ({low_avg_noise})"
        );

        // High-skill pilots should have smaller PID variation magnitude
        let low_avg_pid = low_pid_sum / iterations as f32;
        let high_avg_pid = high_pid_sum / iterations as f32;
        assert!(
            high_avg_pid < low_avg_pid,
            "High skill avg PID var ({high_avg_pid}) should be less than low skill ({low_avg_pid})"
        );
    }

    #[test]
    fn extreme_skill_levels_produce_valid_configs() {
        let mut rng = rand::thread_rng();
        for level in [0.0, 1.0] {
            let skill = SkillProfile {
                level,
                speed: level,
                cornering: level,
                consistency: level,
            };
            for _ in 0..50 {
                let config = generate_drone_config(&skill, &[], &mut rng);
                assert!(!config.noise_amplitude.is_nan());
                assert!(!config.cornering_aggression.is_nan());
                assert!(!config.pid_variation.x.is_nan());
                assert!((0.8..=1.2).contains(&config.cornering_aggression));
                assert!((0.3..=1.5).contains(&config.noise_amplitude));
            }
        }
    }

    #[test]
    fn traits_modify_config() {
        use crate::pilot::personality::PersonalityTrait;

        let mut rng = rand::thread_rng();
        let skill = medium_skill();
        let iterations = 200;

        let mut aggressive_sum = 0.0f32;
        let mut cautious_sum = 0.0f32;

        for _ in 0..iterations {
            let ac =
                generate_drone_config(&skill, &[PersonalityTrait::Aggressive], &mut rng);
            let cc = generate_drone_config(&skill, &[PersonalityTrait::Cautious], &mut rng);
            aggressive_sum += ac.cornering_aggression;
            cautious_sum += cc.cornering_aggression;
        }

        let aggressive_avg = aggressive_sum / iterations as f32;
        let cautious_avg = cautious_sum / iterations as f32;
        assert!(
            aggressive_avg > cautious_avg,
            "Aggressive avg cornering ({aggressive_avg}) should exceed Cautious ({cautious_avg})"
        );
    }
}
