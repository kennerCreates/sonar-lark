# Implementation TODO

## Completed

- [x] **Phase 1**: States, common, main.rs wiring — state machine skeleton
- [x] **Phase 2**: Obstacle and course data layer + RON serialization
- [x] **Documentation**: CLAUDE.md updated, ARCHITECTURE.md created
- [x] **Phase 3**: Menu UI + Spectator Camera enhancements
- [x] **Phase 4**: Editor — Obstacle Workshop (WorkshopState, scene browser, trigger gizmo, save/load/delete)
- [x] **Testing**: Unit tests for obstacle library, course data, and menu discovery (22 tests)
- [x] **Bugfix**: Workshop preview placeholder spawn race condition with deferred despawns
- [x] **Phase 5**: Editor — Course Editor (PlacementState, palette UI, click-to-place, XZ drag, Q/E height, gate ordering, trigger gizmos, gate sequence lines, save/load)
- [x] **Phase 6**: Drone Physics + AI (DroneAssets, spawn 12 drones with randomized PID/configs, PID-lite physics in FixedUpdate, AI waypoint tracking + racing line noise, DespawnOnExit cleanup)
- [x] **Phase 6b**: Drone Realism Audit — motor lag 40→25ms, attitude PD underdamping (kp=7/kd=0.20), per-drone cornering aggression/braking distance/attitude PD variation, adaptive approach offset, dirty air perturbation, prop wash (faked), battery sag, dev dashboard expanded to 11 params
- [x] **Phase 7**: Race — Gate Validation, Timing, Lifecycle (RaceProgress per-drone tracking, RaceClock, AABB gate trigger detection, gate ordering enforcement, hard crash on missed gate, 3-second countdown sequence, race completion detection, countdown + clock UI)

### Phase 8: Results, FPV Camera, Chase Camera
- [ ] Results UI: display race standings (finish time, crashed status)
- [ ] Results navigation: buttons for back-to-menu, replay
- [ ] `DespawnOnExit(AppState::Results)` on results UI
- [ ] FPV camera: mount on target drone transform
- [ ] Chase camera: follow behind/above with smoothing
- [ ] Camera switching: keybinds to cycle mode (Spectator/FPV/Chase)
- [ ] Camera switching: keybinds to cycle target drone
- [ ] Full loop test: Menu → Editor → Race → Results → Menu

## Future (Post-MVP)
- [ ] Player-controlled drone (same throttle/pitch/roll/yaw interface as AI)
- [ ] Per-drone customization (motor thrust, weight, drag, frame size)
- [ ] Multiple obstacle types beyond gates
- [ ] Multi-lap races
- [ ] Crash behavior decision: instant DNF vs. respawn with time penalty
- [ ] Terrain elevation
- [ ] Gamepad support
- [ ] Drone visual models from Blender (replace placeholders)
