use bevy::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnfReason {
    MissedGate(u32),
}

#[derive(Clone, Debug)]
pub struct DroneRaceState {
    pub next_gate: u32,
    pub gates_passed: u32,
    pub finished: bool,
    pub finish_time: Option<f32>,
    pub crashed: bool,
    pub dnf_reason: Option<DnfReason>,
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

    pub fn is_active(&self, drone_index: usize) -> bool {
        self.drone_states
            .get(drone_index)
            .is_some_and(|s| !s.crashed && !s.finished)
    }

    /// Build a snapshot of the final race results for the Results screen.
    pub fn to_race_results(&self, total_time: f32, course_name: String) -> RaceResults {
        let standings = self.standings();
        let entries = standings
            .iter()
            .map(|&(drone_idx, state)| RaceResultEntry {
                drone_index: drone_idx,
                finished: state.finished,
                finish_time: state.finish_time,
                crashed: state.crashed,
                gates_passed: state.gates_passed,
            })
            .collect();
        RaceResults {
            standings: entries,
            total_time,
            course_name,
        }
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
                (false, false) => b.gates_passed.cmp(&a.gates_passed),
            }
        });
        entries
    }
}

/// Snapshot of race results, persists into the Results state.
#[derive(Resource)]
pub struct RaceResults {
    pub standings: Vec<RaceResultEntry>,
    pub total_time: f32,
    pub course_name: String,
}

pub struct RaceResultEntry {
    pub drone_index: usize,
    pub finished: bool,
    pub finish_time: Option<f32>,
    pub crashed: bool,
    pub gates_passed: u32,
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
}
