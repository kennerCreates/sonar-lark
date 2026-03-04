# Pre-Simulation + Acrobatic Splice + Playback — Implementation Plan

## Why

The game is a drone racing league organizer (management game). Races are non-interactive — the player watches AI drones compete, and the spectacle rewards their management decisions (pilot selection, course design). The current physics system produces realistic flight but lacks acrobatic maneuvers at tight turns. Previous attempts to add acrobatics through live physics (maneuver override system) were too complex and were reverted.

**New architecture:** Pre-simulate the race using existing physics → record trajectories → splice in procedural acrobatic curves at tight turns → play back the enhanced recording.

**This gives us three things at once:**
1. Realistic physics-based flight (current system, unchanged)
2. Acrobatic maneuvers (spliced into recorded data, no physics complexity)
3. Replay capability (free — a replay is just re-playing the trajectory buffer)

---

## Architecture Overview

```
Course + Pilots + RaceSeed
         │
    ┌────▼─────────────────────┐
    │  SIMULATION (headless)   │  Existing physics engine, run fast
    │  ~1 second wall time     │  via Bevy time scaling
    │  64 Hz × ~90s race       │
    └────┬─────────────────────┘
         │
    RaceRecording (positions, rotations, velocities, events)
         │
    ┌────▼─────────────────────┐
    │  SPLICE (post-process)   │  Detect tight turns in trajectory
    │  One-time pass           │  Replace with procedural acrobatic curves
    │  ~instant                │  Boundary-matched for seamless blend
    └────┬─────────────────────┘
         │
    Enhanced RaceRecording
         │
    ┌────▼─────────────────────┐
    │  PLAYBACK (rendered)     │  Write recorded transforms to ECS each tick
    │  Real-time               │  Cameras, leaderboard, explosions all work
    │  Player watches + cams   │  unchanged — they read the same components
    └──────────────────────────┘
```

---

## Phases

Each phase is independently testable and committable. A phase only depends on previous phases.

| Phase | What | Risk | Dependencies |
|-------|------|------|-------------|
| **1. Gate/collision → FixedUpdate** | Move detection systems to correct schedule | Low — small targeted change | None |
| **2. Recording infrastructure** | Capture trajectory data during live races | Low — purely additive | None |
| **3. Playback system** | Replay a recording through the ECS | Medium — replaces physics chain conditionally | Phase 2 |
| **4. Fast simulation** | Run physics at high speed before playback | Medium — Bevy time scaling API | Phases 1, 2, 3 |
| **5. Acrobatic splice** | Post-process recording with procedural flips | Medium — curve math + visual tuning | Phases 2, 3 |

**Phases 1 and 2 have no dependencies on each other** and can be done in either order.

**Phase 3 (playback) is the pivotal phase** — once it works, we can play back any recording, which means Phase 4 (fast sim) and Phase 5 (splice) can be developed and tested independently against recorded data.

---

## Phase 1: Move Gate/Collision Detection to FixedUpdate

### Problem

Gate trigger check, obstacle collision check, and miss detection currently run in the `Update` schedule (once per rendered frame). They use swept line segments (`PreviousTranslation → Transform`) for detection. This has two issues:

1. **Latent correctness bug (now):** If 2+ FixedUpdate ticks run in one frame, intermediate gate crossings in the first tick are invisible to the Update-based detection. At 60fps and 64Hz physics, this occasionally happens (2 ticks in one frame). Fast drones could theoretically skip a gate.

2. **Blocks fast simulation (Phase 4):** With time scaling at 1000×, each frame runs ~1000 physics ticks. Update-based detection would only see the last tick's segment, missing everything in between.

### Changes

**File: `src/race/mod.rs`**

Move three systems from the Update chain to FixedUpdate, ordered after the drone physics chain:

```
Current (Update):
  tick_countdown → tick_race_clock → gate_trigger_check →
  obstacle_collision_check → miss_detection →
  sync_spline_progress → check_race_finished

New (split between schedules):

FixedUpdate (after drone physics chain):
  gate_trigger_check → obstacle_collision_check → miss_detection

Update (remaining):
  tick_countdown → tick_race_clock → sync_spline_progress → check_race_finished
```

The detection systems use `PreviousTranslation → Transform` which are per-tick values in FixedUpdate (exactly what we want). `tick_race_clock` stays in Update because it tracks wall-clock time for display. `sync_spline_progress` and `check_race_finished` can stay in Update — they read DronePhase and RaceProgress which are updated by the FixedUpdate systems.

