# Implementation TODO

## Completed

- [x] **Phase 1**: States, common, main.rs wiring — state machine skeleton
- [x] **Phase 2**: Obstacle and course data layer + RON serialization
- [x] **Documentation**: CLAUDE.md updated, ARCHITECTURE.md created
- [x] **Phase 3**: Menu UI + Spectator Camera enhancements
- [x] **Phase 4**: Editor — Obstacle Workshop (WorkshopState, scene browser, trigger gizmo, save/load/delete)

### Phase 5: Editor — Course Editor
- [ ] `PlacementState` resource (selected obstacle, dragging entity, drag height)
- [ ] Obstacle palette UI: browse library, select obstacle to place
- [ ] Click-to-place: raycast to ground plane, spawn obstacle instance
- [ ] XZ drag to reposition placed obstacles
- [ ] Separate Y height control (scroll wheel or key)
- [ ] Gate ordering UI: click gates to set sequence order
- [ ] Gizmo: trigger volume wireframes on all placed obstacles
- [ ] Gizmo: lines connecting gates in sequence order
- [ ] Sync entity transforms back to `CourseData` resource
- [ ] Save course to RON file
- [ ] Load existing course for editing

### Phase 6: Drone Physics + AI
- [ ] `DroneAssets` resource: shared placeholder mesh + material
- [ ] `spawn_drones`: 12 drones at start line with randomized `DroneConfig`
  - Vary PID gains (±15%), line offset, noise amplitude/frequency
- [ ] `DespawnOnExit(AppState::Race)` on all drone entities
- [ ] PID controller: compute error from desired vs actual orientation/position
- [ ] `apply_forces`: convert PID output to thrust and torque
- [ ] `integrate_motion`: Euler integration, gravity, drag
- [ ] `clamp_transform`: prevent drones going below ground
- [ ] All physics systems in `FixedUpdate`, `.chain()`-ed
- [ ] AI: `update_ai_targets` — advance waypoint when close enough
- [ ] AI: `compute_racing_line` — apply per-drone noise + lateral offset
- [ ] Generate initial waypoints from gate positions in `CourseData`

### Phase 7: Race — Gate Validation, Timing, Lifecycle
- [ ] `RaceProgress` resource: per-drone state (next gate, gates passed, crashed, finished, time)
- [ ] `RaceClock` resource: elapsed time, running flag
- [ ] Gate trigger check system: AABB overlap between drones and trigger volumes
- [ ] Gate ordering enforcement: must pass gates in sequence
- [ ] Crash detection: drone hits gate geometry or misses next gate
- [ ] Race countdown sequence on `OnEnter(AppState::Race)`
- [ ] `check_race_complete`: all drones finished or crashed → `AppState::Results`
- [ ] Race clock tick system

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
