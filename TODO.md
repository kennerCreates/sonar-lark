# TODO

## Procedural Pilot System

### Phase 1 — Pilot Data Model, Personality & Persistence ✅
Complete. `Pilot` struct with gamertag, personality traits (8 variants with `DroneConfig` modifiers), skill profiles (level + per-axis variation), stats, and placeholders for Phases 2–3. Combinatorial gamertag generator (6 styles, collision-free). `PilotRoster` (24 pilots) persisted as RON, loaded at startup, saved after each race. `SelectedPilots`/`PilotConfigs` resources created per race from roster. `DroneIdentity` component on each drone. Leaderboard, results UI, and crash effects all read pilot data. 21 pilot-specific tests passing.

### Phase 2 — SVG Portrait Generation ✅
Complete. Pilot avatars using hand-drawn Inkscape SVG fragments assembled at runtime, rasterized via `resvg` 0.47, and displayed as Bevy `Image` textures in UI. `portrait/` submodule with 4 files: `mod.rs` (data model, 6 slot enums including `ShirtStyle`, `PortraitDescriptor` with `generate()`), `fragments.rs` (hand-drawn Inkscape SVG fragments with viewBox `"9.5 11.5 20.1 20.1"`, `assemble_svg()`, `shirt_fragment()`, color helpers with `SHIRT_COLOR` token, portrait-03 fragments use `translate(-28,0)` wrapper), `rasterize.rs` (resvg pipeline -> Bevy `Image`), `cache.rs` (`PortraitCache` resource, 48x48 at race start). Layer order: bg -> hair_back -> face -> shirt -> eyes -> mouth -> hair_front -> accessory. Accessory reduced to 4 variants (Necklace, SpikedCollar, Piercings, Earring) with serde aliases for backward compat. Fallback mappings for old enum variants (Long->Oval, Diamond->Angular, Goggles->Wide, Winking->Normal, Gritted->Frown, Helmet->Beanie, Bald->ShortCrop, Ponytail->LongSwept). Portraits displayed in leaderboard (16x16) and results (20x20) with fallback solid-color squares. Backward-compatible: Phase 1 rosters auto-backfill portraits via deterministic seeding.

### Phase 3 - DELAYED

### Phase 4 — Circuit Attraction & Pilot Recruitment

**Attraction** is driven by two systems — a fan network (word-of-mouth growth) and track/venue quality — that together determine audience size and pilot interest.

**Fan network size** acts as the attraction metric for pilot recruitment. Tier thresholds on total fan count gate pilot pool access.

**Track quality** (0.0–1.0, computed after each race from existing data):
- [ ] Harvest race summary after script generation (all inputs already computed):
  - Gate count — bell curve peaking at ~11 (`e^(-((n - ideal)² / (2 × spread²)))`), rewards 10–12, penalizes both too few and too many
  - Obstacle variety — distinct `obstacle_id`s in course
  - Turn tightness mix — from `classify_turn_tightness()`, reward courses with a mix of gentle/moderate/tight over uniform tightness
  - Elevation changes — y-delta between consecutive gates (`ObstacleInstance.translation`); drives acrobatic maneuver variety
  - Overtake count — `RaceScript.overtakes.len()`
  - Crash sweet spot — same bell curve shape, peaks at 1–2 crashes, penalizes 0 (boring) and 3+ (dangerous)
  - Photo finish gap — 1st–2nd finish time delta, tighter = better
- [ ] Normalize into a single track quality score (weighted sum of per-factor scores, each 0.0–1.0)

**Location attractiveness** (0.0–1.0, per-venue constant — separate from track quality):
- [ ] Add `base_attractiveness: f32` to location data (not folded into track quality — kept as distinct visible stat)
  - Abandoned Warehouse: 0.2 (dingy, free, but low appeal)
  - Local Park: 0.4 (pleasant, accessible, modest draw)
  - Golf Course: 0.7 (scenic, upscale, strong draw)
- [ ] Add `capacity: u32` to location data — max spectators the venue can hold
  - Abandoned Warehouse: 40, Local Park: 80, Golf Course: 200
- [ ] Display both stats in UI so player sees "Track Quality: X" and "Venue Appeal: Y" separately — makes venue upgrades feel like a concrete lever to pull

**Fan network** (word-of-mouth referral tree with engagement tiers):

