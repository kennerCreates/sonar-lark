use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

use crate::common::POINTS_PER_GATE;
use crate::drone::ai::{cyclic_curvature, safe_speed_for_curvature};
use crate::drone::components::{AiTuningParams, DroneConfig};
use crate::pilot::personality::{PersonalityTrait, ScriptModifiers};
use crate::pilot::skill::SkillProfile;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Predetermined outcome for a single drone.
pub struct DroneScript {
    /// Per-segment pacing multiplier (one entry per gate-to-gate segment).
    /// >1.0 = faster than base curvature speed, <1.0 = slower.
    pub segment_pace: Vec<f32>,
    /// If Some, drone crashes near this gate.
    pub crash: Option<CrashScript>,
    /// Gate indices where this drone performs an acrobatic maneuver.
    pub acrobatic_gates: Vec<u32>,
    /// Per-gate spline_t values where this drone is closest to each gate's
    /// world-space position. Per-drone because each drone's racing line is unique.
    pub gate_pass_t: Vec<f32>,
}

#[derive(Clone)]
pub struct CrashScript {
    pub gate_index: u32,
    /// 0.0..1.0 within the turn — how far past the gate the crash occurs.
    pub progress_past_gate: f32,
    pub crash_type: ScriptedCrashType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptedCrashType {
    ObstacleCollision,
    DroneCollision { other_drone_idx: u8 },
}

/// A position swap between two drones at a known gate.
#[allow(dead_code)]
pub struct ScriptedOvertake {
    pub gate_index: u32,
    pub overtaker_idx: u8,
    pub overtaken_idx: u8,
}

/// Complete predetermined race outcome.
#[derive(Resource)]
pub struct RaceScript {
    pub drone_scripts: Vec<DroneScript>,
    /// Pre-computed overtake moments from the drama-pass simulation.
    pub overtakes: Vec<ScriptedOvertake>,
}

// ---------------------------------------------------------------------------
// Race event log (Phase 3 data, structure defined early for resource management)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum RaceEventKind {
    GatePass { drone_idx: u8, gate_index: u32 },
    Overtake { overtaker_idx: u8, overtaken_idx: u8, gate_index: u32 },
    Acrobatic { drone_idx: u8, gate_index: u32 },
    Crash { drone_idx: u8, crash_type: ScriptedCrashType },
    Finish { drone_idx: u8, time: f32 },
}

#[allow(dead_code)]
pub struct TimestampedEvent {
    pub race_time: f32,
    pub kind: RaceEventKind,
}

#[derive(Resource, Default)]
pub struct RaceEventLog {
    pub events: Vec<TimestampedEvent>,
}

// ---------------------------------------------------------------------------
// Input bundle for script generation
// ---------------------------------------------------------------------------

pub struct DroneScriptInput<'a> {
    pub spline: &'a CubicCurve<Vec3>,
    pub config: &'a DroneConfig,
    pub skill: SkillProfile,
    pub traits: Vec<PersonalityTrait>,
}

// ---------------------------------------------------------------------------
// Turn tightness classification
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TurnTightness {
    Gentle,
    Medium,
    Tight,
}

// ---------------------------------------------------------------------------
// Deterministic hash (Fibonacci hashing)
// ---------------------------------------------------------------------------

fn det_hash(seed: u32, a: u32, b: u32) -> u32 {
    let mut h = seed;
    h = h.wrapping_mul(2654435761).wrapping_add(a);
    h = h.wrapping_mul(2654435761).wrapping_add(b);
    h ^ (h >> 16)
}

/// Deterministic f32 in [0, 1) from a hash.
fn det_f32(seed: u32, a: u32, b: u32) -> f32 {
    (det_hash(seed, a, b) & 0x00FF_FFFF) as f32 / 16_777_216.0
}

