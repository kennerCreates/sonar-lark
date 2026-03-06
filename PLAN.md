# Phase 4 — Circuit Attraction & Pilot Recruitment: Implementation Plan

## Overview

Phase 4 adds the management meta-loop: after each race, the player runs marketing campaigns to grow a fan network, which gates pilot recruitment. The core systems are **track quality scoring**, **location/venue data**, a **fan network simulation** (word-of-mouth tree with tier progression/decay), **marketing campaigns**, and **pilot recruitment gating**.

The work is split into 7 implementation steps. Steps 0 is prerequisite refactoring. Steps 1-3 are pure data/logic with no UI or ECS (fully testable). Steps 4-6 wire into Bevy and add UI. Step 7 adds the spectator crowd rendering.

---

## Step 0 — Prerequisite Refactoring

Clean up debt identified in TODO.md that will either block Phase 4 work or cause friction during it. Do all of this before touching Phase 4 logic.

### 0a. Bundle `spawn_obstacle` params into `SpawnObstacleContext`
- **Files**: `obstacle/spawning.rs`, `editor/course_editor/ui/load.rs`
- **What**: Create a `SystemParam` struct that bundles the 8 glTF/material handles. Refactor `spawn_obstacle`, `load_course_into_editor`, `handle_load_button`, `auto_load_pending_course` to use it.
- **Why now**: Phase 4 may add new spawn contexts (fan meshes, venue props). Cleaning this up first avoids compounding the param bloat.

### 0b. Unify `PlacedFilter` usage
- **Files**: `editor/course_editor/mod.rs`, `save_delete.rs`, `load.rs`
- **What**: Make the existing `type PlacedFilter` at `course_editor/mod.rs:94` `pub` and replace the 5 inlined copies in `save_delete.rs` and `load.rs`.
- **Why now**: Quick win, prevents copy drift.

### 0c. Rename `EditorTab` → `PortraitEditorTab`
- **Files**: `dev_menu/portrait_editor/mod.rs`, `build.rs`, `interaction.rs`, `display.rs`
- **What**: Rename the portrait editor's `EditorTab` to `PortraitEditorTab` to avoid name collision with `editor/types.rs::EditorTab`.
- **Why now**: Phase 4 adds UI with tabs — name collisions will cause confusion during development.

### 0d. Remove dead code
- `catchphrases()` in `personality.rs:125`
- `clear_complementary_for()` in `portrait_config.rs:210`
- `MouthStyle::index()` in `slot_enums.rs:111`
- `ObstacleMarker::id` field in `obstacle/spawning.rs:9`

### 0e. Delete redundant re-export files
- Delete `menu/discover.rs` and `editor/course_editor/ui/discover.rs`
- Update imports in `menu/mod.rs` and `course_editor/ui/mod.rs` to import from `crate::course::discovery` directly.

### 0f. Replace glob re-exports with named re-exports
- **Files**: `workshop/ui/mod.rs`, `portrait_editor/mod.rs`
- **What**: Replace `pub use submod::*` with explicit named re-exports.

### 0g. Deduplicate `PANEL_BG`
- **Files**: `drone/dev_dashboard.rs`, `ui_theme.rs`
- **What**: Remove the duplicate `PANEL_BG` in `dev_dashboard.rs:7` and import from `ui_theme`.

---

## Step 1 — Track Quality Scoring (pure logic, no ECS)

**New file**: `src/race/track_quality.rs`

### Data
```rust
pub struct RaceSummary {
    pub gate_count: u32,
    pub distinct_obstacle_ids: u32,
    pub turn_tightness_counts: [u32; 3], // [gentle, medium, tight]
    pub elevation_deltas: Vec<f32>,       // y-delta between consecutive gates
    pub overtake_count: u32,
    pub crash_count: u32,
    pub photo_finish_gap: f32,            // 1st-2nd finish time delta
}

pub struct TrackQuality {
    pub gate_count_score: f32,
    pub obstacle_variety_score: f32,
    pub turn_mix_score: f32,
    pub elevation_score: f32,
    pub overtake_score: f32,
    pub crash_score: f32,
    pub photo_finish_score: f32,
    pub overall: f32,                     // weighted sum
}
```

