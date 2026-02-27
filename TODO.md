# TODO

## Procedural Pilot System

### Phase 1 — Pilot Data Model, Personality & Persistence
Foundation phase. Purely data + logic, no new art assets needed.

- [ ] **Pilot struct** — `pilot/mod.rs`
  - Identity: generated gamertag (not real names — e.g. "xVortex_", "ShadowPulse", "n1tr0glitch")
  - Personality traits: enum set (Aggressive, Cautious, Flashy, Methodical, Reckless, etc.)
  - Skill: base skill level + per-axis variation (raw speed vs cornering vs consistency)
  - Drone build descriptor: placeholder for Phase 3 (frame, arms, canopy, color scheme)
  - Portrait descriptor: placeholder for Phase 2 (part selections + colors)
  - Stats: race history, wins, crashes, best times
- [ ] **Gamertag generator** — combinatorial system (prefixes, roots, suffixes, numbers, leetspeak, underscores/camelCase styles). ~100+ unique tags without repeats.
- [ ] **Personality system** — each trait maps to:
  - A pool of catchphrases/quips (displayed in UI later)
  - Modifiers on `DroneConfig` / `AiTuningParams` values (e.g. Aggressive → higher target speed, tighter gate margins; Cautious → wider approach, fewer crashes)
- [ ] **Skill → DroneConfig mapping** — skill level acts as an overall multiplier on how close to optimal the pilot performs. Per-axis variation means a high-speed pilot can still be sloppy in corners.
- [ ] **Pilot roster persistence** — `pilots.roster.ron` via serde, loaded at app startup, saved after each race. Follows existing RON conventions.
- [ ] **Replace hardcoded drones** — swap `DRONE_NAMES`/`DRONE_COLORS` in `drone/spawning.rs` with pilots drawn from the roster. Each race selects 12 pilots. Pilot color scheme applied to drone materials.
- [ ] **Race results integration** — update pilot stats after each race (finishes, crashes, best times). `RaceResults` references pilots by ID.

### Phase 2 — SVG Portrait Generation
- [ ] Modular SVG portrait system: slots (face shape, eyes, mouth, hair, accessories)
- [ ] Pool of SVG fragments per slot (5-6 options each → thousands of combos)
- [ ] Color parameterization (skin tone, hair color, eye color, accessories)
- [ ] Runtime assembly + render to texture
- [ ] Display portraits in leaderboard, results UI, and pilot roster screens

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