/// Deterministic f32 in [lo, hi) from a hash.
fn det_range(seed: u32, a: u32, b: u32, lo: f32, hi: f32) -> f32 {
    lo + det_f32(seed, a, b) * (hi - lo)
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SIMULATION_DT: f32 = 1.0 / 64.0;
const CURVATURE_SAMPLES_PER_SEGMENT: usize = 8;
const MAX_DNFS: usize = 3;
const MIN_FINISHERS: usize = 4;
const MAX_ACROBATIC_GATES: usize = 5;
const PHOTO_FINISH_GAP_IDEAL: f32 = 2.0;
const PHOTO_FINISH_GAP_MAX: f32 = 5.0;
const MAX_TOP2_NUDGE: f32 = 0.05;
const MAX_MIDPACK_NUDGE: f32 = 0.03;
const PACK_GAP_THRESHOLD: f32 = 3.0;
const GATE_PASS_T_SAMPLES: usize = 12;

// ---------------------------------------------------------------------------
// Shared spline traversal step
// ---------------------------------------------------------------------------

/// One simulation tick: curvature → speed → dt_spline.
/// Returns (speed in world-units/s, dt_spline to advance spline_t by).
#[inline]
fn spline_step(
    spline: &CubicCurve<Vec3>,
    t: f32,
    cycle_t: f32,
    pace: f32,
    tuning: &AiTuningParams,
) -> (f32, f32) {
    let curvature = cyclic_curvature(spline, t, cycle_t);
    let base_speed = safe_speed_for_curvature(curvature, tuning);
    let speed = base_speed * pace;
    let tangent = spline.velocity(t.rem_euclid(cycle_t));
    let tangent_len = tangent.length().max(0.01);
    let dt_spline = speed * SIMULATION_DT / tangent_len;
    (speed, dt_spline)
}

// ---------------------------------------------------------------------------
// Step 1: Estimate base segment times
// ---------------------------------------------------------------------------

/// Dry-run spline traversal at curvature-based speeds (pace=1.0).
/// Returns (per-segment times in seconds, total time).
fn estimate_segment_times(
    spline: &CubicCurve<Vec3>,
    gate_count: u32,
    tuning: &AiTuningParams,
) -> (Vec<f32>, f32) {
    let cycle_t = gate_count as f32 * POINTS_PER_GATE;
    let total_spline_t = cycle_t + crate::common::FINISH_EXTENSION;
    let num_segments = gate_count as usize;
    let mut segment_times = vec![0.0f32; num_segments];

    let mut t = 0.0f32;
    let mut current_seg = 0usize;
    let mut seg_time = 0.0f32;

    while t < total_spline_t {
        let (_speed, dt_spline) = spline_step(spline, t, cycle_t, 1.0, tuning);

        t += dt_spline;
        seg_time += SIMULATION_DT;

        let new_seg = ((t / POINTS_PER_GATE).floor() as usize).min(num_segments - 1);
        if new_seg != current_seg && current_seg < num_segments {
            segment_times[current_seg] = seg_time;
            seg_time = 0.0;
            current_seg = new_seg;
        }
    }
    // Final segment
    if current_seg < num_segments {
        segment_times[current_seg] = seg_time;
    }

    let total = segment_times.iter().sum();
    (segment_times, total)
}

// ---------------------------------------------------------------------------
// Step 2: Compute per-drone per-gate turn tightness
// ---------------------------------------------------------------------------

pub(crate) fn classify_turn_tightness(
    spline: &CubicCurve<Vec3>,
    gate_count: u32,
    tuning: &AiTuningParams,
) -> Vec<TurnTightness> {
    let cycle_t = gate_count as f32 * POINTS_PER_GATE;

    // Derive thresholds from speed bands
    let speed_80 = tuning.max_speed * 0.8;
    let speed_50 = tuning.max_speed * 0.5;
    let kappa_gentle = tuning.safe_lateral_accel / (speed_80 * speed_80);
    let kappa_tight = tuning.safe_lateral_accel / (speed_50 * speed_50);

    let mut tightness = Vec::with_capacity(gate_count as usize);
    for gate_idx in 0..gate_count {
        let seg_start = gate_idx as f32 * POINTS_PER_GATE;
        let seg_end = seg_start + POINTS_PER_GATE;
        let mut peak_curvature = 0.0f32;

        for i in 0..CURVATURE_SAMPLES_PER_SEGMENT {
            let frac = i as f32 / (CURVATURE_SAMPLES_PER_SEGMENT - 1).max(1) as f32;
            let sample_t = seg_start + frac * (seg_end - seg_start);
            peak_curvature = peak_curvature.max(cyclic_curvature(spline, sample_t, cycle_t));
        }

        let class = if peak_curvature < kappa_gentle {
            TurnTightness::Gentle
        } else if peak_curvature < kappa_tight {
            TurnTightness::Medium
        } else {
            TurnTightness::Tight
        };
        tightness.push(class);
    }
    tightness
}

// ---------------------------------------------------------------------------
// Pre-compute per-gate spline_t offsets
// ---------------------------------------------------------------------------

fn compute_gate_pass_t(
    spline: &CubicCurve<Vec3>,
    gate_count: u32,
    gate_positions: &[Vec3],
) -> Vec<f32> {
    let cycle_t = gate_count as f32 * POINTS_PER_GATE;
    let mut pass_t_values = Vec::with_capacity(gate_count as usize);

    for (gate_idx, &gate_pos) in gate_positions.iter().enumerate().take(gate_count as usize) {
        let seg_start = gate_idx as f32 * POINTS_PER_GATE;
        let seg_end = seg_start + POINTS_PER_GATE;

        let mut best_t = seg_start + POINTS_PER_GATE * 0.5;
        let mut best_dist_sq = f32::MAX;

        for i in 0..GATE_PASS_T_SAMPLES {
            let frac = i as f32 / (GATE_PASS_T_SAMPLES - 1).max(1) as f32;
            let sample_t = seg_start + frac * (seg_end - seg_start);
            let pos = spline.position(sample_t.rem_euclid(cycle_t));
            let dist_sq = pos.distance_squared(gate_pos);
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_t = sample_t;
            }
        }

        pass_t_values.push(best_t);
    }

    pass_t_values
}

// ---------------------------------------------------------------------------
// Personality modifiers (aggregated from per-trait ScriptModifiers)
// ---------------------------------------------------------------------------

fn aggregate_script_modifiers(traits: &[PersonalityTrait]) -> ScriptModifiers {
    let mut result = ScriptModifiers {
        risk_factor: 1.0,
        straight_line_bonus: 0.0,
        cornering_bonus: 0.0,
    };
    for t in traits {
        let m = t.script_modifiers();
        result.risk_factor *= m.risk_factor;
        result.straight_line_bonus += m.straight_line_bonus;
        result.cornering_bonus += m.cornering_bonus;
    }
    result
}

