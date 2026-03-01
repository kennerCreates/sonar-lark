# Key Types Reference

| Type | Kind | Module | Purpose |
|------|------|--------|---------|
| `ObstacleId` | Data | obstacle/definition | Unique string ID for obstacle types |
| `ObstacleDef` | Data | obstacle/definition | glb scene name + trigger volume config |
| `ObstacleLibrary` | Resource | obstacle/library | All loaded obstacle definitions |
| `CourseData` | Resource | course/data | All obstacle placements for a course |
| `TriggerVolume` | Component | obstacle/spawning | AABB hitbox on gate entities |
| `GateIndex` | Component | race/gate | Gate sequence order |
| `GateForward` | Component | race/gate | World-space forward direction for gate validation |
| `GatePlanes` | Resource | race/gate | Cached per-gate plane data (center, normal, axes, half-extents) built once at race start for plane-crossing detection |
| `CollisionVolumeConfig` | Data | obstacle/definition | Local-space AABB (offset + half_extents) for obstacle collision volumes |
| `ObstacleCollisionVolume` | Component | obstacle/spawning | Runtime collision volume on obstacle entities (offset, half_extents, is_gate) |
| `ObstacleCollisionCache` | Resource | race/collision | Cached world-space OBBs for all obstacles with collision volumes, built once at race start |
| `RaceProgress` | Resource | race/progress | Per-drone gate/finish/crash tracking |
| `DroneRaceState` | Data | race/progress | Per-drone state: next_gate, gates_passed, finished, finish_time, crashed, dnf_reason |
| `RacePhase` | Resource | race/lifecycle | WaitingToStart → Countdown → Racing → Finished |
| `CountdownTimer` | Resource | race/lifecycle | 3-second countdown timer (inserted on Countdown, removed on Racing) |
| `RaceClock` | Resource | race/timing | Elapsed race time, running flag |
| `CelMaterial` | Asset | rendering/cel_material | Cel-shading material with halftone transition and hue-shifted highlights/shadows |
| `SkyboxMaterial` | Asset | rendering/skybox | Procedural TRON night sky (stars, moon, neon horizon glow) |
| `CelLightDir` | Resource | rendering/mod | World-space light direction shared by all CelMaterial instances |
| `SkyboxEntity` | Component | rendering/skybox | Marker on the skybox sphere entity |
| `CameraState` | Resource | camera/switching | Current camera mode + FPV target drone standings index |
| `CameraMode` | Enum | camera/switching | Chase (pack follow), Fpv (drone-mounted), Spectator (free-fly), CourseCamera(usize) (placed cameras) |
| `CourseCameras` | Resource | camera/switching | Course camera entries built from CourseData at race start (primary first) |
| `CourseCameraEntry` | Data | camera/switching | Pre-computed Transform + optional label for a placed course camera |
| `CameraInstance` | Data | course/data | Serialized camera placement: translation, rotation, is_primary, optional label |
| `ChaseState` | Resource | camera/chase | Smoothed center/velocity for broadcast-style pack-follow camera |
| `SpectatorSettings` | Resource | camera/spectator | Movement speed + mouse sensitivity |
| `RaceResults` | Resource | race/progress | Snapshot of final standings, persists Race→Results state transition |
| `RaceResultEntry` | Data | race/progress | Per-drone result: index, finished, finish_time, crashed, gates_passed |
| `ResultsTransitionTimer` | Resource | race/lifecycle | Brief delay (0.5s) before auto-transitioning Race→Results |
| `AvailableCourses` | Resource | menu/ui | Discovered course files (Menu state only) |
| `SelectedCourse` | Resource | course/loader | User's course selection for racing |
| `WorkshopState` | Resource | editor/workshop | Current obstacle being edited (scene, trigger config, preview) |
| `PreviewObstacle` | Component | editor/workshop | Marker on the 3D preview entity in the workshop |
| `PlacementState` | Resource | editor/course_editor | Selected palette obstacle/prop, active tab, dragging entity, drag height, gate order mode |
| `PlacedObstacle` | Component | editor/course_editor | Marker on every obstacle entity spawned in the course editor; carries `obstacle_id` and `gate_order` |
| `PlacedProp` | Component | editor/course_editor | Marker on every prop entity spawned in the course editor; carries `PropKind` and optional `color_override` |
| `PlacedCamera` | Component | editor/course_editor | Marker on every camera entity spawned in the course editor; carries `is_primary` and optional `label` |
| `EditorTab` | Enum | editor/course_editor | Obstacles (default), Props, or Cameras — switches the left-panel palette |
| `PropEditorMeshes` | Resource | editor/course_editor/ui | Shared mesh+material handles for prop placeholder cubes in the editor |
| `CameraEditorMeshes` | Resource | editor/course_editor/ui | Shared mesh+material handles for camera placeholder cubes in the editor (sky/sunshine colors) |
| `CameraPreview` | Resource | editor/course_editor/preview | Holds camera entity for render-to-texture PiP preview |
| `PreviewCamera` | Component | editor/course_editor/preview | Marker on the secondary Camera3d used for PiP render-to-texture |
| `PropKind` | Enum | course/data | ConfettiEmitter or ShellBurstEmitter — firework emitter type |
| `PropInstance` | Data | course/data | Per-prop placement: kind, translation, rotation, optional color_override |
| `FireworkEmitter` | Component | drone/fireworks | Race-time marker entity spawned from course props; carries `PropKind` and optional `Color` override |
| `PreviousTranslation` | Component | drone/interpolation | Drone translation from previous FixedUpdate tick (for visual interpolation) |
| `PreviousRotation` | Component | drone/interpolation | Drone rotation from previous FixedUpdate tick (for visual interpolation) |
| `PhysicsTranslation` | Component | drone/interpolation | Authoritative physics translation saved after each FixedUpdate tick |
| `PhysicsRotation` | Component | drone/interpolation | Authoritative physics rotation saved after each FixedUpdate tick |
| `DroneAssets` | Resource | drone/spawning | Shared mesh/material handles for all drone entities (from glTF or placeholder) |
| `DroneGltfHandle` | Resource | drone/spawning | Handle to the loaded drone glTF asset |
| `DesiredPosition` | Component | drone/components | AI→PID bridge: target position + velocity hint + curvature-aware speed limit |
| `DronePhase` | Component | drone/components | Per-drone lifecycle: Idle, Racing, Returning, Wandering, Crashed |
| `WanderState` | Component | drone/components | Per-drone wandering state: target position, dwell timer, step counter |
| `WanderBounds` | Resource | drone/ai | Bounding box for post-race wandering area (computed from course obstacle positions + padding) |
| `ExplosionParticle` | Component | drone/explosion | Velocity, lifetime, remaining time, and `ParticleKind` (Debris/HotSmoke/DarkSmoke) for crash particles |
| `ExplosionSounds` | Resource | drone/explosion | 4 handles to explosion audio variants (assets/sounds/explosion_{1..4}.wav) |
| `ExplosionMeshes` | Resource | drone/explosion | Pre-allocated mesh handles for debris (3 sizes), hot smoke, dark smoke — shared across all explosions |
| `FireworkParticle` | Component | drone/fireworks | Velocity, lifetime, remaining time, and `FireworkKind` (Spark/Willow/Confetti) for victory firework particles |
| `FireworkMeshes` | Resource | drone/fireworks | Pre-allocated mesh handles for spark, willow, confetti particle sizes |
| `FireworkSounds` | Resource | drone/fireworks | Handle to firework burst audio (assets/sounds/firework.wav) |
| `FireworksTriggered` | Resource | drone/fireworks | Marker preventing re-triggering of fireworks after first drone finishes |
| `PendingShell` | Component | drone/fireworks | Staggered detonation timer for overhead shell bursts (position, delay, colors) |
| `ReturnPath` | Component | drone/components | Non-cyclic spline for post-race return flight (inserted Racing→Returning, removed Returning→Idle) |
| `AiTuningParams` | Resource | drone/components | Runtime-tunable AI/physics constants (14 params). Persists across race restarts. Exposed via dev dashboard (F4) |
| `LeaderboardRoot` | Component | race/ui | Marker on the race leaderboard panel (top-left standings display, 12 rows with color bars, names, times) |
| `Pilot` | Data | pilot/mod | Persistent pilot identity: gamertag, personality traits, skill profile, color scheme, stats |
| `PilotId` | Data | pilot/mod | Unique u64 identifier for a pilot |
| `PilotRoster` | Resource | pilot/roster | All generated pilots, persisted to `assets/pilots/roster.pilots.ron` |
| `SelectedPilots` | Resource | pilot/mod | 12 pilots chosen for the current race, indexed by drone slot |
| `PilotConfigs` | Resource | pilot/mod | Pre-computed DroneConfigs from selected pilots' skill+personality |
| `DroneIdentity` | Component | drone/components | Per-drone name and color, set from SelectedPilots at spawn |
| `PersonalityTrait` | Enum | pilot/personality | Aggressive, Cautious, Flashy, Methodical, Reckless, Smooth, Technical, Hotdog |
| `SkillProfile` | Data | pilot/skill | Per-pilot skill: level + speed/cornering/consistency axes |
| `PortraitDescriptor` | Data | pilot/portrait | Face/eyes/mouth/hair/shirt/accessory slots + colors (6 slot enums), `generate()` for random creation |
| `PortraitCache` | Resource | pilot/portrait/cache | Cached `Handle<Image>` per pilot, persists across races, built `OnEnter(Race)` |
| `PortraitPaletteConfig` | Resource | dev_menu/portrait_config | Per-slot vetoed color indices and complementary color mappings, persisted to RON |
| `PortraitColorSlot` | Enum | dev_menu/portrait_config | Color pool categories: Skin, Hair, Eye, Shirt, Accessory |
| `PortraitEditorState` | Resource | dev_menu/portrait_editor | Active tab, variant selections, color selections, preview dirty flag |
