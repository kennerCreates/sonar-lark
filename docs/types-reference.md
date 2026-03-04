# Key Types Reference

Do not read this file in full — use `grep` to find specific types. This is a lookup index, not a narrative doc.

## Obstacles & Courses

| Type | Kind | Purpose |
|------|------|---------|
| `ObstacleId` | Data | Unique string ID for obstacle types |
| `ObstacleDef` | Data | glb scene name + trigger volume config |
| `ObstacleLibrary` | Res | All loaded obstacle definitions |
| `CourseData` | Res | All obstacle/prop/camera placements for a course |
| `TriggerVolume` | Comp | AABB hitbox on gate entities |
| `CollisionVolumeConfig` | Data | Local-space AABB for obstacle collision (Vec per obstacle) |
| `CollisionVolumeEntry` | Data | Single collision box within an `ObstacleCollisionVolumes` |
| `ObstacleCollisionVolumes` | Comp | Runtime collision volumes (compound) on obstacle entities |
| `ObstacleCollisionCache` | Res | Cached world-space OBBs, built once at race start |

## Race

| Type | Kind | Purpose |
|------|------|---------|
| `GateIndex` | Comp | Gate sequence order |
| `GateForward` | Comp | World-space forward direction for gate validation |
| `GatePlanes` | Res | Cached per-gate plane data for crossing detection |
| `RaceProgress` | Res | Per-drone gate/finish/crash tracking |
| `DroneRaceState` | Data | Per-drone: next_gate, gates_passed, finished, crashed, dnf_reason |
| `RacePhase` | Res | WaitingToStart / Countdown / Racing / Finished |
| `CountdownTimer` | Res | 3-second countdown (inserted on Countdown, removed on Racing) |
| `RaceClock` | Res | Elapsed race time + running flag |
| `RaceResults` | Res | Final standings snapshot, persists Race->Results |
| `RaceResultEntry` | Data | Per-drone: finished, finish_time, crashed, gates_passed |
| `ResultsTransitionTimer` | Res | 0.5s delay before Race->Results |
| `LeaderboardRoot` | Comp | Marker on race leaderboard panel |

## Drone

| Type | Kind | Purpose |
|------|------|---------|
| `DesiredPosition` | Comp | AI->PID bridge: target pos + velocity hint + speed limit |
| `DronePhase` | Comp | Idle / Racing / Returning / Wandering / Crashed |
| `DroneIdentity` | Comp | Per-drone name and color (from SelectedPilots) |
| `WanderState` | Comp | Post-race wandering target, dwell timer, step counter |
| `WanderBounds` | Res | Bounding box for post-race wandering |
| `ReturnPath` | Comp | Non-cyclic spline for post-race return flight |
| `AiTuningParams` | Res | 14 runtime-tunable AI/physics constants (F4 dashboard) |
| `DroneAssets` | Res | Shared mesh/material handles for all drones |
| `DroneGltfHandle` | Res | Handle to loaded drone glTF |
| `PreviousTranslation/Rotation` | Comp | Previous FixedUpdate tick (for visual interpolation) |
| `PhysicsTranslation/Rotation` | Comp | Authoritative physics state after FixedUpdate |

## Effects

| Type | Kind | Purpose |
|------|------|---------|
| `ExplosionParticle` | Comp | Crash particle: velocity, lifetime, ParticleKind |
| `ExplosionMeshes/Sounds` | Res | Pre-allocated explosion assets |
| `FireworkParticle` | Comp | Victory particle: velocity, lifetime, FireworkKind |
| `FireworkMeshes/Sounds` | Res | Pre-allocated firework assets |
| `FireworksTriggered` | Res | Prevents re-triggering fireworks |
| `PendingShell` | Comp | Staggered detonation timer for shell bursts |
| `FireworkEmitter` | Comp | Race-time entity from course props |

## Rendering & Camera

| Type | Kind | Purpose |
|------|------|---------|
| `CelMaterial` | Asset | Cel-shading material with halftone + hue-shifted highlights |
| `SkyboxMaterial` | Asset | Procedural TRON night sky |
| `CelLightDir` | Res | Shared light direction for all CelMaterial |
| `CameraState` | Res | Current camera mode + FPV target index |
| `CameraMode` | Enum | Chase / Fpv / Spectator / CourseCamera(usize) |
| `CourseCameras` | Res | Course camera entries built at race start |
| `ChaseState` | Res | Smoothed center/velocity for pack-follow camera |
| `SpectatorSettings` | Res | Movement speed + mouse sensitivity |
| `CameraPreview` | Res | PiP render-to-texture camera entity |

## Editor

| Type | Kind | Purpose |
|------|------|---------|
| `WorkshopState` | Res | Current obstacle being edited |
| `PreviewObstacle` | Comp | Marker on 3D preview entity in workshop |
| `PlacementState` | Res | Selected palette item, active tab, drag state, gate order mode |
| `PlacedObstacle/Prop/Camera` | Comp | Markers on entities in course editor |
| `EditorTab` | Enum | Obstacles / Props / Cameras |
| `PropEditorMeshes` | Res | Shared handles for prop placeholder cubes |
| `CameraEditorMeshes` | Res | Shared handles for camera placeholder cubes |

## Pilots

| Type | Kind | Purpose |
|------|------|---------|
| `Pilot` | Data | Gamertag, personality, skill, color scheme, stats |
| `PilotId` | Data | Unique u64 identifier |
| `PilotRoster` | Res | All generated pilots, persisted to RON |
| `SelectedPilots` | Res | 12 pilots for current race |
| `PilotConfigs` | Res | Pre-computed DroneConfigs from skill+personality |
| `PersonalityTrait` | Enum | Aggressive/Cautious/Flashy/Methodical/Reckless/Smooth/Technical/Hotdog |
| `SkillProfile` | Data | Level + speed/cornering/consistency axes |
| `PortraitDescriptor` | Data | Face/eyes/mouth/hair/shirt/accessory slots + colors |
| `PortraitCache` | Res | Cached portrait images, built OnEnter(Race) |
| `PortraitPaletteConfig` | Res | Per-slot vetoed colors + complementary mappings |
| `PortraitEditorState` | Res | Dev menu portrait editor state |

## Menu

| Type | Kind | Purpose |
|------|------|---------|
| `AvailableCourses` | Res | Discovered course files (Menu state) |
| `SelectedCourse` | Res | User's course selection for racing |
| `PropKind` | Enum | ConfettiEmitter / ShellBurstEmitter |
| `CameraInstance` | Data | Serialized camera placement |
| `PropInstance` | Data | Serialized prop placement |