/// True if the drone is eligible for acrobatics based on personality.
fn is_acrobatic_personality(traits: &[PersonalityTrait]) -> bool {
    traits.iter().any(|t| {
        matches!(
            t,
            PersonalityTrait::Flashy | PersonalityTrait::Hotdog | PersonalityTrait::Aggressive
        )
    })
}

// ---------------------------------------------------------------------------
// Simulate the race with given segment paces
// ---------------------------------------------------------------------------

/// Returns per-drone finish times and per-drone per-gate arrival times.
fn simulate_race(
    drones: &[DroneScriptInput],
    segment_paces: &[Vec<f32>],
    gate_pass_t_all: &[Vec<f32>],
    crashes: &[Option<CrashScript>],
    tuning: &AiTuningParams,
) -> (Vec<f32>, Vec<Vec<f32>>) {
    let drone_count = drones.len();
    if drone_count == 0 {
        return (Vec::new(), Vec::new());
    }
    let gate_count = segment_paces[0].len() as u32;
    let cycle_t = gate_count as f32 * POINTS_PER_GATE;
    let finish_t = cycle_t + crate::common::FINISH_EXTENSION;

    let mut spline_ts = vec![0.0f32; drone_count];
    let mut finish_times = vec![f32::MAX; drone_count];
    let mut gate_arrival_times: Vec<Vec<f32>> =
        vec![vec![f32::MAX; gate_count as usize]; drone_count];
    let mut sim_time = 0.0f32;

    let max_sim_ticks = ((finish_t / SIMULATION_DT) * 3.0) as usize;

    for _tick in 0..max_sim_ticks {
        sim_time += SIMULATION_DT;
        let mut all_done = true;

        for d in 0..drone_count {
            if finish_times[d] < f32::MAX {
                continue;
            }

            // Check if crashed before this point
            if let Some(ref crash) = crashes[d] {
                let crash_t = gate_pass_t_all[d][crash.gate_index as usize]
                    + crash.progress_past_gate * POINTS_PER_GATE;
                if spline_ts[d] >= crash_t {
                    finish_times[d] = sim_time;
                    continue;
                }
            }

            let seg_idx = ((spline_ts[d] / POINTS_PER_GATE).floor() as usize)
                .min(gate_count as usize - 1);
            let (_speed, dt_spline) =
                spline_step(drones[d].spline, spline_ts[d], cycle_t, segment_paces[d][seg_idx], tuning);

            let old_t = spline_ts[d];
            spline_ts[d] += dt_spline;

            // Record gate arrivals
            for g in 0..gate_count as usize {
                if gate_arrival_times[d][g] == f32::MAX
                    && old_t < gate_pass_t_all[d][g]
                    && spline_ts[d] >= gate_pass_t_all[d][g]
                {
                    gate_arrival_times[d][g] = sim_time;
                }
            }

            if spline_ts[d] >= finish_t {
                finish_times[d] = sim_time;
            } else {
                all_done = false;
            }
        }

        if all_done {
            break;
        }
    }

    (finish_times, gate_arrival_times)
}

// ---------------------------------------------------------------------------
// Detect overtakes from gate arrival times
// ---------------------------------------------------------------------------

