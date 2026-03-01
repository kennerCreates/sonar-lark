# Editor System

## Obstacle Asset Loading

All obstacle models come from a single `assets/models/obstacles.glb`. Individual objects are accessed via `Gltf::named_nodes` → `GltfNode` → `GltfMesh` → primitives, using the Blender object name. Each obstacle spawns a parent entity with child `Mesh3d`/`MeshMaterial3d` per primitive.

## Workshop Pattern

`WorkshopState` created `OnEnter(ObstacleWorkshop)`, removed `OnExit`. `PreviewObstacle` entities manually despawned on exit. Node list populated via `run_if(obstacles_gltf_ready)` + `run_if(workshop_nodes_pending)`; placeholder cube if no glb match. Workshop module split: `mod.rs` (types, lifecycle, plugin), `preview.rs` (preview spawning), `gizmos.rs` (trigger/collision/ground gizmos), `widgets.rs` (move/resize widget drawing and handling).

## Menu Pattern

`AvailableCourses` created `OnEnter(Menu)`, removed `OnExit`. `SelectedCourse` persists across states into Race.

## Course Props

`PropInstance` in `CourseData.props` with `PropKind` (ConfettiEmitter/ShellBurstEmitter), optional `color_override`. `#[serde(default)]` for backward compat. Editor uses tabbed UI (`EditorTab::Obstacles`/`Props`/`Cameras`), `PlacedProp` component, `PlacedFilter` type alias for shared queries.

## Course Cameras

`CameraInstance` in `CourseData.cameras` with `translation`, `rotation`, `is_primary`, optional `label`. `#[serde(default)]` for backward compat. Editor "Cameras" tab with `PlacedCamera` component, frustum gizmo visualization (primary=sunshine, normal=sky). `CameraEditorMeshes` resource for placeholder cubes. Primary toggle enforces single-primary. Soft cap warning at >9 cameras. PiP preview (384x216 render-to-texture) appears when a `PlacedCamera` is selected — `PreviewCamera` entity with `RenderTarget::Image`, `CameraPreview` resource, auto-hidden when deselected. `preview.rs` module in `editor/course_editor/`.

## Transform Gizmos

`transform_gizmos/` directory module: `mod.rs` (widget resource types, constants, `sample_ring_screen_dist()`), `move_gizmo.rs`, `rotate_gizmo.rs` (includes `angle_in_plane()` tests), `scale_gizmo.rs`.

## Course Editor UI Files

`ui/` directory: `discover.rs` (re-exports from `course::discovery`), `left_panel.rs` + `right_panel.rs` (UI construction), `data.rs` (build_course_data + tests), `load.rs` (load into editor), `save_delete.rs` (save/delete/navigation/gate ordering), `systems.rs` (interaction handlers, display updates), `types.rs` (marker components, re-exports `CourseEntry`). Button styling uses shared `ui_theme` module.

## Course Delete Pattern

`PendingCourseDelete` tracks deletion with inline Yes/Cancel confirmation. Resets editor state if deleted course is currently loaded.
