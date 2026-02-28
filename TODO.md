# TODO

## Procedural Pilot System

### Phase 1 — Pilot Data Model, Personality & Persistence ✅
Complete. `Pilot` struct with gamertag, personality traits (8 variants with `DroneConfig` modifiers), skill profiles (level + per-axis variation), stats, and placeholders for Phases 2–3. Combinatorial gamertag generator (6 styles, collision-free). `PilotRoster` (24 pilots) persisted as RON, loaded at startup, saved after each race. `SelectedPilots`/`PilotConfigs` resources created per race from roster. `DroneIdentity` component on each drone. Leaderboard, results UI, and crash effects all read pilot data. 21 pilot-specific tests passing.

### Phase 2 — SVG Portrait Generation ✅
Complete. Pilot avatars using hand-drawn Inkscape SVG fragments assembled at runtime, rasterized via `resvg` 0.47, and displayed as Bevy `Image` textures in UI. `portrait/` submodule with 4 files: `mod.rs` (data model, 6 slot enums including `ShirtStyle`, `PortraitDescriptor` with `generate()`), `fragments.rs` (hand-drawn Inkscape SVG fragments with viewBox `"9.5 11.5 20.1 20.1"`, `assemble_svg()`, `shirt_fragment()`, color helpers with `SHIRT_COLOR` token, portrait-03 fragments use `translate(-28,0)` wrapper), `rasterize.rs` (resvg pipeline -> Bevy `Image`), `cache.rs` (`PortraitCache` resource, 48x48 at race start). Layer order: bg -> hair_back -> face -> shirt -> eyes -> mouth -> hair_front -> accessory. Accessory reduced to 4 variants (Necklace, SpikedCollar, Piercings, Earring) with serde aliases for backward compat. Fallback mappings for old enum variants (Long->Oval, Diamond->Angular, Goggles->Wide, Winking->Normal, Gritted->Frown, Helmet->Beanie, Bald->ShortCrop, Ponytail->LongSwept). Portraits displayed in leaderboard (16x16) and results (20x20) with fallback solid-color squares. Backward-compatible: Phase 1 rosters auto-backfill portraits via deterministic seeding.

#### 2A — Data Model (`pilot/portrait.rs`)

Expand `PortraitDescriptor` from its current empty placeholder into a full descriptor:

```rust
pub struct PortraitDescriptor {
    pub face_shape: FaceShape,
    pub eyes: EyeStyle,
    pub mouth: MouthStyle,
    pub hair: HairStyle,
    pub shirt: ShirtStyle,
    pub accessory: Option<Accessory>,
    pub skin_tone: [f32; 3],       // sRGB
    pub hair_color: [f32; 3],      // sRGB
    pub eye_color: [f32; 3],       // sRGB
    pub shirt_color: [f32; 3],     // sRGB (derived via derive_shirt_color())
    pub accessory_color: [f32; 3], // sRGB (derived from pilot primary)
}
```

**Slot enums** (each maps to a hand-drawn Inkscape SVG fragment):

| Slot | Enum | Variants | Notes |
|------|------|----------|-------|
| Face shape | `FaceShape` | Oval, Round, Square, Angular | 4 active variants (Long/Diamond aliased to Oval/Angular via serde fallback). |
| Eyes | `EyeStyle` | Normal, Narrow, Wide, Visor | 4 active variants (Goggles->Wide, Winking->Normal fallback). |
| Mouth | `MouthStyle` | Neutral, Smile, Smirk, Frown | 4 active variants (Gritted->Frown fallback). |
| Hair/Headwear | `HairStyle` | ShortCrop, Mohawk, LongSwept, Beanie | 4 active variants (Helmet->Beanie, Bald->ShortCrop, Ponytail->LongSwept fallback). |
| Shirt | `ShirtStyle` | Crew, Round, Turtleneck, Vneck | 4 variants. New slot — drawn between face and eyes layers. |
| Accessory | `Accessory` | Necklace, SpikedCollar, Piercings, Earring | 4 variants (renamed from old names, serde aliases for backward compat). Optional (50% chance of `None`). |