### Logic
- `harvest_race_summary(script: &RaceScript, gate_positions: &[Vec3], obstacle_ids: &[&str], tightness: &[Vec<TurnTightness>]) -> RaceSummary` — extract all inputs from data already computed during script generation.
- `compute_track_quality(summary: &RaceSummary) -> TrackQuality` — pure scoring function:
  - **Gate count**: Gaussian bell curve peaking at 11, spread ~2. `e^(-((n - 11)^2 / (2 * 4)))`.
  - **Obstacle variety**: `min(1.0, distinct_ids / 5.0)` (diminishing returns past 5 types).
  - **Turn mix**: Shannon entropy of the 3-bin tightness distribution, normalized to [0,1]. Uniform distribution of gentle/medium/tight scores highest.
  - **Elevation**: Mean absolute y-delta normalized against a target (~3.0 units). `1 - e^(-mean_delta / target)`.
  - **Overtakes**: `min(1.0, overtakes / 8.0)` (diminishing returns).
  - **Crash sweet spot**: Gaussian peaking at 1.5, spread 1.0. 0 crashes and 3+ both penalized.
  - **Photo finish**: `e^(-gap / 3.0)` — tighter gap = higher score.
  - **Overall**: Weighted sum with configurable weights (default: gate_count 0.10, variety 0.10, turn_mix 0.20, elevation 0.10, overtakes 0.20, crashes 0.15, photo_finish 0.15).

### Harvesting
- The inputs for `RaceSummary` are all available at the point `generate_race_script()` returns. Rather than modifying `generate_race_script` itself, add a separate `harvest_race_summary()` function that takes the same inputs + the produced `RaceScript` and extracts the summary. The caller (in `lifecycle.rs`) will call both in sequence.
- `classify_turn_tightness` is currently private in `script.rs` — make it `pub(crate)` so `track_quality.rs` can use it, or pass the already-computed tightness data through.

### Testing
- Unit tests for each scoring function with known inputs/expected outputs.
- Edge cases: 0 gates, 0 overtakes, 0 crashes, single gate, all-tight course, all-gentle course.

### Wire-up point (deferred to Step 5)
- `TrackQuality` stored as a `Resource` after race script generation, displayed in results UI.

---

## Step 2 — Location & Venue Data (pure data, serialization)

**New file**: `src/course/location.rs`