Five-tier progression per person: **Cold → Aware → Attendee → Fan → Superfan**
- [ ] `Person` struct: `recruited_by: Option<PersonId>`, `tier: FanTier`, `races_attended: u16`, `races_since_attended: u8`, `spread_count: u8`
- [ ] `FanTier` enum: `Cold`, `Aware`, `Attendee`, `Fan`, `Superfan`

**Tier definitions:**
- [ ] **Cold** — default state. Not in the network yet. Unreachable except through marketing (posters) or word of mouth from someone who knows them. Cold people don't exist as data — they're the implicit infinite pool outside the network
- [ ] **Aware** — aware the league exists but hasn't attended. Entry point into the network. Created by: word-of-mouth spread from Attendees+, or marketing campaigns. Each race, rolls an **attendance chance** based on location attractiveness + whether their recruiter is active. Low base chance, but marketing nudges it up
- [ ] **Attendee** — has attended at least one race. Promoted from Aware after first attendance. Now participates in word-of-mouth spreading (can recruit new Aware people). Attendance chance is moderate — they've been once, might come again if the race was good
- [ ] **Fan** — regular attendee. Promoted after attending N races (e.g., 3 of last 5). High attendance chance — they're hooked. Spreads more effectively than Attendees (higher spread chance). Counts toward pilot recruitment tier thresholds
- [ ] **Superfan** — devoted. Promoted after attending M races (e.g., 8+ total, attended last 3 consecutively). Near-guaranteed attendance. Highest spread effectiveness. Superfans are also more resilient to decay — takes longer to drop tiers

