# Plan: Firework Emitter Props in Course Editor

## Overview
Add two types of firework emitters (Confetti, Shell Burst) as placeable props in the course editor, with a new tab system separating obstacles from props.

## Phase 1: Data Model (src/course/data.rs)

Add `PropKind` enum, `PropInstance` struct, and a `props` field to `CourseData`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropKind {
    ConfettiEmitter,
    ShellBurstEmitter,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropInstance {
    pub kind: PropKind,
    pub translation: Vec3,
    pub rotation: Quat,
    /// RGBA override color. None = use winner's drone color at race time.
    pub color_override: Option<[f32; 4]>,
}
```

Add to `CourseData`:
```rust
#[serde(default)]
pub props: Vec<PropInstance>,
```

`#[serde(default)]` ensures backward compatibility — existing RON files without `props` load with an empty Vec.

## Phase 2: Editor Components & State (src/editor/course_editor/mod.rs)

### New types
```rust
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    Obstacles,
    Props,
}

#[derive(Component, Clone)]
pub struct PlacedProp {
    pub kind: PropKind,
    pub color_override: Option<[f32; 4]>,
}
```

### PlacementState additions
- Add `active_tab: EditorTab` field (default `Obstacles`)
- Add `selected_prop_kind: Option<PropKind>` for prop palette selection

### Gizmo query changes
All gizmo systems (`draw_move_gizmo`, `handle_move_gizmo`, `draw_rotate_gizmo`, `handle_rotate_gizmo`, `draw_scale_gizmo`, `handle_scale_gizmo`, `draw_selection_highlight`) currently query `With<PlacedObstacle>`. Change to `Or<(With<PlacedObstacle>, With<PlacedProp>)>` so gizmos work on both types.

### Selection changes
- `find_placed_ancestor_from_ray`: Change to check for either `PlacedObstacle` or `PlacedProp` during hierarchy walk
- `handle_placement_and_selection`: Gate order mode only applies to `PlacedObstacle`; prop selection works the same as obstacle selection
- `handle_delete_key`: Already works on any `selected_entity`, no change needed

### New system: `draw_prop_gizmos`
Draw editor-only visual markers for placed props:
- **Confetti Emitter**: yellow sphere gizmo (0.6m) + upward arrows showing burst direction
- **Shell Burst Emitter**: orange sphere gizmo (0.6m) + starburst lines above showing detonation area

### Cleanup
- `cleanup_course_editor`: Also despawn `PlacedProp` entities
- Register new systems in plugin

## Phase 3: Tab UI & Prop Palette (src/editor/course_editor/ui.rs)

### New marker components
```rust
struct ObstacleTabButton;
struct PropsTabButton;
struct PropPaletteButton(PropKind);
struct ObstaclePaletteContent;
struct PropPaletteContent;
struct PropColorButton;
struct PropColorLabel;
```

### Left panel restructure (`build_left_panel`)
Replace the "Obstacle Palette" header with:
1. **Tab row**: two buttons "Obstacles" | "Props", horizontally laid out
2. **Obstacle content** (`ObstaclePaletteContent`): existing `PaletteContainer` with obstacle buttons — visible when Obstacles tab active
3. **Props content** (`PropPaletteContent`): two buttons "Confetti Emitter" and "Shell Burst Emitter" — visible when Props tab active

### New systems
- `handle_tab_switch`: Toggle `PlacementState.active_tab`, set `Display::None`/`Display::Flex` on content containers
- `handle_prop_palette_selection`: When prop button clicked, spawn a placeholder cube mesh at origin with `PlacedProp`, select it
- `update_tab_button_colors`: Highlight active tab

### Prop placeholder mesh
Spawn a small colored cube (0.4m) using `CelMaterial`:
- Confetti: `palette::SUNSHINE` (gold)
- Shell Burst: `palette::TANGERINE` (orange)

Placeholder meshes pre-allocated in a `PropEditorMeshes` resource on `OnEnter(CourseEditor)`.

### Save changes (`handle_save_button`)
Also query `(PlacedProp, Transform)` to build `Vec<PropInstance>` and include in `CourseData`.

### Load changes (`load_course_into_editor`)
After spawning obstacles, iterate `course.props` to spawn placeholder entities with `PlacedProp`.

### Color override UI
When a `PlacedProp` is selected, show a small section below the palette:
- "Color: Auto" or "Color: [name]" label
- Button to cycle: Auto → SUNSHINE → NEON_RED → SKY → FROG → ORCHID → VANILLA → Auto

## Phase 4: Race-time Emitter Spawning (src/course/loader.rs)

### New component (in fireworks.rs)
```rust
#[derive(Component)]
pub struct FireworkEmitter {
    pub kind: PropKind,
    pub color_override: Option<Color>,
}
```

### spawn_course changes
After spawning obstacles, iterate `CourseData.props`:
- For each `PropInstance`, spawn an invisible marker entity with:
  - `Transform` from prop translation + rotation
  - `FireworkEmitter { kind, color_override }`
  - `DespawnOnExit(AppState::Race)`

## Phase 5: Modified Firework Triggering (src/drone/fireworks.rs)

### `detect_first_finish` changes
1. Query `FireworkEmitter` entities
2. If any emitters exist:
   - For each emitter, spawn the appropriate effect at emitter position:
     - `ConfettiEmitter` → call `spawn_confetti()` using emitter's rotation for direction
     - `ShellBurstEmitter` → spawn `PendingShell` above emitter position (3 staggered shells)
   - Color: use `emitter.color_override.unwrap_or(winner_drone_color)`
   - Do NOT spawn auto-fireworks at gate 0
3. If no emitters exist: keep existing auto-firework behavior unchanged

## File Change Summary

| File | Changes |
|------|---------|
| `src/course/data.rs` | Add `PropKind`, `PropInstance`, `props` field on `CourseData` |
| `src/editor/course_editor/mod.rs` | Add `PlacedProp`, `EditorTab`, update gizmo queries, add `draw_prop_gizmos`, update selection/cleanup |
| `src/editor/course_editor/ui.rs` | Tab UI, prop palette, prop placement, color override UI, save/load props |
| `src/drone/fireworks.rs` | Add `FireworkEmitter`, modify `detect_first_finish` to use placed emitters |
| `src/course/loader.rs` | Spawn `FireworkEmitter` entities from `CourseData.props` at race time |

## Performance Notes
- Prop placeholder cubes: 1 mesh + 1 material each, negligible
- Editor gizmo drawing: ~2-10 gizmo shapes per prop, negligible
- Race-time: same particle counts as existing fireworks, just at different positions
- No per-frame cost for emitters until triggered (just marker entities)

## Backward Compatibility
- `#[serde(default)]` on `props` field means existing `.course.ron` files load without changes
- Old courses = no emitters = auto-fireworks at gate 0 (unchanged behavior)
