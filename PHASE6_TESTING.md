# Phase 6 Manual Testing Feedback Form

## Drone Spawning

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 1 | Drones spawn on race start | Select a course with gates, enter Race mode | 12 drone entities appear near the first gate in a 3x4 grid formation | |
| 2 | Placeholder without .glb | Remove/rename `drone.glb`, enter Race | Small red cubes appear instead of drone models | |
| 3 | Drones face first gate | Enter Race, observe drone orientations at spawn | All drones face toward the first gate | |
| 4 | No gates = no drones | Race with a course that has no gates | Warning in console, no drones spawn | |

## Physics

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 5 | Drones move toward gates | Watch drones after spawn | Drones accelerate toward the first gate waypoint | |
| 6 | Hover stability | Drones near a waypoint | Drones settle near the target without wild oscillation | |
| 7 | Visual lean | Drones moving at speed | Drones tilt in direction of travel (~30 deg max) | |
| 8 | Ground clamp | Lower a gate below ground (in editor) | Drones don't go below ground plane | |

## AI Racing

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 9 | Gate progression | Watch drones race | Drones visit gates in order (gate 0, 1, 2, ...) | |
| 10 | Racing line variation | Watch multiple drones | Drones take slightly different paths (lateral offsets, weaving) | |
| 11 | Smooth cornering | Drone approaching a turn | Drone starts turning before reaching the gate (look-ahead blend) | |
| 12 | Course completion | All gates visited | Drones hold position after passing all waypoints | |

## State Transitions

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 13 | DespawnOnExit cleanup | Exit Race (back to Menu) | All drone entities despawned, no orphan meshes | |
| 14 | Re-enter Race | Go Menu, then Race again | Fresh 12 drones spawn, no leftover state | |

## Edge Cases

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 15 | Single gate course | Course with exactly 1 gate | Drones spawn, fly to gate, then hold position | |
| 16 | Rapid state toggle | Enter/exit Race quickly multiple times | No crashes, no resource leaks | |

## Performance

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 17 | Frame rate during race | Monitor FPS with 12 drones flying | Stable 60fps, no hitches | |
| 18 | FixedUpdate consistency | Observe drone movement smoothness | No jitter or teleporting | |

## Regression

| # | Test | Action | Expected Result | Pass / Fail / Notes |
|---|------|--------|-----------------|---------------------|
| 19 | Menu still works | Navigate menu, select courses | No change in behavior | |
| 20 | Editor still works | Open Workshop and Course Editor | No change in behavior | |
| 21 | Spectator camera in Race | WASD + mouse look during Race | Camera works as before alongside drones | |
