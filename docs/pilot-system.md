# Pilot System

## Pilot Pattern

`PilotRoster` resource loaded at `Startup` (persists entire app lifetime). `SelectedPilots` + `PilotConfigs` created `OnEnter(Race)` (12 random pilots from roster), removed `OnExit(Results)`. `SelectedPilots` provides gamertags/colors to UI; `PilotConfigs` provides pre-computed `DroneConfig` values to `spawn_drones()`. After each race, `update_pilot_stats_after_race` updates pilot stats (wins, finishes, crashes, best times) and saves roster to disk. `DroneIdentity` component on each drone entity carries name + color. Personality traits (`PersonalityTrait` enum) map to `DroneConfig` modifiers via `TraitModifiers`. Skill profiles (`SkillProfile`) compress randomization ranges toward optimal values at higher skill levels. Gamertags are combinatorial (prefixes + roots + suffixes + style variations).

## Portrait Pattern

`portrait/` submodule with 5 files. `PortraitDescriptor` has 6 slot enums (FaceShape/EyeStyle/MouthStyle/HairStyle/ShirtStyle/Accessory) + color fields (including `shirt_color` derived via `derive_shirt_color()`). `PortraitDescriptor::generate(rng, primary_color)` creates randomized portraits. Accessory has 4 variants: EarringRound, EarringRing, NecklaceChain, NecklacePendant (with serde aliases for old names: Necklace, SpikedCollar, Piercings, Earring, etc.).

### SVG Pipeline

`loader.rs` parses a single master Inkscape SVG (`assets/portraits/pilot-portraits.svg`) at startup, extracting layers by `inkscape:label` into `PortraitParts` resource. `fragments.rs` assembles portrait SVGs from `PortraitParts` fragments with per-layer hex color replacement: BLACK (#000000) = primary color, WHITE (#ffffff) = secondary color, per layer type (face: skin_tone/skin_highlight, hair: hair_color, eyes: hair_color/eye_color, mouth: skin_highlight/vanilla, shirt: shirt_color, accessory: acc_color/acc_shadow). Global replacements: #808080 → VANILLA (#f2f2da), #333333 → BLACK (#000000). Background layer: #808080 → bg_color. ViewBox is `"0 0 20 20"`. Layer order (back to front): bg, hair_back, face, shirt, eyes, mouth, hair_front, accessory. Fallback mappings collapse old enum variants to new ones (e.g., Long->Oval, Goggles->Wide, Helmet->Beanie).

### Rasterization & Cache

`rasterize.rs` uses `resvg` 0.47 to render SVG -> `tiny_skia::Pixmap` -> Bevy `Image` (512x512 via `PORTRAIT_SIZE` in cache.rs). Hot-reload: F6 re-reads master SVG and invalidates `PortraitCache`. `PortraitCache` resource (`HashMap<PilotId, Handle<Image>>`) built `OnEnter(Race)` via chained `setup_portrait_cache` system (after `select_pilots_for_race`), persists across races. Roster migration: `backfill_empty_portraits()` auto-generates portraits for Phase 1 pilots using deterministic RNG seeded by `PilotId`. Portraits displayed in leaderboard (`LbPortrait` component, 64x64) and results screen (128x128 `ImageNode`), with solid-color fallback.

## Dev Menu Pattern

`AppState::DevMenu` accessed via "Dev" button on main menu. `dev_menu/` module with `portrait_config.rs` (data model + persistence) and `portrait_editor.rs` (UI + systems). `PortraitPaletteConfig` resource with per-slot vetoed colors and complementary mappings, persisted to `assets/dev/portrait_palette.ron`. `PortraitEditorState` resource tracks active tab, variant/color selections, dirty flag. Editor has part tabs (Face/Eyes/Mouth/Hair/Shirt/Accessory), variant radio buttons, 8x8 primary color grid (left-click select, right-click veto), secondary color grid for slots with `has_secondary()` (Skin/Eye/Accessory), live 512x512 preview. `generate_with_config()` on `PortraitDescriptor` picks from non-vetoed `PALETTE_COLORS` and respects complementary mappings. `generate_initial_roster()` loads palette config from disk.
