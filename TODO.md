# TODO

## Procedural Pilot System

### Phase 1 — Pilot Data Model, Personality & Persistence ✅
Complete. `Pilot` struct with gamertag, personality traits (8 variants with `DroneConfig` modifiers), skill profiles (level + per-axis variation), stats, and placeholders for Phases 2–3. Combinatorial gamertag generator (6 styles, collision-free). `PilotRoster` (24 pilots) persisted as RON, loaded at startup, saved after each race. `SelectedPilots`/`PilotConfigs` resources created per race from roster. `DroneIdentity` component on each drone. Leaderboard, results UI, and crash effects all read pilot data. 21 pilot-specific tests passing.

### Phase 2 — SVG Portrait Generation ✅
Complete. Pilot avatars using hand-drawn Inkscape SVG fragments assembled at runtime, rasterized via `resvg` 0.47, and displayed as Bevy `Image` textures in UI. `portrait/` submodule with 4 files: `mod.rs` (data model, 6 slot enums including `ShirtStyle`, `PortraitDescriptor` with `generate()`), `fragments.rs` (hand-drawn Inkscape SVG fragments with viewBox `"9.5 11.5 20.1 20.1"`, `assemble_svg()`, `shirt_fragment()`, color helpers with `SHIRT_COLOR` token, portrait-03 fragments use `translate(-28,0)` wrapper), `rasterize.rs` (resvg pipeline -> Bevy `Image`), `cache.rs` (`PortraitCache` resource, 48x48 at race start). Layer order: bg -> hair_back -> face -> shirt -> eyes -> mouth -> hair_front -> accessory. Accessory reduced to 4 variants (Necklace, SpikedCollar, Piercings, Earring) with serde aliases for backward compat. Fallback mappings for old enum variants (Long->Oval, Diamond->Angular, Goggles->Wide, Winking->Normal, Gritted->Frown, Helmet->Beanie, Bald->ShortCrop, Ponytail->LongSwept). Portraits displayed in leaderboard (16x16) and results (20x20) with fallback solid-color squares. Backward-compatible: Phase 1 rosters auto-backfill portraits via deterministic seeding.

### Phase 3 — Modular Drone Models
- [ ] Model part library in Blender: frames (5-6), arms (4), canopies (3-4), optional extras (prop guards, antennas)
- [ ] Single `.glb` with named nodes (follows obstacle library pattern)
- [ ] Runtime assembly: select parts per pilot's drone build descriptor, parent into entity hierarchy
- [ ] Apply pilot color scheme via `CelMaterial` color remapping
- [ ] ~6×4×4×3 = 288+ visual combos before color variation

### Phase 4 — Circuit Reputation & Pilot Attraction (Design TBD)
- [ ] Reputation system for the player's circuit
- [ ] Pilot attraction mechanics (higher reputation → higher-skilled pilots)
- [ ] Possible multi-factor preferences (course difficulty, track fame, etc.)
- [ ] Circuit management meta-game loop

---

## Future (Post-MVP)
- [ ] Player-controlled drone (same throttle/pitch/roll/yaw interface as AI)
- [ ] Per-drone customization (motor thrust, weight, drag, frame size)
- [ ] Multiple obstacle types beyond gates
- [ ] Multi-lap races
- [ ] Terrain elevation
- [ ] Gamepad support
