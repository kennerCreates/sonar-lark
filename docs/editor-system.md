# Editor System

## Obstacle Asset Loading

All obstacle models come from a single `assets/models/obstacles.glb`. Individual objects are accessed via `Gltf::named_nodes` → `GltfNode` → `GltfMesh` → primitives, using the Blender object name. Each obstacle spawns a parent entity with child `Mesh3d`/`MeshMaterial3d` per primitive.

## Workshop Pattern

**Note:** The obstacle workshop has been moved to the dev menu (`DevMenuPage::ObstacleWorkshop`). Source files remain in `src/editor/workshop/` but the plugin is registered by `DevMenuPlugin`. Camera rig is set up/torn down on workshop page enter/exit.

`WorkshopState` created `OnEnter(ObstacleWorkshop)`, removed `OnExit`. `PreviewObstacle` entities manually despawned on exit. Node list populated via `run_if(obstacles_gltf_ready)` + `run_if(workshop_nodes_pending)`; placeholder cube if no glb match. Workshop module split: `mod.rs` (types, lifecycle, plugin), `preview.rs` (preview spawning), `gizmos.rs` (trigger/collision/ground gizmos), `widgets.rs` (move/resize widget drawing and handling).

## Menu Pattern

`AvailableCourses` created `OnEnter(Menu)`, removed `OnExit`. `SelectedCourse` persists across states into Race.

## Course Props

`PropInstance` in `CourseData.props` with `PropKind` (ConfettiEmitter/ShellBurstEmitter), optional `color_override`. `#[serde(default)]` for backward compat. Editor uses tabbed UI (`EditorTab::Obstacles`/`Props`/`Cameras`), `PlacedProp` component, `PlacedFilter` type alias for shared queries.

## Course Cameras

`CameraInstance` in `CourseData.cameras` with `translation`, `rotation`, `is_primary`, optional `label`. `#[serde(default)]` for backward compat. Editor "Cameras" tab with `PlacedCamera` component, frustum gizmo visualization (primary=sunshine, normal=sky). `CameraEditorMeshes` resource for placeholder cubes. Primary toggle enforces single-primary. Soft cap warning at >9 cameras. PiP preview (384x216 render-to-texture) appears when a `PlacedCamera` is selected — `PreviewCamera` entity with `RenderTarget::Image`, `CameraPreview` resource, auto-hidden when deselected. `preview.rs` module in `editor/course_editor/`.

## Undo/Redo System

`src/editor/undo.rs`. Generic `UndoStack<A>` resource with 50-action capacity, used by both course editor and workshop. Ctrl+Z undoes, Ctrl+Y redoes.

**Course editor** uses `CourseEditorAction` enum: transform changes, spawn/delete (obstacles, props, cameras), flip gate, prop color changes. Undo-of-delete remaps entity IDs (respawns entity, patches stored ID). Stack cleared on load, new course, hot-reload.

**Workshop** uses `WorkshopAction` with a snapshot-based approach (`WorkshopSnapshot`). Stack cleared on obstacle switch.

Key types: `UndoStack<A>`, `CourseEditorAction`, `WorkshopAction`, `WorkshopSnapshot`, `CameraSnapshot`.

## Transform Gizmos

`transform_gizmos/` directory module: `mod.rs` (widget resource types, constants, `sample_ring_screen_dist()`), `move_gizmo.rs`, `rotate_gizmo.rs` (includes `angle_in_plane()` tests), `scale_gizmo.rs`.

### Axis-Constrained Move Gizmo

The move gizmo draws per-axis arrows (X=red, Y=green, Z=blue) and a plane indicator square (yellow). Clicking an arrow constrains movement to that single axis. Clicking the plane square allows free XZ-plane movement. Both course editor and workshop use this pattern.

## Course Editor UI Files

`ui/` directory: `discover.rs` (re-exports from `course::discovery`), `left_panel.rs` + `right_panel.rs` (UI construction), `data.rs` (build_course_data + tests), `load.rs` (load into editor), `save_delete.rs` (save/delete/navigation/gate ordering), `systems.rs` (interaction handlers, display updates), `types.rs` (marker components, re-exports `CourseEntry`). Button styling uses shared `ui_theme` module.

## Course Delete Pattern

`PendingCourseDelete` tracks deletion with inline Yes/Cancel confirmation. Resets editor state if deleted course is currently loaded.
