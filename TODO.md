# Implementation TODO

## Completed

- [x] **Skeleton**: States, common, main.rs wiring — state machine skeleton
- [x] **Data**: Obstacle and course data layer + RON serialization
- [x] **Documentation**: CLAUDE.md updated, ARCHITECTURE.md created
- [x] **UI Camera**: Menu UI + Spectator Camera enhancements
- [x] **Obstacle Workshop**: Editor — Obstacle Workshop (WorkshopState, scene browser, trigger gizmo, save/load/delete)
- [x] **Testing**: Unit tests for obstacle library, course data, and menu discovery (22 tests)
- [x] **Bugfix**: Workshop preview placeholder spawn race condition with deferred despawns
- [x] **Course Editor**: Editor — Course Editor (PlacementState, palette UI, click-to-place, XZ drag, Q/E height, gate ordering, trigger gizmos, gate sequence lines, save/load)
- [x] **Drone Physics**: Drone Physics + AI (DroneAssets, spawn 12 drones with randomized PID/configs, PID-lite physics in FixedUpdate, AI waypoint tracking + racing line noise, DespawnOnExit cleanup)
- [x] **Drone Realism**: Drone Realism Audit — motor lag 40→25ms, attitude PD underdamping (kp=7/kd=0.20), per-drone cornering aggression/braking distance/attitude PD variation, adaptive approach offset, dirty air perturbation, prop wash (faked), battery sag, dev dashboard expanded to 11 params
- [x] **Race**: Race — Gate Validation, Timing, Lifecycle (RaceProgress per-drone tracking, RaceClock, AABB gate trigger detection, gate ordering enforcement, hard crash on missed gate, 3-second countdown sequence, race completion detection, countdown + clock UI)
- [x] **Rendering Overhaul**: Cel-shaded materials with halftone gradient effect and hue shifting (warm highlights, cool shadows). Procedural TRON night skybox (stars, moon, neon horizon glow). Custom WGSL shaders. All spawn points (ground, obstacles, drones) refactored from StandardMaterial to CelMaterial. Explosions unchanged (unlit emissive).
- [x] **Results Chase Camera**: Results, FPV Camera, Chase Camera — Race results screen with standings/times/DNF, auto-transition Race→Results, RACE AGAIN/MAIN MENU buttons, pack-follow chase camera (default), FPV drone-mounted camera with standings-order cycling, camera mode switching (C key), camera HUD indicator, full gameplay loop (Menu→Race→Results→Menu)
- [x] **Course Props**: Firework emitter props in course editor — tabbed UI (Obstacles/Props), PropKind (ConfettiEmitter/ShellBurstEmitter), placement with color override, save/load in CourseData, race-time FireworkEmitter spawning, detect_first_finish uses placed emitters (falls back to auto gate-0 if none placed)

## Backlog (Quality / Polish)
- [ ] Display `gates_passed` in Results UI (data already in `RaceResultEntry.gates_passed`)
- [ ] Directional gate validation — penalize or DNF drones that pass through gates backwards (using `GateForward` component, already stored)
- [ ] Deferred `RaceProgress` insertion timing — currently created at countdown end; if a drone could somehow reach gate 0 during countdown, the gate pass would be missed. Low risk (drones start behind gates) but could be tightened by inserting `RaceProgress` earlier.

## Backlog (Code Health)
- [x] Split `editor/course_editor/ui.rs` → `ui/` directory (types.rs, build.rs, file_ops.rs, systems.rs). Split `editor/course_editor/mod.rs` → extract `overlays.rs` and `transform_gizmos.rs`. Split `editor/workshop/ui.rs` → `ui/` directory (build.rs, systems.rs).
- [x] Replace async poll loading pattern (drone/obstacle glTF) with `AssetServer::is_loaded_with_dependencies()` run conditions — removes per-frame early-return guards.
- [ ] Add unit tests for pure editor utility functions: `CourseData` construction from placed entities, transform gizmo math in `editor/gizmos.rs`, course discovery.
- [ ] Add change detection to `draw_flight_spline_preview` — skip spline rebuild when no obstacles have moved/added/removed (e.g., dirty flag or `Changed<Transform>` on placed obstacles).

## Future (Post-MVP)
- [ ] Player-controlled drone (same throttle/pitch/roll/yaw interface as AI)
- [ ] Per-drone customization (motor thrust, weight, drag, frame size)
- [ ] Multiple obstacle types beyond gates
- [ ] Multi-lap races
- [x] Crash behavior decision: instant DNF vs. respawn with time penalty
- [ ] Terrain elevation
- [ ] Gamepad support
- [x] Drone visual models from Blender (replace placeholders)