#[allow(clippy::needless_range_loop)]
fn detect_overtakes(
    gate_arrival_times: &[Vec<f32>],
    gate_count: u32,
) -> Vec<ScriptedOvertake> {
    let drone_count = gate_arrival_times.len();
    if drone_count < 2 || gate_count < 2 {
        return Vec::new();
    }

    let mut overtakes = Vec::new();

    // Build position order at each gate
    let mut prev_order: Vec<usize> = (0..drone_count).collect();
    prev_order.sort_by(|&a, &b| {
        gate_arrival_times[a][0]
            .partial_cmp(&gate_arrival_times[b][0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for g in 1..gate_count as usize {
        let mut curr_order: Vec<usize> = (0..drone_count).collect();
        curr_order.sort_by(|&a, &b| {
            gate_arrival_times[a][g]
                .partial_cmp(&gate_arrival_times[b][g])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Detect position swaps
        for (pos, &drone_idx) in curr_order.iter().enumerate() {
            let prev_pos = prev_order.iter().position(|&d| d == drone_idx).unwrap();
            if pos < prev_pos {
                // This drone gained positions — find who it overtook
                for &overtaken_drone in &prev_order[pos..prev_pos] {
                    if overtaken_drone != drone_idx {
                        overtakes.push(ScriptedOvertake {
                            gate_index: g as u32,
                            overtaker_idx: drone_idx as u8,
                            overtaken_idx: overtaken_drone as u8,
                        });
                    }
                }
            }
        }

        prev_order = curr_order;
    }

    overtakes
}

// ---------------------------------------------------------------------------
// Main entry point: generate_race_script()
// ---------------------------------------------------------------------------

pub fn generate_race_script(
    gate_count: u32,
    gate_positions: &[Vec3],
    drones: &[DroneScriptInput],
    race_seed: u32,
    tuning: &AiTuningParams,
) -> RaceScript {
    let drone_count = drones.len();

    if gate_count == 0 || drone_count == 0 {
        return RaceScript {
            drone_scripts: Vec::new(),
            overtakes: Vec::new(),
        };
    }

    // -----------------------------------------------------------------------
    // Step 0: Pre-compute per-gate spline_t offsets for each drone
    // -----------------------------------------------------------------------
    let gate_pass_t_all: Vec<Vec<f32>> = drones
        .iter()
        .map(|d| compute_gate_pass_t(d.spline, gate_count, gate_positions))
        .collect();

    // -----------------------------------------------------------------------
    // Step 1: Estimate base segment times for each drone
    // -----------------------------------------------------------------------
    let mut estimated_times: Vec<f32> = Vec::with_capacity(drone_count);
    let mut all_segment_times: Vec<Vec<f32>> = Vec::with_capacity(drone_count);
    for d in drones {
        let (seg_times, total) = estimate_segment_times(d.spline, gate_count, tuning);
        all_segment_times.push(seg_times);
        estimated_times.push(total);
    }

    // -----------------------------------------------------------------------
    // Step 2: Compute per-drone per-gate turn tightness
    // -----------------------------------------------------------------------
    let tightness_all: Vec<Vec<TurnTightness>> = drones
        .iter()
        .map(|d| classify_turn_tightness(d.spline, gate_count, tuning))
        .collect();

    // -----------------------------------------------------------------------
    // Step 3: Assign finish order and target times
    // -----------------------------------------------------------------------
    let mut target_finish_times: Vec<f32> = Vec::with_capacity(drone_count);
    for (i, est) in estimated_times.iter().enumerate() {
        let consistency = drones[i].skill.consistency;
        // ±5-15% perturbation inversely proportional to consistency
        let max_perturbation = 0.05 + 0.10 * (1.0 - consistency);
        let perturbation = det_range(race_seed, i as u32, 1000, -max_perturbation, max_perturbation);
        target_finish_times.push(est * (1.0 + perturbation));
    }

    // -----------------------------------------------------------------------
    // Step 4: Assign DNFs (0-3 per race, minimum 4 finishers)
    // -----------------------------------------------------------------------
    let num_segments = gate_count as usize;
    let mut crashes: Vec<Option<CrashScript>> = vec![None; drone_count];
    let mut crash_count = 0usize;

    // Compute per-drone difficulty and crash probability
    let mut crash_candidates: Vec<(usize, f32, u32)> = Vec::new(); // (drone_idx, probability, gate)
    for (d_idx, tightness) in tightness_all.iter().enumerate() {
        let tight_count = tightness
            .iter()
            .filter(|&&t| t == TurnTightness::Tight)
            .count();
        let drone_difficulty = tight_count as f32 / gate_count as f32;
        let mods = aggregate_script_modifiers(&drones[d_idx].traits);
        let crash_prob = (1.0 - drones[d_idx].skill.level) * drone_difficulty * mods.risk_factor;

        let roll = det_f32(race_seed, d_idx as u32, 2000);
        if roll < crash_prob {
            // Pick a tight-turn gate for the crash
            let tight_gates: Vec<u32> = tightness
                .iter()
                .enumerate()
                .filter(|(_, t)| **t == TurnTightness::Tight)
                .map(|(i, _)| i as u32)
                .collect();
            if !tight_gates.is_empty() {
                // Pick a gate from the tight gates using deterministic selection
                let gate_idx =
                    det_hash(race_seed, d_idx as u32, 2001) as usize % tight_gates.len();
                crash_candidates.push((d_idx, crash_prob, tight_gates[gate_idx]));
            }
        }
    }

    // Sort by crash probability (most dramatic first) and cap
    crash_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for &(d_idx, _, gate) in &crash_candidates {
        if crash_count >= MAX_DNFS {
            break;
        }
        if drone_count - crash_count <= MIN_FINISHERS {
            break;
        }
        let progress = det_range(race_seed, d_idx as u32, 2002, 0.1, 0.8);
        crashes[d_idx] = Some(CrashScript {
            gate_index: gate,
            progress_past_gate: progress,
            crash_type: ScriptedCrashType::ObstacleCollision,
        });
        crash_count += 1;
    }

    // -----------------------------------------------------------------------
    // Step 5: Assign acrobatics
    // -----------------------------------------------------------------------
    let mut acrobatic_gates_all: Vec<Vec<u32>> = vec![Vec::new(); drone_count];
    for (d_idx, tightness) in tightness_all.iter().enumerate() {
        if crashes[d_idx].is_some() {
            continue;
        }
        let skill = &drones[d_idx].skill;
        let is_acrobatic = is_acrobatic_personality(&drones[d_idx].traits) || skill.cornering > 0.85;
        if !is_acrobatic || skill.cornering <= 0.6 {
            continue;
        }

        let mut acro_gates = Vec::new();
        for (g_idx, &tight) in tightness.iter().enumerate() {
            if tight == TurnTightness::Tight && acro_gates.len() < MAX_ACROBATIC_GATES {
                acro_gates.push(g_idx as u32);
            }
        }
        acrobatic_gates_all[d_idx] = acro_gates;
    }

    // -----------------------------------------------------------------------
    // Step 6: Compute per-segment pace profiles
    // -----------------------------------------------------------------------
    let mut segment_paces: Vec<Vec<f32>> = Vec::with_capacity(drone_count);
    for d_idx in 0..drone_count {
        let skill = &drones[d_idx].skill;
        let config = drones[d_idx].config;

        // Derived attributes
        let mods = aggregate_script_modifiers(&drones[d_idx].traits);
        let s_speed = (skill.level * 0.4 + skill.speed * 0.6).clamp(0.0, 1.0);
        let straight_line_factor =
            1.0 + (s_speed - 0.5) * 0.15 + mods.straight_line_bonus;

        let s_corner = (skill.level * 0.4 + skill.cornering * 0.6).clamp(0.0, 1.0);
        let cornering_efficiency = config.cornering_aggression
            * (0.85 + s_corner * 0.30)
            + mods.cornering_bonus;

        let base_pace = if target_finish_times[d_idx] > 0.001 {
            estimated_times[d_idx] / target_finish_times[d_idx]
        } else {
            1.0
        };

        let mut paces = Vec::with_capacity(num_segments);
        for (seg_idx, &tightness) in tightness_all[d_idx].iter().enumerate().take(num_segments) {
            let consistency_noise = {
                let noise_range = 0.10 * (1.0 - skill.consistency);
                det_range(race_seed, d_idx as u32, seg_idx as u32, -noise_range, noise_range)
            };

            let pace = match tightness {
                TurnTightness::Gentle => base_pace * straight_line_factor,
                TurnTightness::Medium | TurnTightness::Tight => {
                    base_pace * cornering_efficiency * (1.0 + consistency_noise)
                }
            };
            paces.push(pace.max(0.3)); // Floor: never go below 30% speed
        }

        // For DNF drones, pace is computed normally (they fly until crash point)

        // Normalization pass: adjust paces so total time matches target
        let (sim_times, sim_total) = simulate_paced_time(
            drones[d_idx].spline,
            gate_count,
            &paces,
            crashes[d_idx].as_ref().map(|c| {
                gate_pass_t_all[d_idx][c.gate_index as usize]
                    + c.progress_past_gate * POINTS_PER_GATE
            }),
            tuning,
        );
        let _ = sim_times; // unused but available for debugging

        let target = if crashes[d_idx].is_some() {
            // For crashed drones, just use the simulated time (no target to hit)
            sim_total
        } else {
            target_finish_times[d_idx]
        };

        if sim_total > 0.001 && crashes[d_idx].is_none() {
            let correction = target / sim_total;
            for p in &mut paces {
                *p *= correction;
            }
        }

        segment_paces.push(paces);
    }

    // -----------------------------------------------------------------------
    // Step 7: Drama pass — photo finishes and mid-pack clustering
    // -----------------------------------------------------------------------
    let (overtakes, final_gate_arrivals) = apply_drama_pass(
        drones, &mut segment_paces, &gate_pass_t_all, &crashes, tuning, gate_count,
    );

    // -----------------------------------------------------------------------
    // Step 8: Drone-on-drone crash assignment (after drama pass)
    // -----------------------------------------------------------------------
    assign_drone_collisions(
        drones, &mut crashes, &mut crash_count,
        &gate_pass_t_all, &final_gate_arrivals, gate_count, race_seed,
    );

    // -----------------------------------------------------------------------
    // Assemble RaceScript
    // -----------------------------------------------------------------------
    let drone_scripts = (0..drone_count)
        .map(|d| DroneScript {
            segment_pace: segment_paces[d].clone(),
            crash: crashes.get(d).and_then(|c| {
                c.as_ref().map(|cs| CrashScript {
                    gate_index: cs.gate_index,
                    progress_past_gate: cs.progress_past_gate,
                    crash_type: cs.crash_type,
                })
            }),
            acrobatic_gates: acrobatic_gates_all[d].clone(),
            gate_pass_t: gate_pass_t_all[d].clone(),
        })
        .collect();

    RaceScript {
        drone_scripts,
        overtakes,
    }
}

// ---------------------------------------------------------------------------
// Step 7: Drama pass
// ---------------------------------------------------------------------------

/// Tighten photo finish gap and cluster mid-pack drones.
/// Mutates `segment_paces` in place. Returns (overtakes, final_gate_arrivals).
fn apply_drama_pass(
    drones: &[DroneScriptInput],
    segment_paces: &mut [Vec<f32>],
    gate_pass_t_all: &[Vec<f32>],
    crashes: &[Option<CrashScript>],
    tuning: &AiTuningParams,
    gate_count: u32,
) -> (Vec<ScriptedOvertake>, Vec<Vec<f32>>) {
    let num_segments = gate_count as usize;

    // Simulate full race to get finish times and gate arrivals
    let (sim_finish_times, _gate_arrival_times) =
        simulate_race(drones, segment_paces, gate_pass_t_all, crashes, tuning);

    // Sort by finish time to find top-2
    let mut finish_order: Vec<(usize, f32)> = sim_finish_times
        .iter()
        .enumerate()
        .filter(|(d, _)| crashes[*d].is_none())
        .map(|(d, &t)| (d, t))
        .collect();
    finish_order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Photo finish: tighten gap between top 2
    if finish_order.len() >= 2 {
        let (first_idx, first_time) = finish_order[0];
        let (second_idx, second_time) = finish_order[1];
        let gap = second_time - first_time;

        if gap > PHOTO_FINISH_GAP_IDEAL
            && gap <= PHOTO_FINISH_GAP_MAX
            && drones[first_idx].skill.level > 0.5
            && drones[second_idx].skill.level > 0.5
        {
            let segments_to_nudge = 3.min(num_segments);
            let nudge_per_seg = (MAX_TOP2_NUDGE / segments_to_nudge as f32).min(MAX_TOP2_NUDGE);
            for p in &mut segment_paces[second_idx][(num_segments - segments_to_nudge)..num_segments] {
                *p *= 1.0 + nudge_per_seg;
            }
        }
    }

    // Mid-pack clustering: identify isolated drones and nudge toward packs
    if finish_order.len() > 4 {
        let mut nudge_count = 0;
        let order_snapshot: Vec<(usize, f32)> = finish_order.clone();

        for rank in 1..order_snapshot.len().saturating_sub(1) {
            if nudge_count >= 4 {
                break;
            }
            let (d_idx, time) = order_snapshot[rank];
            let gap_ahead = time - order_snapshot[rank - 1].1;
            let gap_behind = order_snapshot[rank + 1].1 - time;

            if gap_ahead > PACK_GAP_THRESHOLD && gap_behind > PACK_GAP_THRESHOLD {
                let segments_to_nudge = 2.min(num_segments);
                for p in &mut segment_paces[d_idx][(num_segments - segments_to_nudge)..num_segments] {
                    *p *= 1.0 + MAX_MIDPACK_NUDGE;
                }
                nudge_count += 1;
            }
        }
    }

    // Re-simulate after drama adjustments to get final overtakes
    let (_, final_gate_arrivals) =
        simulate_race(drones, segment_paces, gate_pass_t_all, crashes, tuning);

    let overtakes = detect_overtakes(&final_gate_arrivals, gate_count);
    (overtakes, final_gate_arrivals)
}

// ---------------------------------------------------------------------------
// Step 8: Drone-on-drone crash assignment
// ---------------------------------------------------------------------------

/// Assign drone-on-drone collisions based on proximity at gates.
/// Mutates `crashes` and `crash_count` in place.
fn assign_drone_collisions(
    drones: &[DroneScriptInput],
    crashes: &mut [Option<CrashScript>],
    crash_count: &mut usize,
    gate_pass_t_all: &[Vec<f32>],
    final_gate_arrivals: &[Vec<f32>],
    gate_count: u32,
    race_seed: u32,
) {
    let drone_count = drones.len();
    let cycle_t = gate_count as f32 * POINTS_PER_GATE;

    // Find pairs of drones that arrive at the same gate within 1s of each other
    // and whose spline positions are within 3m at that gate.
    let mut collision_candidates: Vec<(usize, usize, u32, f32)> = Vec::new();
    for g in 0..gate_count as usize {
        for d1 in 0..drone_count {
            if crashes[d1].is_some() {
                continue;
            }
            for d2 in (d1 + 1)..drone_count {
                if crashes[d2].is_some() {
                    continue;
                }
                let t1 = final_gate_arrivals[d1][g];
                let t2 = final_gate_arrivals[d2][g];
                if (t1 - t2).abs() > 1.0 || t1 == f32::MAX || t2 == f32::MAX {
                    continue;
                }
                let pos1 = drones[d1]
                    .spline
                    .position(gate_pass_t_all[d1][g].rem_euclid(cycle_t));
                let pos2 = drones[d2]
                    .spline
                    .position(gate_pass_t_all[d2][g].rem_euclid(cycle_t));
                let dist = (pos1 - pos2).length();
                if dist < 3.0 {
                    collision_candidates.push((d1, d2, g as u32, dist));
                }
            }
        }
    }

    // Sort by proximity (closest = most dramatic)
    collision_candidates
        .sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));

    for &(d1, d2, gate, _) in &collision_candidates {
        if *crash_count >= MAX_DNFS {
            break;
        }
        if drone_count - *crash_count <= MIN_FINISHERS {
            break;
        }
        if crashes[d1].is_some() || crashes[d2].is_some() {
            continue;
        }
        // Weaker drone crashes; stronger survives
        let (crasher, survivor) = if drones[d1].skill.level <= drones[d2].skill.level {
            (d1, d2)
        } else {
            (d2, d1)
        };
        let progress = det_range(race_seed, crasher as u32, 3000 + gate, 0.1, 0.5);
        crashes[crasher] = Some(CrashScript {
            gate_index: gate,
            progress_past_gate: progress,
            crash_type: ScriptedCrashType::DroneCollision {
                other_drone_idx: survivor as u8,
            },
        });
        *crash_count += 1;
    }

    // Re-check minimum finishers after drone-on-drone crashes
    while drone_count - *crash_count < MIN_FINISHERS && *crash_count > 0 {
        if let Some(pos) = crashes.iter().rposition(|c| {
            c.as_ref()
                .is_some_and(|cs| matches!(cs.crash_type, ScriptedCrashType::DroneCollision { .. }))
        }) {
            crashes[pos] = None;
            *crash_count -= 1;
        } else {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: simulate paced traversal time
// ---------------------------------------------------------------------------

/// Dry-run spline traversal with per-segment pace multipliers.
/// Returns (per-segment times, total time). Stops at crash_t if Some.
fn simulate_paced_time(
    spline: &CubicCurve<Vec3>,
    gate_count: u32,
    paces: &[f32],
    crash_t: Option<f32>,
    tuning: &AiTuningParams,
) -> (Vec<f32>, f32) {
    let cycle_t = gate_count as f32 * POINTS_PER_GATE;
    let finish_t = crash_t.unwrap_or(cycle_t + crate::common::FINISH_EXTENSION);
    let num_segments = gate_count as usize;
    let mut segment_times = vec![0.0f32; num_segments];

    let mut t = 0.0f32;
    let mut current_seg = 0usize;
    let mut seg_time = 0.0f32;
    let mut total_time = 0.0f32;

    let max_ticks = ((finish_t / SIMULATION_DT) * 3.0) as usize;
    for _ in 0..max_ticks {
        if t >= finish_t {
            break;
        }

        let seg_idx = ((t / POINTS_PER_GATE).floor() as usize).min(num_segments - 1);
        let (_speed, dt_spline) = spline_step(spline, t, cycle_t, paces[seg_idx], tuning);

        t += dt_spline;
        seg_time += SIMULATION_DT;
        total_time += SIMULATION_DT;

        let new_seg = ((t / POINTS_PER_GATE).floor() as usize).min(num_segments - 1);
        if new_seg != current_seg && current_seg < num_segments {
            segment_times[current_seg] = seg_time;
            seg_time = 0.0;
            current_seg = new_seg;
        }
    }
    if current_seg < num_segments {
        segment_times[current_seg] = seg_time;
    }

    (segment_times, total_time)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::cubic_splines::CubicCardinalSpline;

    fn default_tuning() -> AiTuningParams {
        AiTuningParams::default()
    }

    fn default_skill() -> SkillProfile {
        SkillProfile {
            level: 0.5,
            speed: 0.5,
            cornering: 0.5,
            consistency: 0.5,
        }
    }

    fn default_config() -> DroneConfig {
        DroneConfig {
            pid_variation: Vec3::ZERO,
            line_offset: 0.0,
            noise_amplitude: 0.5,
            noise_frequency: 1.0,
            hover_noise_amp: Vec3::new(0.08, 0.03, 0.07),
            hover_noise_freq: Vec3::new(0.2, 0.25, 0.2),
            cornering_aggression: 1.0,
            braking_distance: 1.0,
            attitude_kp_mult: 1.0,
            attitude_kd_mult: 1.0,
            racing_line_bias: 0.0,
            approach_offset_scale: 1.0,
            gate_pass_offset: 0.35,
        }
    }

    /// Build a simple oval track with `n` gates spread in a circle.
    fn make_oval_course(n: usize, radius: f32) -> (Vec<Vec3>, Vec<Vec3>, CubicCurve<Vec3>) {
        let positions: Vec<Vec3> = (0..n)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / n as f32;
                Vec3::new(radius * angle.cos(), 2.0, radius * angle.sin())
            })
            .collect();

        let forwards: Vec<Vec3> = (0..n)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / n as f32;
                Vec3::new(-angle.sin(), 0.0, angle.cos())
            })
            .collect();

        // Build a simple cyclic spline through gate positions
        let spline = CubicCardinalSpline::new_catmull_rom(positions.clone())
            .to_curve_cyclic()
            .expect("failed to build test spline");

        (positions, forwards, spline)
    }

    #[test]
    fn empty_inputs_return_empty_script() {
        let tuning = default_tuning();
        let script = generate_race_script(0, &[], &[], 42, &tuning);
        assert!(script.drone_scripts.is_empty());
        assert!(script.overtakes.is_empty());
    }

    #[test]
    fn basic_script_generation() {
        let tuning = default_tuning();
        let (positions, _forwards, spline) = make_oval_course(6, 30.0);
        let config = default_config();
        let skill = default_skill();

        let drones: Vec<DroneScriptInput> = (0..4)
            .map(|_| DroneScriptInput {
                spline: &spline,
                config: &config,
                skill: skill.clone(),
                traits: vec![],
            })
            .collect();

        let script = generate_race_script(6, &positions, &drones, 42, &tuning);

        assert_eq!(script.drone_scripts.len(), 4);
        for ds in &script.drone_scripts {
            assert_eq!(ds.segment_pace.len(), 6);
            assert_eq!(ds.gate_pass_t.len(), 6);
            for &p in &ds.segment_pace {
                assert!(p > 0.0, "pace must be positive, got {p}");
            }
        }
    }

    #[test]
    fn skilled_drones_finish_faster_on_average() {
        let tuning = default_tuning();
        let (positions, _forwards, spline) = make_oval_course(6, 30.0);
        let config = default_config();

        let high_skill = SkillProfile {
            level: 0.9,
            speed: 0.9,
            cornering: 0.9,
            consistency: 0.9,
        };
        let low_skill = SkillProfile {
            level: 0.2,
            speed: 0.2,
            cornering: 0.2,
            consistency: 0.2,
        };

        // Run multiple seeds and average
        let mut high_sum = 0.0f32;
        let mut low_sum = 0.0f32;
        let trials = 20;

        for seed in 0..trials {
            let drones = vec![
                DroneScriptInput {
                    spline: &spline,
                    config: &config,
                    skill: high_skill.clone(),
                    traits: vec![],
                },
                DroneScriptInput {
                    spline: &spline,
                    config: &config,
                    skill: low_skill.clone(),
                    traits: vec![],
                },
            ];
            let script = generate_race_script(6, &positions, &drones, seed, &tuning);

            // Simulate to get actual finish times
            let gate_pass_t_all: Vec<Vec<f32>> = drones
                .iter()
                .map(|d| compute_gate_pass_t(d.spline, 6, &positions))
                .collect();
            let paces: Vec<Vec<f32>> = script
                .drone_scripts
                .iter()
                .map(|ds| ds.segment_pace.clone())
                .collect();
            let crashes: Vec<Option<CrashScript>> = vec![None, None];
            let (finish_times, _) =
                simulate_race(&drones, &paces, &gate_pass_t_all, &crashes, &tuning);

            high_sum += finish_times[0];
            low_sum += finish_times[1];
        }

        let high_avg = high_sum / trials as f32;
        let low_avg = low_sum / trials as f32;
        assert!(
            high_avg < low_avg,
            "High-skill avg time ({high_avg:.1}s) should be less than low-skill ({low_avg:.1}s)"
        );
    }

    #[test]
    fn crash_count_within_bounds() {
        let tuning = default_tuning();
        let (positions, _forwards, spline) = make_oval_course(8, 20.0);
        let config = default_config();

        let reckless_skill = SkillProfile {
            level: 0.1,
            speed: 0.1,
            cornering: 0.1,
            consistency: 0.1,
        };

        let mut total_crashes = 0;
        let trials = 50;
        for seed in 0..trials {
            let drones: Vec<DroneScriptInput> = (0..12)
                .map(|_| DroneScriptInput {
                    spline: &spline,
                    config: &config,
                    skill: reckless_skill.clone(),
                    traits: vec![PersonalityTrait::Reckless],
                })
                .collect();

            let script = generate_race_script(8, &positions, &drones, seed, &tuning);

            let crashes = script
                .drone_scripts
                .iter()
                .filter(|ds| ds.crash.is_some())
                .count();
            assert!(
                crashes <= MAX_DNFS,
                "Seed {seed}: got {crashes} crashes, max is {MAX_DNFS}"
            );
            let finishers = 12 - crashes;
            assert!(
                finishers >= MIN_FINISHERS,
                "Seed {seed}: only {finishers} finishers, need >= {MIN_FINISHERS}"
            );
            total_crashes += crashes;
        }

        // At least some races should have crashes with reckless unskilled pilots
        assert!(
            total_crashes > 0,
            "Expected some crashes across {trials} races with reckless unskilled pilots"
        );
    }

    #[test]
    fn acrobatics_only_for_skilled_drones() {
        let tuning = default_tuning();
        // Tight course to ensure Tight turn classifications
        let (positions, _forwards, spline) = make_oval_course(8, 12.0);
        let config = default_config();

        let high_cornering = SkillProfile {
            level: 0.8,
            speed: 0.8,
            cornering: 0.9,
            consistency: 0.8,
        };
        let low_cornering = SkillProfile {
            level: 0.3,
            speed: 0.3,
            cornering: 0.3,
            consistency: 0.3,
        };

        let drones = vec![
            DroneScriptInput {
                spline: &spline,
                config: &config,
                skill: high_cornering,
                traits: vec![PersonalityTrait::Flashy],
            },
            DroneScriptInput {
                spline: &spline,
                config: &config,
                skill: low_cornering,
                traits: vec![PersonalityTrait::Cautious],
            },
        ];

        let script = generate_race_script(8, &positions, &drones, 42, &tuning);

        // Low-cornering cautious drone should not have acrobatics
        assert!(
            script.drone_scripts[1].acrobatic_gates.is_empty(),
            "Low-skill cautious drone should not have acrobatic gates"
        );
    }

    #[test]
    fn gate_pass_t_values_are_within_segment() {
        let (positions, _forwards, spline) = make_oval_course(6, 30.0);
        let gate_pass_t = compute_gate_pass_t(&spline, 6, &positions);

        for (g, &t) in gate_pass_t.iter().enumerate() {
            let seg_start = g as f32 * POINTS_PER_GATE;
            let seg_end = seg_start + POINTS_PER_GATE;
            assert!(
                t >= seg_start && t <= seg_end,
                "Gate {g} pass_t {t} outside [{seg_start}, {seg_end}]"
            );
        }
    }

    #[test]
    fn segment_paces_are_positive() {
        let tuning = default_tuning();
        let (positions, _forwards, spline) = make_oval_course(6, 30.0);
        let config = default_config();
        let skill = default_skill();

        for seed in 0..20u32 {
            let drones: Vec<DroneScriptInput> = (0..8)
                .map(|_| DroneScriptInput {
                    spline: &spline,
                    config: &config,
                    skill: skill.clone(),
                    traits: vec![],
                })
                .collect();

            let script = generate_race_script(6, &positions, &drones, seed, &tuning);
            for (d, ds) in script.drone_scripts.iter().enumerate() {
                for (s, &p) in ds.segment_pace.iter().enumerate() {
                    assert!(
                        p > 0.0 && p.is_finite(),
                        "Seed {seed}, drone {d}, segment {s}: invalid pace {p}"
                    );
                }
            }
        }
    }

    #[test]
    fn det_hash_is_deterministic() {
        let a = det_hash(42, 1, 2);
        let b = det_hash(42, 1, 2);
        assert_eq!(a, b);

        let c = det_hash(42, 1, 3);
        assert_ne!(a, c);
    }

    #[test]
    fn det_f32_in_range() {
        for i in 0..100 {
            let v = det_f32(42, i, 0);
            assert!(v >= 0.0 && v < 1.0, "det_f32 out of range: {v}");
        }
    }
}