**Color ranges** (randomized at pilot generation, persisted forever):

| Color | Generation Strategy |
|-------|-------------------|
| Skin tone | 8 preset base tones with ±5% per-channel jitter |
| Hair color | Random hue, saturation 0.3–0.9, lightness 0.2–0.8 |
| Eye color | Random hue, constrained saturation/lightness for readability |
| Background | `ColorScheme.primary` (pilot's drone color) — fills the entire canvas behind the face |
| Accessory color | Derived from `ColorScheme.primary` (slightly shifted for contrast) |

**Combo count**: 4 × 4 × 4 × 4 × 4 × 5 (4 accessories + None) = **5,120 structural combos** before color variation. With color randomization, effectively infinite uniqueness.

**Serialization**: Full `PortraitDescriptor` serialized in roster RON via `#[serde(default)]` for backward compat with Phase 1 rosters. Default generates a random portrait on first access (migration path).

- [ ] Define `FaceShape`, `EyeStyle`, `MouthStyle`, `HairStyle`, `Accessory` enums with `Serialize`/`Deserialize`
- [ ] Expand `PortraitDescriptor` with all slot + color fields, `#[serde(default)]` on each field
- [ ] Implement `PortraitDescriptor::generate(rng, primary_color)` — random selection per slot, color generation
- [ ] Add `PortraitDescriptor::is_empty()` check (for detecting default/placeholder descriptors)
- [ ] Update `roster::generate_initial_pilots()` to call `PortraitDescriptor::generate()` for each new pilot
- [ ] Roster migration: on load, detect pilots with default/empty portrait and backfill with deterministic RNG seeded from `PilotId`

#### 2B — SVG Fragment Library (`pilot/portrait/fragments.rs`)

SVG fragments are **hand-drawn in Inkscape** and stored as const string slices in Rust source (not asset files). Each fragment is a `<g>` group with placeholder color attributes that get string-replaced during assembly. This avoids asset-loading complexity and keeps portraits deterministic and always available.

**Fragment format** — each fragment is an SVG `<g>` element designed for a `"9.5 11.5 20.1 20.1"` viewBox (cropped portrait area from the Inkscape canvas). Portrait-03 fragments use a `translate(-28,0)` wrapper to align correctly within the viewBox:
```xml
<g id="face_oval">
  <ellipse cx="19.5" cy="21.5" rx="8" ry="9" fill="SKIN_TONE"/>
  <ellipse cx="19.5" cy="21.5" rx="8" ry="9" fill="none" stroke="SKIN_SHADOW" stroke-width="0.5"/>
</g>
```

**Color placeholders** (replaced during assembly):
- `BG_COLOR` — pilot's primary color, fills the background
- `SKIN_TONE` — base skin color
- `SKIN_SHADOW` — darkened skin (auto-computed, ~70% lightness of base)
- `SKIN_HIGHLIGHT` — lightened skin (auto-computed, ~130% lightness of base)
- `HAIR_COLOR` — base hair color
- `HAIR_SHADOW` — darkened hair
- `EYE_COLOR` — iris color
- `SHIRT_COLOR` — shirt color (derived via `derive_shirt_color()`)
- `ACC_COLOR` — accessory color (shifted from pilot primary for contrast)
- `ACC_SHADOW` — darkened accessory color

**Layer order** (back to front): **bg** (background rect) → **hair_back** → **face** → **shirt** → **eyes** → **mouth** → **hair_front** → **accessory**

**Design constraints**:
- All fragments designed for `"9.5 11.5 20.1 20.1"` viewBox (hand-drawn in Inkscape)
- Portrait-03 fragments use `translate(-28,0)` to re-center into the viewBox
- High-contrast outlines so face features pop on the colored background
- Pilot primary color background ensures portraits are always visible against the dark UI — no separate border needed
- `shirt_fragment()` function selects the shirt SVG based on `ShirtStyle` enum
- Accessory fragment function matches Necklace/SpikedCollar/Piercings/Earring variants

- [ ] Create `pilot/portrait/` submodule (`mod.rs` + `fragments.rs`)
- [ ] Define const SVG fragments for all 6 face shapes
- [ ] Define const SVG fragments for all 6 eye styles
- [ ] Define const SVG fragments for all 5 mouth styles
- [ ] Define const SVG fragments for all 7 hair styles (some need back+front layers)
- [ ] Define const SVG fragments for all 5 accessories
- [ ] Define color placeholder constants (`SKIN_TONE`, `HAIR_COLOR`, etc.)
- [ ] Implement `compute_shadow(base: [f32; 3]) -> [f32; 3]` and `compute_highlight(base: [f32; 3]) -> [f32; 3]` helpers
- [ ] Implement `assemble_svg(descriptor: &PortraitDescriptor, bg_color: [f32; 3]) -> String` — emits full-canvas `<rect>` with `BG_COLOR`, selects fragments, performs color replacement, wraps in `<svg>` root with viewBox

#### 2C — Rasterization Pipeline (`pilot/portrait/rasterize.rs`)

Use `resvg` (pure Rust SVG renderer, ~0.47) + `usvg` (SVG parser) + `tiny-skia` (pixel buffer) to convert assembled SVG strings into Bevy `Image` assets. No render-to-texture camera needed — this is a CPU-side rasterization that produces pixel data directly.

**Dependency additions** to `Cargo.toml`:
```toml
resvg = "0.47"   # pulls in usvg + tiny-skia transitively
```

**Rasterization flow**:
```
PortraitDescriptor
    → assemble_svg()        → SVG string
    → usvg::Tree::from_str() → parsed SVG tree
    → resvg::render()       → tiny_skia::Pixmap (RGBA pixels)
    → Pixmap::data()        → &[u8] raw RGBA
    → bevy::Image::new()    → Bevy Image asset
```

**Output sizes**:
- Primary: **48×48** — used in leaderboard rows and results rows (fits in 20px row height when scaled)
- Future: 128×128 for detail views (pilot roster screen in Phase 4)
- Render at target size directly (no downscaling) for crisp pixel art feel

**Performance budget**: 24 portraits × ~0.5ms each = ~12ms total at startup. Acceptable as a one-time cost during roster generation. During race, only 12 portraits rendered (selected pilots). Can be done in `OnEnter(Race)` without frame budget concern since it happens before the first rendered frame.

- [ ] Add `resvg` dependency to `Cargo.toml`
- [ ] Implement `rasterize_portrait(descriptor: &PortraitDescriptor, bg_color: [f32; 3], size: u32) -> Image` — assemble SVG (with background), parse with `usvg`, render with `resvg`, convert `Pixmap` → Bevy `Image` (Rgba8UnormSrgb format)
- [ ] Handle error cases gracefully (malformed SVG → fallback solid-color square filled with pilot's primary color)
- [ ] Unit test: rasterize a portrait, verify output image dimensions and non-zero pixel data

#### 2D — Portrait Cache Resource (`pilot/portrait/cache.rs`)

A Bevy `Resource` that holds `Handle<Image>` for each pilot's rendered portrait, keyed by `PilotId`. Avoids re-rasterizing portraits every race.

```rust
#[derive(Resource)]
pub struct PortraitCache {
    portraits: HashMap<PilotId, Handle<Image>>,
}
```

**Lifecycle**:
1. `OnEnter(Race)` → `setup_portrait_cache` system:
   - For each of the 12 selected pilots, check if portrait is already cached
   - If not, call `rasterize_portrait()` → `images.add()` → store handle
   - Insert `PortraitCache` resource
2. `PortraitCache` persists across races (no cleanup on exit) — portraits accumulate as pilots are encountered. After a few races, all 24 pilots are cached.
3. `PortraitCache` is cleaned up only if the roster is regenerated (edge case).

**Why not pre-render all 24 at startup?** Could do that too (only ~12ms), but lazy caching means the startup path doesn't change, and it's trivially extensible if the roster grows beyond 24.

- [ ] Define `PortraitCache` resource with `HashMap<PilotId, Handle<Image>>`
- [ ] Implement `setup_portrait_cache` system — runs `OnEnter(Race)`, reads `SelectedPilots` + `PilotRoster`, rasterizes missing portraits (passing `color_scheme.primary` as `bg_color`), inserts resource
- [ ] Run `setup_portrait_cache` with `run_if(resource_exists::<SelectedPilots>)` guard (pilots must be selected first)
- [ ] System ordering: `setup_portrait_cache` must run after `select_pilots_for_race` (both `OnEnter(Race)`)
- [ ] Implement `PortraitCache::get(pilot_id) -> Option<Handle<Image>>` accessor

#### 2E — UI Integration: Race Leaderboard

Add a small portrait thumbnail (20×20 display, sourced from 48×48 texture) to each leaderboard row, between the color bar and the name text.

**Current row layout** ([race/ui.rs:470–520](src/race/ui.rs#L470-L520)):
```
[4px color bar] [4px gap] [position + name text] [time text]
```

**New row layout**:
```
[4px color bar] [4px gap] [20×20 portrait] [4px gap] [position + name text] [time text]
```

**Implementation**:
- New marker component: `LbPortrait(usize)` (parallels `LbColorBar`, `LbNameText`, `LbTimeText`)
- During `spawn_leaderboard`, if `PortraitCache` is available, insert `ImageNode` with the pilot's portrait handle. If cache not yet ready (unlikely due to system ordering), use a transparent placeholder.
- `update_leaderboard` system: on standings reorder, update each row's `ImageNode` to match the new pilot at that position. This requires swapping image handles when standings change — either update the `ImageNode` source, or rebuild the leaderboard when portrait data becomes available.
- Fallback: if no portrait available for a pilot, display a solid-color 20×20 square using `BackgroundColor(pilot_color)`.

- [ ] Add `LbPortrait(usize)` marker component
- [ ] Modify `spawn_leaderboard` to include a 20×20 `ImageNode` per row (or `BackgroundColor` fallback)
- [ ] Update `update_leaderboard` to swap portrait image handles when standings order changes
- [ ] Verify portrait visibility — pilot primary color background should naturally pop against the dark leaderboard panel

#### 2F — UI Integration: Results Screen

Add portrait thumbnails to the results standings rows, same approach as leaderboard.

**Current row layout** ([results/ui.rs:106–160](src/results/ui.rs#L106-L160)):
```
[4px color bar] [6px gap] [position + name] [time/DNF] [gates/total]
```

**New row layout**:
```
[4px color bar] [6px gap] [20×20 portrait] [6px gap] [position + name] [time/DNF] [gates/total]
```

- [ ] Modify results UI `build_results_standings` to include portrait `ImageNode` per row
- [ ] Read from `PortraitCache` resource (still available during Results state since it persists)
- [ ] Fallback to solid-color square if no portrait cached

#### 2G — Roster Migration & Backward Compatibility

Existing `roster.pilots.ron` files from Phase 1 have `portrait: ()` (empty struct). When loaded, `#[serde(default)]` produces a default `PortraitDescriptor`. We need to detect these and backfill with generated portraits.

**Migration strategy**:
- `PortraitDescriptor::default()` produces a sentinel value (e.g., `face_shape: FaceShape::Oval` and a flag field `generated: false`, or simply check if all colors are `[0,0,0]`).
- Simpler approach: add `#[serde(default)] pub generated: bool` field. Default is `false`. `generate()` sets it to `true`. On roster load, any pilot with `generated == false` gets a new portrait generated from a deterministic RNG seeded by their `PilotId.0`.
- Deterministic seeding ensures the same pilot always gets the same portrait even if migration runs multiple times.

- [ ] Add `generated: bool` field (or equivalent sentinel) to `PortraitDescriptor`
- [ ] In `roster::load_or_generate_roster`, after loading, iterate pilots and backfill empty portraits
- [ ] Use `StdRng::seed_from_u64(pilot.id.0)` for deterministic portrait generation on migration
- [ ] Save roster after migration so backfill only happens once
- [ ] Test: load a Phase 1 roster file → verify all pilots get portraits → save → reload → portraits unchanged

#### 2H — Testing

**Unit tests** (pure logic, no ECS):

| Test | Module | What it verifies |
|------|--------|-----------------|
| Portrait generation bounds | `portrait/mod.rs` | All enum variants valid, colors in 0.0–1.0, accessory `None` ~50% of the time over 100 samples |
| SVG assembly validity | `portrait/fragments.rs` | `assemble_svg()` output is parseable by `usvg::Tree::from_str()` for every enum combination |
| Color placeholder replacement | `portrait/fragments.rs` | No unreplaced `SKIN_TONE`/`HAIR_COLOR`/etc. literals remain in assembled SVG |
| Shadow/highlight computation | `portrait/fragments.rs` | `compute_shadow` darkens, `compute_highlight` brightens, values clamped to 0.0–1.0 |
| Rasterization output | `portrait/rasterize.rs` | `rasterize_portrait()` returns `Image` with correct dimensions, non-zero pixel data |
| Serialization roundtrip | `portrait/mod.rs` | `PortraitDescriptor` survives RON serialize → deserialize |
| Backward compat | `roster.rs` | Load a RON string with empty `portrait: ()` → default descriptor → backfill works |
| Deterministic migration | `roster.rs` | Same `PilotId` always produces same portrait when seeded |

- [ ] Write portrait generation bounds test (100-iteration stress test)
- [ ] Write SVG assembly validity test (enumerate all slot combos, parse each with `usvg`)
- [ ] Write color placeholder replacement test
- [ ] Write shadow/highlight clamping test
- [ ] Write rasterization output dimensions test
- [ ] Write `PortraitDescriptor` RON roundtrip test
- [ ] Write backward compat test (Phase 1 roster loading)
- [ ] Write deterministic migration test

#### 2I — Performance Notes

- **Rasterization cost**: resvg renders simple SVGs in ~0.3–0.5ms each (64×64, minimal geometry). 12 portraits = ~6ms. 24 portraits = ~12ms. Well within budget for a one-time operation.
- **Memory**: 48×48 RGBA = 9,216 bytes per portrait. 24 cached = ~221 KB. Negligible.
- **No per-frame cost**: Portraits are static images once rasterized. `ImageNode` in Bevy UI is a simple textured quad — zero additional rendering overhead beyond what any UI image costs.
- **Startup impact**: Zero. Rasterization happens `OnEnter(Race)`, not at startup. Roster loading remains fast (just deserialize, no rasterization).
- **resvg binary size**: ~400KB added to the binary. Acceptable.

#### 2J — File Structure

```
src/pilot/
├── mod.rs              (existing — update PortraitDescriptor, add portrait module)
├── portrait/
│   ├── mod.rs          (PortraitDescriptor, slot enums, generate(), is_empty())
│   ├── fragments.rs    (const SVG strings, assemble_svg(), color helpers)
│   ├── rasterize.rs    (resvg pipeline, rasterize_portrait() → Image)
│   └── cache.rs        (PortraitCache resource, setup system)
├── personality.rs      (existing — unchanged)
├── skill.rs            (existing — unchanged)
├── gamertag.rs         (existing — unchanged)
└── roster.rs           (existing — update for migration backfill)
```

#### 2K — Implementation Order

1. **2A** — Data model (enums, `PortraitDescriptor`, generation) — no dependencies
2. **2B** — SVG fragments (const strings, assembly function) — depends on 2A for enum types
3. **2C** — Rasterization (resvg integration) — depends on 2B for assembled SVGs
4. **2H (partial)** — Tests for 2A–2C (generation, SVG validity, rasterization) — validates foundation
5. **2D** — Portrait cache resource — depends on 2C for rasterization
6. **2G** — Roster migration — depends on 2A for generation, 2D for cache
7. **2E** — Leaderboard UI integration — depends on 2D for cache access
8. **2F** — Results UI integration — depends on 2D for cache access
9. **2H (remainder)** — Backward compat and integration tests
10. **2I** — Performance validation (manual timing check)

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