### Verification

- `cargo build && cargo clippy && cargo test`
- Play a race: gates register correctly, crashes happen, leaderboard updates, race finishes
- Verify with debug draw (F3) that gate planes still align

---

## Phase 2: Recording Infrastructure

### Purpose

Capture per-drone trajectory data every physics tick during a race. This is the foundation for both playback and replay.

### New File: `src/drone/recording.rs`

#### Data Structures

```rust
/// Per-drone snapshot for a single physics tick.
pub struct TrajectoryFrame {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub phase: DronePhase,
    pub spline_t: f32,
    pub target_gate_index: u32,
}
// ~49 bytes per drone per tick
// 12 drones × 5,760 ticks (90s race) × 49B ≈ 3.4 MB

/// Timestamped race event, recorded during simulation.
pub enum RaceEvent {
    GatePass {
        tick: u32,
        drone_idx: u8,
        gate_idx: u32,
    },
    Finish {
        tick: u32,
        drone_idx: u8,
        elapsed_secs: f32,
    },
    Crash {
        tick: u32,
        drone_idx: u8,
        position: Vec3,
        velocity: Vec3,
        color: Color,
        reason: DnfReason,
    },
}

/// Complete recording of a race. Created during simulation, consumed by playback.
#[derive(Resource)]
pub struct RaceRecording {
    pub frames: Vec<[TrajectoryFrame; DRONE_COUNT]>,
    pub events: Vec<RaceEvent>,
    pub tick_rate_hz: f32,
    /// Per-drone splines, stored for FPV camera look-ahead during playback.
    pub splines: Vec<CubicCurve<Vec3>>,
    /// Per-drone gate count (all drones same, but stored for completeness).
    pub gate_count: u32,
}
```

#### Recording System

`record_trajectory_frame` — runs in FixedUpdate, AFTER the gate/collision detection chain (so crash events from the current tick are captured).

Each tick:
1. For each drone entity: read `Transform`, `DroneDynamics`, `DronePhase`, `AIController` → push `TrajectoryFrame`
2. Compare current frame to previous frame for event detection:
   - `phase` changed to `Crashed` → record `RaceEvent::Crash`
   - `target_gate_index` advanced → record `RaceEvent::GatePass`
   - Phase changed from `Racing` to `VictoryLap` → record `RaceEvent::Finish`
3. Append frame array and any new events to `RaceRecording`

#### Spline Capture

On the first tick (or when `RaceRecording` is initialized), clone each drone's `AIController.spline` into `RaceRecording.splines`. These are needed during playback for FPV camera look-ahead.

### File Modifications

- **`src/drone/mod.rs`** — add `pub mod recording`, register `record_trajectory_frame` in FixedUpdate after the detection chain
- **`src/race/lifecycle.rs`** — initialize `RaceRecording` resource when race starts (in `tick_countdown` when transitioning to Racing), clean up in `cleanup_race`

### Verification

- Race plays normally (no behavior change)
- After a race finishes: inspect `RaceRecording` in a test or debug system
  - Frame count ≈ race_duration × 64
  - Events match the leaderboard (same finishers, same crash count)
  - All 12 drones have data every tick

---

## Phase 3: Playback System

### Purpose

Replace the live physics chain with a system that reads from `RaceRecording` and writes to the same ECS components. All downstream systems (cameras, leaderboard, explosions, fireworks) work unchanged.

### New File: `src/drone/playback.rs`

#### Data Structures

```rust
/// Tracks playback progress.
#[derive(Resource)]
pub struct PlaybackState {
    pub current_tick: u32,
    pub countdown_timer: f32,  // 3.0s visual countdown before playback starts
    pub playing: bool,
}
```

#### Systems

**`playback_countdown`** (Update, run_if `is_watching` AND `!playing`):
- Tick down `countdown_timer` by `time.delta_secs()`
- Display "3", "2", "1", "GO!" (reuse existing countdown overlay)
- When timer reaches 0: set `playing = true`, init `RaceClock` and `RaceProgress`