**Tier progression & decay:**
- [ ] **Promotion**: based on attendance history (races_attended count + recent streak). One-way ratchet during active attendance — you climb by showing up
- [ ] **Demotion**: missing races causes tier decay. Superfan → Fan after ~3 missed. Fan → Attendee after ~3 missed. Attendee → Aware after ~3 missed. Aware → removed from network after ~5 missed (they've forgotten about you). Each tier has its own decay threshold
- [ ] Decay is **per-person**, not global — one bad race doesn't collapse everything, but a streak of bad races causes a wave of demotions from the leaves inward

**Spreading (word of mouth):**
- [ ] Only Attendees, Fans, and Superfans can spread (Aware people don't evangelize something they haven't tried)
- [ ] Spread chance scales with tier: Attendee < Fan < Superfan
- [ ] Spread potency scales with race excitement (track quality + location attractiveness) — boring race = less to talk about
- [ ] Successful spread → new Aware person added as child node in the tree

**Attendance & capacity:**
- [ ] Each race: iterate all Aware+ people, roll attendance decision per tier. Compute `demand` (total wanting to attend)
- [ ] If `demand > capacity`, show overflow in results UI (e.g., "62 wanted to attend — venue only holds 40!"). Actual attendance = `min(demand, capacity)`. Turned-away people don't decay — they tried to come
- [ ] Render attending fans as low-poly instanced meshes with `CelMaterial`, sine-wave vertex cheering. Count scales with attendance. Single draw call via instancing

**Seeding:**
- [ ] League starts with 3–5 Aware people (friends/family who know about it but haven't come yet). First race attendance is small and organic

**Marketing campaigns** (strategic decisions, not creative judgment — each affects the fan network differently):
- [ ] All three campaigns **nudge Aware people** toward attending — slightly higher attendance chance next race. Marketing reminds people the league exists and lowers the barrier to first attendance
- [ ] **Posters**: Inject N new Aware people (strangers who saw the poster — independent roots, no recruiter). Cheap, decent volume. Also nudges existing Aware people. Budget → diminishing returns: `new_heard_of = floor(k × ln(1 + budget))`
- [ ] **Highlight Reel**: Boosts **spread potency** — Attendees/Fans/Superfans who spread have a higher success chance next race (the reel gives them something compelling to share). Also recruits a small number of new Aware people directly (people who stumble on the reel online). Primary value is amplifying organic growth, not raw volume
- [ ] **Merch**: Boosts **spread volume** — spreaders get extra recruitment rolls (simulates wearing the merch in public and getting asked about it). Doesn't make each attempt more likely to convert, but gives more attempts. Merch also slows tier decay for the buyer (they're invested — takes longer to demote)
- [ ] Budget → diminishing returns curve for all campaigns (`1 - e^(-k·budget)`)
- [ ] Poster editor remains a pure creative sandbox — no scoring or judging of player art
- [ ] Auto-generated poster option for players who want to skip the editor

**Pilot recruitment** — fan count gates the available pilot pool:
- [ ] Fan + Superfan count determines which tier of pilots will consider joining (tier gating, not smooth curve). Aware and Attendees don't count — pilots care about dedicated following, not casual awareness
- [ ] Within accessible tier, pilot personality traits create variance in willingness

**Meta-loop:**
- [ ] Build course → Race → Campaign → Attraction grows → Better pilots available → Repeat

#### Refactoring (before or during Phase 4)
- [ ] Bundle `spawn_obstacle` params into `SpawnObstacleContext` system param — currently 16 params, causing 15-16 param bloat in `load_course_into_editor`, `handle_load_button`, `auto_load_pending_course`. Collapse the 8 gltf/material handles into one struct. (`obstacle/spawning.rs`, `editor/course_editor/ui/load.rs`)
- [ ] Unify `PlacedFilter` usage — `type PlacedFilter` exists at `course_editor/mod.rs:94` but is inlined 5 more times in `save_delete.rs` and `load.rs`. Make `pub` and use everywhere. Critical before post-MVP "multiple obstacle types beyond gates" or terrain.
- [ ] Rename `EditorTab` in portrait editor to `PortraitEditorTab` — name collision with `editor/types.rs::EditorTab`. No runtime issue but confuses codebase search.
- [ ] Remove dead code: `catchphrases()` (`personality.rs:125`), `clear_complementary_for()` (`portrait_config.rs:210`), `MouthStyle::index()` (`slot_enums.rs:111`), `ObstacleMarker::id` field (`obstacle/spawning.rs:9`).
- [ ] Delete redundant single-line re-export files: `menu/discover.rs`, `editor/course_editor/ui/discover.rs`. Import from `crate::course::discovery` directly.
- [ ] Replace glob re-exports (`pub use submod::*`) with named re-exports in `workshop/ui/mod.rs` and `portrait_editor/mod.rs`.
- [ ] Deduplicate `PANEL_BG` — `dev_dashboard.rs:7` redefines the identical value from `ui_theme::PANEL_BG`. Import instead.

---

### Phase 3 — Modular Drone Models -- DELAYED FOR NOW
- [ ] Model part library in Blender: frames (5-6), arms (4), canopies (3-4), optional extras (prop guards, antennas)
- [ ] Single `.glb` with named nodes (follows obstacle library pattern)
- [ ] Runtime assembly: select parts per pilot's drone build descriptor, parent into entity hierarchy
- [ ] Apply pilot color scheme via `CelMaterial` color remapping
- [ ] ~6×4×4×3 = 288+ visual combos before color variation

#### Refactoring (during Phase 3)
- [ ] Replace `DroneAssets` with a part-aware `DronePartLibrary` — keyed by slot name (frame/arms/canopy/extra), with per-part transforms. Current flat `Vec<Handle<Mesh>>` + single `mesh_transform` cannot represent modular parts. (`drone/spawning.rs:27-31`)
- [ ] Expand `ColorScheme` to multi-color — add `#[serde(default)] secondary: Option<[f32; 3]>` (and possibly `accent`) for two-tone liveries. Currently only `primary`. (`pilot/mod.rs:55-57`)
- [ ] Wire `DroneBuildDescriptor` through to spawning — add `drone_build: DroneBuildDescriptor` to `SelectedPilot` so the spawn pipeline can read part selections. Currently the field exists on `Pilot` but is dropped at the selection bridge. (`pilot/mod.rs:118-122`, `drone/spawning.rs`)
- [ ] Bundle `crash_drone` parameters into a `CrashContext` struct — currently 13 params (`race/collision.rs:120-163`). Phase 3 may add modular debris, further stressing this signature.

## Future (Post-MVP)
- [ ] Player-controlled drone (same throttle/pitch/roll/yaw interface as AI)
- [ ] Per-drone customization (motor thrust, weight, drag, frame size)
- [ ] Multiple obstacle types beyond gates
- [ ] Multi-lap races
- [ ] Terrain elevation
- [ ] Gamepad support