### Data
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub base_attractiveness: f32,  // 0.0-1.0
    pub capacity: u32,             // max spectators
}
```

### Initial locations
Hard-coded initial set (could later move to RON):
- Abandoned Warehouse: attractiveness 0.2, capacity 40
- Local Park: attractiveness 0.4, capacity 80
- Golf Course: attractiveness 0.7, capacity 200

### Integration
- Add `location: String` field to `CourseData` (with `#[serde(default)]` for backward compat — defaults to "Abandoned Warehouse").
- `LocationRegistry` resource (simple `Vec<Location>` or `HashMap<String, Location>`) loaded at startup.
- Player selects location when creating/editing a course (or it's assigned per-venue — depends on UI flow, but the data model is the same).

### Testing
- Serde roundtrip for `Location`.
- Backward compat: existing `.course.ron` files without `location` field deserialize correctly.

---

## Step 3 — Fan Network Simulation (pure logic, no ECS)

**New file**: `src/league/mod.rs` (new top-level module)
**New file**: `src/league/fan_network.rs`
**New file**: `src/league/marketing.rs`
**New file**: `src/league/recruitment.rs`

This is the largest step. All logic is pure functions operating on data structs — no Bevy ECS dependency. This makes it fully unit-testable.

### 3a. Core data model (`league/mod.rs` + `fan_network.rs`)

```rust
pub type PersonId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanTier { Aware, Attendee, Fan, Superfan }

#[derive(Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub recruited_by: Option<PersonId>,
    pub tier: FanTier,
    pub races_attended: u16,
    pub races_since_attended: u8,
    pub spread_count: u8,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct FanNetwork {
    pub people: Vec<Person>,
    next_id: PersonId,
}

/// Inputs to the per-race fan simulation.
pub struct RaceAttractionInputs {
    pub track_quality: f32,          // 0.0-1.0
    pub location_attractiveness: f32,// 0.0-1.0
    pub capacity: u32,
    pub marketing: MarketingEffects,
    pub seed: u32,
}

/// Output of the per-race fan simulation.
pub struct RaceAttractionResult {
    pub demand: u32,                // total wanting to attend
    pub actual_attendance: u32,     // min(demand, capacity)
    pub turned_away: u32,           // demand - actual
    pub new_aware_from_spread: u32,
    pub promotions: u32,
    pub demotions: u32,
    pub removed: u32,               // Aware people who decayed out
    pub fan_count: u32,             // Fan + Superfan total after this race
    pub network_size: u32,          // total Aware+ after this race
}
```

### 3b. Tier progression & decay logic (`fan_network.rs`)

Core function: `pub fn simulate_race(network: &mut FanNetwork, inputs: &RaceAttractionInputs) -> RaceAttractionResult`

Steps within `simulate_race`:
1. **Attendance roll**: For each Aware+ person, compute attendance probability based on tier + location attractiveness + marketing nudge. Roll deterministically (seeded). Collect `demand`.
2. **Capacity cap**: `actual_attendance = min(demand, capacity)`. If capped, randomly select which people actually attend (priority by tier — Superfans first, then Fans, etc.).
3. **Update attendance tracking**: For each attendee, increment `races_attended`, reset `races_since_attended = 0`. For non-attendees who weren't turned away, increment `races_since_attended`.
4. **Promotions**: Check each person against promotion thresholds:
   - Aware → Attendee: first attendance (automatic on attend)
   - Attendee → Fan: attended 3 of last 5 races (track via `races_attended` + `races_since_attended`)
   - Fan → Superfan: 8+ total attended AND attended last 3 consecutively
5. **Demotions**: Check each person against decay thresholds:
   - Superfan → Fan: `races_since_attended >= 3`
   - Fan → Attendee: `races_since_attended >= 3`
   - Attendee → Aware: `races_since_attended >= 3`
   - Aware → removed: `races_since_attended >= 5`
   - Turned-away people do NOT increment `races_since_attended` (they tried)
6. **Spreading**: For each Attendee/Fan/Superfan who attended, roll spread chance (scales with tier and race excitement = `track_quality * 0.5 + location_attractiveness * 0.5`). On success, add new Aware person as child in tree.

Attendance probability by tier (base values, modified by location + marketing):
- Aware: 0.15
- Attendee: 0.40
- Fan: 0.75
- Superfan: 0.95

Spread chance by tier (base values, modified by race excitement):
- Attendee: 0.10
- Fan: 0.20
- Superfan: 0.35

### 3c. Marketing campaigns (`league/marketing.rs`)

```rust
pub struct MarketingEffects {
    pub aware_attendance_nudge: f32,  // added to Aware attendance probability
    pub new_aware_count: u32,         // injected from posters/highlight reel
    pub spread_potency_mult: f32,     // multiplier on spread chance (highlight reel)
    pub spread_volume_bonus: u32,     // extra spread rolls per spreader (merch)
    pub decay_slowdown: bool,         // merch slows decay
}

pub struct CampaignBudgets {
    pub posters: f32,
    pub highlight_reel: f32,
    pub merch: f32,
}

pub fn compute_marketing_effects(budgets: &CampaignBudgets) -> MarketingEffects
```

Campaign formulas:
- **Posters**: `new_aware = floor(8.0 * ln(1 + budget))`, `aware_nudge = 0.05 * (1 - e^(-0.1 * budget))`
- **Highlight Reel**: `spread_potency_mult = 1.0 + 0.5 * (1 - e^(-0.05 * budget))`, `new_aware = floor(2.0 * ln(1 + budget))`
- **Merch**: `spread_volume_bonus = floor(3.0 * (1 - e^(-0.03 * budget)))`, `decay_slowdown = budget > 0`

### 3d. Pilot recruitment gating (`league/recruitment.rs`)

```rust
pub struct RecruitmentTier {
    pub fan_threshold: u32,    // Fan + Superfan count needed
    pub pilot_skill_range: (f32, f32),  // min/max skill level accessible
    pub label: &'static str,
}

pub const RECRUITMENT_TIERS: &[RecruitmentTier] = &[
    RecruitmentTier { fan_threshold: 0,  pilot_skill_range: (0.1, 0.3), label: "Amateur" },
    RecruitmentTier { fan_threshold: 5,  pilot_skill_range: (0.2, 0.5), label: "Local" },
    RecruitmentTier { fan_threshold: 15, pilot_skill_range: (0.3, 0.7), label: "Regional" },
    RecruitmentTier { fan_threshold: 30, pilot_skill_range: (0.5, 0.85), label: "National" },
    RecruitmentTier { fan_threshold: 60, pilot_skill_range: (0.7, 1.0), label: "Elite" },
];

pub fn accessible_tier(fan_count: u32) -> &'static RecruitmentTier
pub fn pilot_willing_to_join(pilot: &Pilot, tier: &RecruitmentTier, seed: u32) -> bool
```

### 3e. Seeding
- `FanNetwork::new_seeded()` creates a network with 3-5 Aware people (friends/family). No recruiter (`recruited_by: None`).

### 3f. Persistence
- `FanNetwork` serialized as RON to `assets/league/fan_network.ron` via existing `persistence::save_ron` / `load_ron`.
- `LeagueState` resource wraps `FanNetwork` + accumulated `money: f32` + current `CampaignBudgets`.

### Testing (extensive — this is the core simulation)
- Seeding: verify initial network size 3-5, all Aware.
- Single race with no marketing: verify small attendance, some spread, promotions.
- Promotion thresholds: verify Aware→Attendee on first attend, Attendee→Fan after threshold.
- Decay: verify demotion after missed races, Aware removal after 5 missed.
- Capacity overflow: verify `actual_attendance = min(demand, capacity)`, turned-away don't decay.
- Marketing: verify poster injection count matches formula, highlight reel boosts spread, merch adds rolls.
- Recruitment: verify fan count gates tier access correctly.
- Determinism: same seed produces same results.
- Multi-race sequences: run 10+ simulated races, verify network grows/shrinks plausibly.

---

## Step 4 — Bevy Integration & State Wiring

### Resources
- `LeagueState` resource (wraps `FanNetwork`, `money`, `CampaignBudgets`, `TrackQuality` from last race). Loaded from RON at startup, saved after each race.
- `TrackQuality` resource — computed after script generation, consumed by results UI and fan simulation.
- `RaceAttractionResult` resource — output of fan simulation, consumed by results UI.

### System flow
1. **After script generation** (`lifecycle.rs`, in the `generate_race_script_system`):
   - Call `harvest_race_summary()` + `compute_track_quality()`.
   - Insert `TrackQuality` resource.
2. **On entering Results state** (new system in `results/mod.rs`):
   - Read `TrackQuality`, `LeagueState`, course's `Location`.
   - Call `simulate_race()` on the fan network.
   - Insert `RaceAttractionResult` resource.
   - Save updated `LeagueState` to RON.
3. **On entering HypeSetup state** (existing `hype/` module):
   - Player sets campaign budgets (UI already partially exists via `AdCampaign` enum in `states.rs`).
   - Campaign budgets stored in `LeagueState`.

### Module registration
- New `LeaguePlugin` in `src/league/mod.rs` — registers startup system (load `LeagueState`), state-transition systems.
- Register in `main.rs`.

---

## Step 5 — Results & Campaign UI

### Results screen additions (`results/ui.rs`)
- Display **Track Quality** breakdown (7 sub-scores + overall) in a compact panel.
- Display **Venue Appeal** (location attractiveness).
- Display **Attendance**: "X attended (Y wanted to come)" with overflow warning if capacity exceeded.
- Display **Fan Network** summary: total network size, Fan+Superfan count, recruitment tier label.
- Display **Money** earned from ticket sales (attendance * ticket price).

### Campaign UI (in `hype/` module, `HypeSetup` state)
- The `AdCampaign` enum and `HypeMode` state already exist in `states.rs`.
- Enable all three campaigns (currently only `Posters` returns `is_enabled() = true`).
- Budget sliders/input for each campaign type.
- Preview of expected effects (e.g., "~12 new people will hear about your league").
- "Run Campaign" button → compute marketing effects → apply to `LeagueState` budgets.
- Auto-generated poster option (skip poster editor, still get poster marketing effect).

### State flow update
Current: `Race → Results → Menu → Editor → Race`
Updated: `Race → Results → HypeSetup → Menu → Editor → Race`
- Results screen gets a "Continue" button that goes to `HypeSetup` (instead of directly to Menu).
- HypeSetup "Done" goes to Menu.
- This inserts the campaign phase into the core loop.

---

## Step 6 — Pilot Recruitment Integration

### Roster generation changes (`pilot/roster.rs`)
- When generating the initial roster or regenerating pilots, use `accessible_tier()` from `LeagueState.fan_network` to determine the skill range.
- Personality traits modulate willingness: Reckless pilots join with less fans, Cautious pilots need more.
- "Recruit Pilot" UI on the menu screen — shows available pilots based on current recruitment tier, player picks who to recruit.

### Progression feel
- Display recruitment tier prominently: "Your league attracts **Regional** pilots" with a progress bar toward the next tier.
- When a new tier unlocks, show a notification/celebration.

---

## Step 7 — Spectator Crowd Rendering

### Visual representation of fans at races
- Low-poly instanced meshes with `CelMaterial`.
- Sine-wave vertex displacement for cheering animation (in vertex shader or as a simple system).
- Count scales with `actual_attendance` from the fan simulation.
- Single draw call via GPU instancing.
- Placed near the course (along the "sidelines" of gates, or at designated spectator areas).
- **Performance budget**: Must stay within 60fps target. Cap rendered crowd at ~200 instances. For attendance > 200, scale instance density rather than count.

### Implementation
- New component: `SpectatorCrowd` with instance count + bounds.
- Spawn system runs on entering `Race` state, reads `LeagueState` for attendance prediction (or uses previous race's attendance as estimate).
- Simple mesh: 4-6 triangle humanoid silhouette, colored by `CelMaterial`.
- Position spread: random within defined bounds near gates, all facing toward the track.

---

## Dependency Graph

```
Step 0 (Refactoring) ─────────────────────────────────┐
                                                       │
Step 1 (Track Quality) ──────┐                         │
                              │                        │
Step 2 (Location Data) ──────┤                         │
                              ├── Step 4 (Bevy Wiring) ── Step 5 (UI) ── Step 6 (Recruitment)
Step 3 (Fan Network) ────────┘                                                │
                                                                              │
                                                                     Step 7 (Crowd Rendering)
```

Steps 1, 2, and 3 are independent of each other and can be developed in parallel after Step 0. Steps 4-7 are sequential.

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Fan simulation too complex to tune | Medium | Keep all constants as named consts, not magic numbers. Add a "fast-forward 10 races" test that prints network stats for tuning. |
| Crowd rendering blows perf budget | High | Cap instance count, use simplest possible mesh, benchmark early. Could defer to post-Phase 4 if needed. |
| `FanNetwork` save file grows large | Low | `Person` is ~20 bytes. 1000 fans = 20KB RON. Not a concern until thousands of races. |
| Marketing balance feels wrong | Medium | All formulas use configurable constants. Iterate after playtesting. |
| Backward compat for `CourseData` | Low | `#[serde(default)]` on new `location` field. Existing courses default to "Abandoned Warehouse". |

---

## Estimated Test Count

| Step | New tests |
|------|-----------|
| 0 (Refactoring) | 0 (existing tests must still pass) |
| 1 (Track Quality) | ~12 (scoring functions, edge cases) |
| 2 (Location Data) | ~4 (serde, backward compat) |
| 3 (Fan Network) | ~25 (simulation, marketing, recruitment, determinism, multi-race) |
| 4 (Bevy Wiring) | ~2 (resource insertion, state transitions) |
| 5 (UI) | 0 (manual testing) |
| 6 (Recruitment) | ~5 (tier gating, personality modifiers) |
| 7 (Crowd) | ~2 (instance count scaling, position spread) |
| **Total** | **~50** |