**`advance_playback`** (FixedUpdate, run_if `is_watching` AND `playing`):
- For each drone, read `recording.frames[current_tick][drone_idx]`:
  - Write `Transform.translation` = frame.translation
  - Write `Transform.rotation` = frame.rotation
  - Write `DroneDynamics.velocity` = frame.velocity
  - Write `DronePhase` = frame.phase
  - Write `AIController.spline_t` = frame.spline_t
  - Write `AIController.target_gate_index` = frame.target_gate_index
- The existing interpolation pipeline (FixedFirst → FixedPreUpdate → FixedPostUpdate → PostUpdate) handles `PreviousTranslation`, `PhysicsTranslation`, and visual interpolation automatically — the playback system writes to `Transform` in the same FixedUpdate slot where physics would

**`replay_events`** (FixedUpdate, run_if `is_watching` AND `playing`, after `advance_playback`):
- Scan `recording.events` for events matching `current_tick`
- `GatePass` → call `progress.record_gate_pass(drone_idx, gate_idx)` (updates leaderboard)
- `Finish` → call `progress.record_finish(drone_idx, elapsed_secs)`
- `Crash` → call `crash_drone()` (spawns explosion particles, hides entity, updates RaceProgress)
- Advance `current_tick`

**`detect_playback_finished`** (Update, run_if `is_watching`):
- When `current_tick >= recording.frames.len()`: trigger same finish flow as `check_race_finished` (set `RacePhase::Finished`, insert `ResultsTransitionTimer`)

#### Why This Works Without Changing Cameras/UI/Effects

The playback system writes to the exact same ECS components that the physics chain writes to:

| Consumer | Reads | Playback provides? |
|----------|-------|--------------------|
| Chase camera | Transform, DroneDynamics.velocity, DronePhase, PreviousTranslation | Yes (Transform written by playback, others by interpolation pipeline) |
| FPV camera | Transform, DroneDynamics.velocity, DronePhase, AIController.spline/spline_t | Yes (spline stored in RaceRecording, written back to AIController) |
| Gate trigger | PreviousTranslation, Transform, DronePhase | Not needed — events come from recording |
| Collision | PreviousTranslation, Transform, DronePhase | Not needed — events come from recording |
| Leaderboard | RaceProgress (resource) | Yes — replay_events updates RaceProgress |
| Explosions | Triggered by crash_drone() call | Yes — replay_events calls crash_drone() |
| Fireworks | Triggered by RaceProgress finish detection | Yes — replay_events records finishes in RaceProgress |

### Conditional System Execution

**New resource:**
```rust
#[derive(Resource, Default)]
pub enum RaceMode {
    #[default]
    Live,       // Current behavior (Phase 2 testing)
    Simulating, // Fast sim (Phase 4)
    Watching,   // Playback (this phase)
}
```

Run conditions:
- `is_simulating()` → `matches!(mode, Simulating | Live)` (physics chain runs)
- `is_watching()` → `matches!(mode, Watching)` (playback systems run)
- Gate/collision detection in FixedUpdate: `run_if(is_simulating)` (not needed during playback — events come from recording)

### File Modifications

- **`src/drone/mod.rs`** — add `pub mod playback`, register playback systems with `run_if(is_watching)`, add `run_if(is_simulating)` to physics chain and detection chain
- **`src/race/mod.rs`** — init `RaceMode` resource
- **`src/race/lifecycle.rs`** — `check_race_finished` only runs during Simulating/Live

### Verification

Phase 3 can be tested with `RaceMode::Live` (default):
1. Race runs normally with recording (Phase 2)
2. After race ends, instead of going to Results, set `RaceMode::Watching` and reset to beginning
3. Playback should look identical to the live race
4. Cameras, leaderboard, explosions, fireworks should all work
5. Compare final leaderboard standings between live and playback — must match

---

## Phase 4: Fast Simulation

### Purpose

Run the physics simulation at high speed (sub-second wall time) so the player only sees the enhanced playback, never the raw simulation.

### Approach: Bevy Time Scaling

During `Simulating`:
1. Set `Time<Virtual>::set_relative_speed(1000.0)` — Bevy runs ~1000 FixedUpdate ticks per frame
2. Raise `Time<Fixed>` max accumulator/delta cap to allow many ticks per frame (exact API to verify against Bevy 0.18 docs)
3. A 90-second race at 64Hz = 5,760 ticks, which at 1000× completes in ~6 frames (~0.1s)

### Simulation Flow

