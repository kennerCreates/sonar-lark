use bevy::prelude::*;
use std::cmp::Ordering;

use crate::drone::components::{AIController, Drone, DronePhase};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DnfReason {
    MissedGate(u32),
    ObstacleCollision,
    DroneCollision,
    GroundCollision,
}

#[derive(Clone, Debug)]
pub struct DroneRaceState {
    pub next_gate: u32,
    pub gates_passed: u32,
    pub finished: bool,
    pub finish_time: Option<f32>,
    pub crashed: bool,
    pub dnf_reason: Option<DnfReason>,
    /// Continuous progress along the race spline, synced from AIController each frame.
    /// Used as a tiebreaker in standings when gates_passed is equal.
    pub spline_t: f32,
}

impl Default for DroneRaceState {
    fn default() -> Self {
        Self {
            next_gate: 0,
            gates_passed: 0,
            finished: false,
            finish_time: None,
            crashed: false,
            dnf_reason: None,
            spline_t: 0.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct RaceProgress {
    pub drone_states: Vec<DroneRaceState>,
    pub total_gates: u32,
}

impl RaceProgress {
    pub fn record_gate_pass(&mut self, drone_index: usize, gate_index: u32) {
        let Some(state) = self.drone_states.get_mut(drone_index) else { return };
        if state.crashed || state.finished {
            return;
        }
        state.gates_passed += 1;
        state.next_gate = gate_index + 1;
    }

    pub fn record_finish(&mut self, drone_index: usize, time: f32) {
        let Some(state) = self.drone_states.get_mut(drone_index) else { return };
        if state.crashed || state.finished {
            return;
        }
        state.finished = true;
        state.finish_time = Some(time);
    }

    pub fn record_crash(&mut self, drone_index: usize, reason: DnfReason) {
        let Some(state) = self.drone_states.get_mut(drone_index) else { return };
        if state.crashed || state.finished {
            return;
        }
        state.crashed = true;
        state.dnf_reason = Some(reason);
    }

    /// Returns true if at least one drone has finished the race.
    pub fn any_finished(&self) -> bool {
        self.drone_states.iter().any(|s| s.finished)
    }

    #[allow(dead_code)]
    pub fn is_active(&self, drone_index: usize) -> bool {
        self.drone_states
            .get(drone_index)
            .is_some_and(|s| !s.crashed && !s.finished)
    }

    /// Returns standings sorted: finished drones by time (ascending), then
    /// active/crashed drones by gates_passed (descending).
    pub fn standings(&self) -> Vec<(usize, &DroneRaceState)> {
        let mut entries: Vec<(usize, &DroneRaceState)> =
            self.drone_states.iter().enumerate().collect();
        entries.sort_by(|(_, a), (_, b)| {
            match (a.finished, b.finished) {
                (true, true) => {
                    a.finish_time
                        .partial_cmp(&b.finish_time)
                        .unwrap_or(Ordering::Equal)
                }
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => b.gates_passed.cmp(&a.gates_passed).then(
                    b.spline_t
                        .partial_cmp(&a.spline_t)
                        .unwrap_or(Ordering::Equal),
                ),
            }
        });
        entries
    }
}

/// Syncs spline_t from AIController into RaceProgress each frame,
/// so standings reflect continuous progress between gates.
pub fn sync_spline_progress(
    mut progress: Option<ResMut<RaceProgress>>,
    drones: Query<(&Drone, &AIController, &DronePhase)>,
) {
    let Some(ref mut progress) = progress else { return };
    for (drone, ai, phase) in &drones {
        if *phase != DronePhase::Racing {
            continue;
        }
        let idx = drone.index as usize;
        if let Some(state) = progress.drone_states.get_mut(idx) {
            state.spline_t = ai.spline_t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_progress(drone_count: usize, total_gates: u32) -> RaceProgress {
        RaceProgress {
            drone_states: (0..drone_count)
                .map(|_| DroneRaceState::default())
                .collect(),
            total_gates,
        }
    }

    #[test]
    fn record_gate_pass_advances_next_gate() {
        let mut p = make_progress(2, 5);
        p.record_gate_pass(0, 0);
        assert_eq!(p.drone_states[0].next_gate, 1);
        assert_eq!(p.drone_states[0].gates_passed, 1);
    }

    #[test]
    fn record_gate_pass_increments_count() {
        let mut p = make_progress(1, 5);
        p.record_gate_pass(0, 0);
        p.record_gate_pass(0, 1);
        p.record_gate_pass(0, 2);
        assert_eq!(p.drone_states[0].gates_passed, 3);
        assert_eq!(p.drone_states[0].next_gate, 3);
    }

    #[test]
    fn record_gate_pass_ignored_after_crash() {
        let mut p = make_progress(1, 5);
        p.record_crash(0, DnfReason::MissedGate(0));
        p.record_gate_pass(0, 1);
        assert_eq!(p.drone_states[0].gates_passed, 0);
    }

    #[test]
    fn record_gate_pass_ignored_after_finish() {
        let mut p = make_progress(1, 5);
        p.record_finish(0, 42.0);
        p.record_gate_pass(0, 3);
        assert_eq!(p.drone_states[0].gates_passed, 0);
    }

    #[test]
    fn record_finish_sets_time() {
        let mut p = make_progress(1, 5);
        p.record_finish(0, 42.5);
        assert!(p.drone_states[0].finished);
        assert_eq!(p.drone_states[0].finish_time, Some(42.5));
    }

    #[test]
    fn record_finish_idempotent() {
        let mut p = make_progress(1, 5);
        p.record_finish(0, 42.5);
        p.record_finish(0, 99.0);
        assert_eq!(p.drone_states[0].finish_time, Some(42.5));
    }

    #[test]
    fn record_crash_sets_reason() {
        let mut p = make_progress(1, 5);
        p.record_crash(0, DnfReason::MissedGate(2));
        assert!(p.drone_states[0].crashed);
        assert_eq!(p.drone_states[0].dnf_reason, Some(DnfReason::MissedGate(2)));
    }

    #[test]
    fn record_crash_idempotent() {
        let mut p = make_progress(1, 5);
        p.record_crash(0, DnfReason::MissedGate(2));
        p.record_crash(0, DnfReason::MissedGate(3));
        assert_eq!(p.drone_states[0].dnf_reason, Some(DnfReason::MissedGate(2)));
    }

    #[test]
    fn is_active_default() {
        let p = make_progress(1, 5);
        assert!(p.is_active(0));
    }

    #[test]
    fn is_active_false_after_crash() {
        let mut p = make_progress(1, 5);
        p.record_crash(0, DnfReason::MissedGate(0));
        assert!(!p.is_active(0));
    }

    #[test]
    fn is_active_false_after_finish() {
        let mut p = make_progress(1, 5);
        p.record_finish(0, 10.0);
        assert!(!p.is_active(0));
    }

    #[test]
    fn is_active_out_of_bounds() {
        let p = make_progress(1, 5);
        assert!(!p.is_active(99));
    }

    #[test]
    fn standings_sorts_finished_by_time() {
        let mut p = make_progress(3, 5);
        p.record_finish(0, 30.0);
        p.record_finish(1, 20.0);
        p.record_finish(2, 25.0);
        let s = p.standings();
        assert_eq!(s[0].0, 1); // 20s
        assert_eq!(s[1].0, 2); // 25s
        assert_eq!(s[2].0, 0); // 30s
    }

    #[test]
    fn standings_finished_before_crashed() {
        let mut p = make_progress(3, 5);
        p.record_crash(0, DnfReason::MissedGate(1));
        p.record_finish(1, 20.0);
        p.record_gate_pass(2, 0);
        p.record_gate_pass(2, 1);
        let s = p.standings();
        assert_eq!(s[0].0, 1); // finished
        // drone 2 (2 gates) should come before drone 0 (0 gates)
        assert_eq!(s[1].0, 2);
        assert_eq!(s[2].0, 0);
    }

    #[test]
    fn standings_crashed_sorted_by_gates_passed() {
        let mut p = make_progress(3, 5);
        p.record_gate_pass(0, 0);
        p.record_crash(0, DnfReason::MissedGate(1));
        p.record_gate_pass(1, 0);
        p.record_gate_pass(1, 1);
        p.record_gate_pass(1, 2);
        p.record_crash(1, DnfReason::MissedGate(3));
        p.record_crash(2, DnfReason::MissedGate(0));
        let s = p.standings();
        assert_eq!(s[0].0, 1); // 3 gates
        assert_eq!(s[1].0, 0); // 1 gate
        assert_eq!(s[2].0, 2); // 0 gates
    }

    #[test]
    fn standings_uses_spline_t_as_tiebreaker() {
        let mut p = make_progress(3, 5);
        // All three drones have passed 2 gates
        p.record_gate_pass(0, 0);
        p.record_gate_pass(0, 1);
        p.record_gate_pass(1, 0);
        p.record_gate_pass(1, 1);
        p.record_gate_pass(2, 0);
        p.record_gate_pass(2, 1);
        // But drone 2 is furthest along the spline, drone 1 next, drone 0 last
        p.drone_states[0].spline_t = 6.5;
        p.drone_states[1].spline_t = 7.8;
        p.drone_states[2].spline_t = 8.2;
        let s = p.standings();
        assert_eq!(s[0].0, 2); // highest spline_t
        assert_eq!(s[1].0, 1);
        assert_eq!(s[2].0, 0); // lowest spline_t
    }
}
