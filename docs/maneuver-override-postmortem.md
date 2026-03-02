# Maneuver Override System — Postmortem

Attempted and reverted on 2026-03-02. Commits `62e92cb..35dc46f` (7 commits, ~1900 lines), reverted in `ef2580b`.

## Goal

Replace the current "car-like" banking turns for AI drones with realistic attitude-based maneuvers (Split-S, Power Loop, Aggressive Bank) for tight turns. Real racing drones flip/loop to reorient thrust through 180° in ~160ms rather than flying wide arcs. The system aimed to replicate this.

## What Was Built

### New module: `src/drone/maneuver/` (5 files)

- **`detection.rs`** — Scanned the spline ahead of each drone, sampled tangent directions, and identified sharp turns (>120° = flip maneuver, 70-120° = aggressive bank). Had guards for speed, cooldown, and gate proximity.
- **`trigger.rs`** — Activated maneuvers at the right spline position. Computed entry/exit positions and velocities, built trajectory curves, and attached `ActiveManeuver` components.
- **`trajectory.rs`** (originally `profiles.rs`, then `execution.rs`) — Generated Hermite spline trajectories for Split-S (3-point: entry → apex → exit) and Power Loop (4-point with climb and inverted apex). Duration scaled by drone speed.
- **`cleanup.rs`** — Removed `ActiveManeuver` components when maneuvers completed (by elapsed time or distance to exit point).
- **`mod.rs`** — Plugin registration, `ManeuverKind` enum, `ManeuverPhaseTag`, system ordering in `FixedUpdate`.

### Other changes

- **`drone/components.rs`** — Added `ActiveManeuver` and `TiltOverride` components.
- **`drone/physics.rs`** — Modified `position_pid` to skip drones with `ActiveManeuver` (via `Without<>` filter), and read `TiltOverride` to raise the 83° tilt clamp for aggressive banks.
- **`drone/ai/racing_line.rs`** — Modified speed computation to not brake before maneuver points.
- **`pilot/personality.rs` + `pilot/skill.rs`** — Added `maneuver_aggression` trait and skill mapping.
- **`drone/debug_draw.rs`** — Visualized maneuver trajectories as gizmo lines.
- **`PLAN.md` + `docs/drone-movement-research.md`** — Detailed plan and 303-line research doc on real-world drone physics.
- **`assets/courses/tight_course.course.ron`** — Test course with tight turns.

## Architecture

```
              ┌──────────────────────────────────┐
              │ trigger_maneuvers                 │
              │ (detects tight turns, inserts     │
              │  ActiveManeuver or TiltOverride)  │
              └──────────┬───────────────────────┘
                         │
       ┌─────────────────┼─────────────────┐
       │ Has ActiveManeuver?               │
       │                                   │
 ┌─────▼──────┐                    ┌───────▼──────┐
 │ execute_    │                   │ position_pid  │
 │ maneuvers   │                   │ (+ TiltOver-  │
 │ (writes     │                   │  ride if set) │
 │ Desired-    │                   │ (normal path) │
 │ Attitude    │                   └───────┬───────┘
 │ directly)   │                           │
 └─────┬───────┘                           │
       │                                   │
       └──────────┬────────────────────────┘
                  │
          DesiredAttitude
                  │
          attitude_controller → motor_lag → apply_forces → integrate_motion
```

Two integration paths, same downstream physics:
- **Split-S / Power Loop:** `ActiveManeuver` component, full PID bypass, direct attitude+thrust control
- **Aggressive Bank:** `TiltOverride` component, PID still runs with raised limit (~103°)

## What Went Wrong

The system went through several iterations, none fully successful:

1. **Initial implementation** (`62e92cb`) — Drones crashed during maneuvers. The PID bypass + direct attitude control caused unstable flight.
2. **Cleanup fixes** (`7960952`) — Still broken. Improved cleanup logic but core instability remained.
3. **More investigation** (`09405c5`) — Added gate proximity guards, debug systems, tight test course. Still crashing.
4. **Fixed crashes** (`3d9fde6`) — Drones stopped crashing but were *slower* with maneuvers than without. The trajectory curves weren't matching drone capabilities.
5. **Faster maneuvers** (`250d34e`) — Sped up maneuvers but drones started missing gates.
6. **Removed PID override** (`81bbb5c`) — Stripped back to just trigger detection without the full PID bypass. Fewer maneuvers, fewer crashes, but defeated the purpose.
7. **Back to crashes** (`35dc46f`) — Major rewrite replacing `profiles.rs`/`execution.rs` with `trajectory.rs`, simplifying the approach. Still crashing.

## Core Tensions That Need Resolving

1. **PID bypass vs. stability** — Skipping `position_pid` during flips is necessary (the tilt clamp and thrust calc break when inverted), but the maneuver trajectory system wasn't producing stable enough attitude targets to fly without PID help.
2. **Speed vs. gate accuracy** — Faster maneuvers meant less time to correct course, causing gate misses. The exit positions/velocities from the trajectory didn't align well enough with the spline to resume normal flight cleanly.
3. **Maneuver-to-normal handoff** — The transition back from maneuver control to PID control was abrupt, causing oscillation or crashes at the recovery point.

## Preserved Artifacts

All code remains accessible in git history:

- Full source: `git show 62e92cb:src/drone/maneuver/detection.rs` (etc.)
- Research doc: `git show 62e92cb:docs/drone-movement-research.md` — real-world drone physics reference (turn radii, maneuver catalog, underactuation constraints)
- Plan: `git show 62e92cb:PLAN.md` — architectural analysis of which systems are/aren't compatible with inversion
- Test course: `git show 09405c5:assets/courses/tight_course.course.ron`
- Uncommitted WIP: `git stash list` (stash 0)