```
OnEnter(AppState::Race)
  │
  ├── Load assets (glTF, sounds) — existing systems, unchanged
  ├── Spawn drones — existing spawn_drones, unchanged
  │
  ▼ (once drones spawned + assets ready)

Insert RaceMode::Simulating
  │
  ├── Set time scale high
  ├── Hide all drone entities (Visibility::Hidden)
  ├── Show loading overlay ("Simulating race...")
  ├── Skip countdown: set all DronePhase → Racing immediately
  ├── Init RaceClock + RaceProgress
  │
  ▼ (physics chain runs at high speed, recording captures data)

check_race_finished detects all drones done
  │
  ├── Reset time scale to 1.0
  ├── Run acrobatic splice on RaceRecording (Phase 5, or no-op if not yet implemented)
  ├── Unhide drone entities
  ├── Remove loading overlay
  ├── Set RaceMode::Watching
  ├── Init PlaybackState { countdown_timer: 3.0, playing: false }
  │
  ▼ (playback begins with countdown)
```

### New System: `start_simulation`

Runs in Update during `AppState::Race` with run conditions:
- `resource_exists::<DroneAssets>` AND `resource_exists::<CourseData>` AND `NOT resource_exists::<RaceMode>`

This system:
1. Inserts `RaceMode::Simulating`
2. Sets time scale high + raises tick cap
3. Hides drones
4. Shows loading overlay
5. Sets all drones to Racing, inits RaceClock + RaceProgress + RaceRecording

### Transition System: `finish_simulation`

Runs in Update during `RaceMode::Simulating`:
- When all drones in RaceProgress are finished or crashed:
  1. Reset time scale
  2. Run splice pass (Phase 5, no-op initially)
  3. Unhide drones
  4. Remove loading overlay
  5. Set `RaceMode::Watching`
  6. Init `PlaybackState`

### File Modifications

- **`src/race/lifecycle.rs`** — `start_simulation` and `finish_simulation` systems
- **`src/race/mod.rs`** — register new systems, loading overlay setup
- **`src/drone/mod.rs`** — ensure physics chain has `run_if(is_simulating)`
- **`src/race/start_button.rs`** — START RACE button is hidden/removed during simulation (race starts automatically)

### Fallback

If Bevy's time scaling doesn't support enough ticks per frame (there may be a hard cap in `FixedUpdate` to prevent spiral of death), the fallback is an **exclusive system** that manually advances `Time<Fixed>` and calls `World::run_schedule(FixedUpdate)` in a loop. This is more complex but gives complete control. Only pursue if time scaling fails.

### Verification

- Time from entering Race state to first playback frame should be < 2 seconds
- Race results must be identical to a normal-speed run with the same `RaceSeed`
- Loading overlay appears briefly and disappears
- No visual artifacts on first playback frame

---

## Phase 5: Acrobatic Splice

### Purpose

Post-process the `RaceRecording` to replace tight-turn segments with procedurally generated acrobatic maneuver curves. This is the payoff — the drones now flip.

### New File: `src/drone/splice.rs`

### Detection: Finding Splice Candidates

For each drone's recorded trajectory:

1. **Compute heading** per frame: `atan2(velocity.z, velocity.x)` (horizontal plane)
2. **Sliding window** (~32 frames / 0.5s): accumulate absolute heading change
3. **Threshold**: where cumulative heading change exceeds ~100° in the window, mark as candidate
4. **Find entry/exit**: walk backward/forward from the peak to find frames where heading change rate drops below a threshold — these are the stable entry and exit points

### Filtering: Which Drones Get Acrobatics

Not every drone should flip at every tight turn. Filter by:

- **Pilot skill**: `DroneConfig.cornering_aggression` > threshold (e.g., 1.2). Aggressive pilots flip, cautious ones bank. This ties the visual spectacle to the management layer — better pilots do cooler things.
- **Altitude budget**: entry frame `translation.y` > minimum (e.g., 3.0m). Split-S needs room to dive.
- **Proximity**: no other drone within ~3m during the splice window. Two drones flipping in the same space would look wrong.
- **Already spliced**: don't overlap splice windows.

### Curve Generation: Split-S

Given entry frame `E` (tick_e) and exit frame `X` (tick_x):

**Duration:** `n = tick_x - tick_e` ticks (use existing timing, don't stretch or compress)

**Position curve — Hermite spline with vertical dip:**
- Parameterize t ∈ [0, 1] over the splice window
- Match boundary conditions:
  - p(0) = E.translation, p'(0) = E.velocity × dt_window
  - p(1) = X.translation, p'(1) = X.velocity × dt_window
- Add a control bias: at t = 0.5, displace downward by `dip_amount` (e.g., altitude_budget × 0.4)
- Use cubic Hermite with the dip worked into the tangent/position at midpoint

**Rotation curve — keyframed quaternion slerp:**
- t = 0.00: E.rotation (entry orientation)
- t = 0.15: half-roll inverted (rotate 180° around velocity axis)
- t = 0.50: nose-down, pulling through the bottom of the arc
- t = 0.85: recovering toward level flight
- t = 1.00: X.rotation (exit orientation)
- Slerp between adjacent keyframes with ease-in-out smoothing
- Pilot skill affects slerp quality: skilled = smooth, less skilled = slight wobble (add small-amplitude noise to intermediate keyframes)

**Velocity — derived:**
- After generating position curve, compute velocity as finite differences: `v[i] = (p[i+1] - p[i]) / dt`
- This guarantees position-velocity consistency

### Splice Application

```
For each accepted candidate:
  1. Generate position, rotation, velocity curves for the splice window
  2. Blend zones (4-8 frames) at entry and exit:
     - Entry blend: lerp(original, spliced, smoothstep(t)) for frames [tick_e-4, tick_e+4]
     - Exit blend: lerp(spliced, original, smoothstep(t)) for frames [tick_x-4, tick_x+4]
  3. Replace frames in RaceRecording with blended result
```

### Future Maneuver Types

Start with Split-S only. Later additions (same architecture, different curve shapes):
- **Power Loop**: vertical climb + backward loop. Position curve arcs upward instead of downward. For turns where the next gate is higher.
- **Aggressive Bank**: not a full flip, just raise the tilt past the normal 83° clamp to ~120°. For medium turns that don't warrant a full flip.

### Per-Drone Visual Variation

Even within the same maneuver type, variation comes from:
- Entry/exit conditions differ per drone (different speeds, positions, orientations)
- Pilot-specific roll direction preference (`racing_line_bias`)
- Skill-based rotation smoothness
- Altitude budget varies (more altitude = deeper dip = more dramatic)

### File Modifications

- **`src/drone/splice.rs`** — NEW: all detection + generation + application
- **`src/drone/mod.rs`** — add `pub mod splice`
- **`src/drone/components.rs`** — potentially add a skill/acrobatic threshold field to `DroneConfig` (or derive from existing `cornering_aggression`)
- **`src/race/lifecycle.rs`** — call `splice::process_recording(&mut recording)` in `finish_simulation` before transitioning to Watching

### Verification

- Create a test course with tight turns (>90° direction change between consecutive gates)
- Run a race: some drones should visibly flip at tight corners
- Aggressive drones flip, cautious drones bank — verify by checking which DroneConfigs produce splices
- Check entry/exit smoothness: no visible pop, teleport, or discontinuity
- Check that spliced drones don't clip through the ground or obstacles
- Race results (finish order, times) are unchanged — splice doesn't affect the outcome

---

## Results State Considerations

The current Results state has drones transition to `Wandering` (free-roaming flight). During playback, when the recording ends:

1. `detect_playback_finished` triggers the Results transition
2. Drones are at their final recorded positions/phases
3. `OnEnter(Results)`: `transition_to_wandering` sets VictoryLap drones to Wandering as usual
4. **Switch back to live physics** for wandering: set `RaceMode::Live` or add a `PostRace` mode where the physics chain runs again

This means the physics chain needs to be active during Results (as it currently is) for wandering behavior. The `RaceMode` should be cleared or set to `Live` on Results entry so the physics chain resumes.

---

## Open Questions (to resolve during implementation)

1. **Bevy 0.18 time scaling API**: exact methods for `Time<Virtual>::set_relative_speed()` and `Time<Fixed>` max delta cap — verify against docs
2. **Tick cap**: does Bevy 0.18 have a hard limit on FixedUpdate ticks per frame? If so, what's the max and can it be configured?
3. **Loading overlay**: should this be a simple text overlay or a progress bar? (Progress bar is easy — we know total gates × avg ticks per gate)
4. **Splice tuning**: the heading-change threshold and altitude minimums will need iteration. Start conservative (only splice very tight turns) and relax over time.
