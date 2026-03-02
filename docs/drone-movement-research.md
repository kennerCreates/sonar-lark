# Drone Movement Research — Aggressive Turns & Acrobatic Maneuvers

Research document for adding realistic tight-turn behavior to AI drones. Covers real-world FPV racing physics, the specific maneuvers pilots use, why loops/flips beat banking at tight corners, and how this maps to our existing simulation.

## Table of Contents

1. [Why Drones Can't "Just Turn"](#1-why-drones-cant-just-turn)
2. [The Maneuver Catalog](#2-the-maneuver-catalog)
3. [Why Loops Beat Banking](#3-why-loops-beat-banking)
4. [What Our Sim Currently Does](#4-what-our-sim-currently-does)
5. [The Gap](#5-the-gap)
6. [Implementation Approaches](#6-implementation-approaches)
7. [Sources](#7-sources)

---

## 1. Why Drones Can't "Just Turn"

### The Underactuation Constraint

A quadrotor has 6 degrees of freedom (x, y, z, roll, pitch, yaw) but only 4 control inputs (four motor speeds). Those inputs map to:

- **Total thrust** — always perpendicular to the rotor plane, pointing "up" in body frame
- **Roll torque** — tilts left/right
- **Pitch torque** — tilts forward/back
- **Yaw torque** — spins around vertical axis

The critical constraint: **thrust is always body-up.** A drone has no wings, no lift surfaces. The only way to accelerate horizontally is to tilt so the thrust vector gains a horizontal component:

```
F_world = Rotation(attitude) * [0, 0, thrust]^T + [0, 0, -mg]^T
```

Every directional change requires reorienting the aircraft *first*. This is what makes drone turning fundamentally different from cars (which steer wheels) or planes (which deflect control surfaces while maintaining lift).

### Yaw Doesn't Turn

Counterintuitively, **yawing does not change the drone's flight path.** Spinning around the body's Z-axis leaves the thrust direction unchanged in world coordinates — still pointing "up" relative to the body, which is still "up" relative to the world if level. To curve the flight path, the drone must roll or pitch. Yaw only changes which direction the nose points, not where the drone accelerates.

### The Banking Turn Radius Problem

For a drone in a banked turn maintaining altitude:

```
Turn radius:  r = v^2 / (g * tan(bank_angle))
```

At high speed, the radius explodes:

| Speed     | 45° bank  | 60° bank  | 80° bank  |
|-----------|-----------|-----------|-----------|
| 50 km/h   | 19.7 m    | 11.4 m    | 3.5 m     |
| 100 km/h  | 78.7 m    | 45.4 m    | 14.2 m    |
| 150 km/h  | 177 m     | 102 m     | 31.9 m    |

Even at 80° bank (extreme), a drone at race speed needs 14+ meters to do a 180. On a tight course with gates a few meters apart, conventional banking can't cut it.

### What Racing Pilots Actually Do

When the turn is tighter than what banking allows at current speed, pilots switch to **attitude-based maneuvers** — they flip or loop the drone, reorienting the thrust vector through 180° in body rotation rather than flying a spatial arc. At 1150 deg/s roll rate (our sim's current value), a 180° reorientation takes ~160 ms. That's faster than flying a 30-meter semicircle.

---

## 2. The Maneuver Catalog

### 2A. Banked Turn (conventional)

The drone rolls to one side, creating a horizontal thrust component for centripetal force. Coordinated yaw keeps the nose along the flight path. Throttle increases to compensate lost vertical thrust.

**When used:** Gentle to moderate curves where there's room for a wide arc. The bread-and-butter turn for sweeping bends.

**Trade-offs:** Smooth and altitude-maintaining, but radius is speed-dependent. At high speed + tight turn, the drone runs out of thrust budget (needs centripetal force AND altitude maintenance simultaneously).

### 2B. Split-S (the hairpin flip)

The signature tight-turn maneuver in drone racing. This is likely what you're seeing in videos.

**Sequence:**
1. Half-roll (invert the drone — ~80 ms at race speeds)
2. Pull back on pitch while inverted (pulls the nose down and through)
3. Reduce throttle while inverted (gravity assists; full throttle would slam into the ground)
4. Throttle up as the drone comes through the bottom of the arc
5. Exit flying in the opposite direction, at lower altitude but often higher speed

**Why it works:** The Split-S exploits gravity as a free acceleration source. When inverted and diving, the drone converts altitude to speed. It gains speed *through* the turn rather than losing it — impossible in a pure banking turn. The Drone Racing League identifies it as a core racing element.

**Visual signature:** The drone appears to do a quick half-flip, dives briefly, then swoops out level going the other way. Total maneuver time: 0.5–1.5 seconds depending on entry speed and altitude budget.

**Risk:** If the pilot cuts throttle too aggressively during the roll, the entry angle steepens dangerously. The pilot must "punch" throttle to avoid hitting the ground, losing all momentum. A smooth parabolic line is essential.

### 2C. Power Loop (the vertical reversal)

A full backward loop — climb while pitching backward through 360°.

**Sequence:**
1. Approach at speed
2. Pull back on pitch + increase throttle (climbing arc)
3. Continue pitch past 90° (going up and backward), reduce throttle near the top
4. Past 180° (inverted, at peak altitude), smoothly increase throttle
5. Complete the loop, resume forward flight

**Why it works:** Trades speed for altitude on the way up, regains it on the way down. The drone needs enough tangential velocity at the top to avoid free-fall: `v_critical = sqrt(g * r_loop)`. Below this, the drone goes ballistic and loses control authority.

**When used:** Flying over obstacles or through vertically-stacked gates. Also when approaching a gate from below and needing to exit above (or vice versa). Both a racing technique and a freestyle crowd-pleaser.

**Visual signature:** A large, smooth backward loop. The radius depends on entry speed — faster entry = larger loop.

### 2D. Power Split (power loop → split-S combo)

Initiates a power loop but transitions into a split-S partway through. Takes the drone up and over an obstacle, then reverses direction.

**When used:** Complex 3D track sections with gates at different heights requiring both vertical and directional changes.

### 2E. Snap Turn / Juicy Snap

A quick snap in one direction followed by an immediate flick back. Creates a "juking" motion — the drone briefly redirects then snaps back onto line.

**When used:** Quick directional micro-adjustments between gates. Dodging obstacles. Resetting attitude after a perturbation.

### 2F. Yaw Pivot

At speed, the pilot cuts throttle, yaws 180°, then punches throttle in the new direction. Unlike the Split-S, no altitude is traded.

**When used:** Tight reversals where altitude must be maintained. Less efficient than a Split-S (doesn't exploit gravity) but useful in altitude-constrained sections.

---

## 3. Why Loops Beat Banking

### 3A. Thrust Vector Reorientation Speed

Banking turn reversal at 100 km/h, 80° bank (best case): semicircle radius ~14 m, path length ~44 m, time ~1.6 seconds.

Flip-based reversal: 180° pitch at 1150 deg/s takes ~160 ms. During the flip the drone coasts ballistically (moves ~4.4 m forward at 100 km/h). After completing the flip, the full thrust vector points backward, decelerating and reversing. Total reversal time: ~0.8–1.2 seconds.

**The key insight:** For high-TWR drones, angular reorientation is faster than flying an arc. The higher the speed, the bigger the advantage — banking radius grows with v², but flip time is nearly constant.

### 3B. Gravity as Free Energy (Split-S)

In a Split-S, gravity accelerates the drone during the inverted descent phase — speed for free. A banking turn *fights* gravity (must maintain altitude with vertical thrust component). The Split-S exits faster than it entered; a banking turn always exits slower due to energy lost to centripetal force.

### 3C. Altitude as a Resource

Race courses are 3D. Gates sit at varying heights. A Split-S naturally loses altitude; a power loop gains it. Pilots choose the maneuver that matches the required altitude change, getting the direction reversal "for free" — they needed to change altitude anyway.

### 3D. Motor Saturation Budget

During an aggressive 80° bank at high speed, the drone needs nearly all its thrust for centripetal acceleration, with almost nothing left for altitude maintenance. Motors are saturated just holding the turn.

A flip-based maneuver distributes load differently: during the flip itself, motors primarily provide torque for rotation (much lower thrust). The drone is briefly ballistic, but the motors aren't fighting gravity and centripetal force simultaneously. Better duty cycle, less saturation.

### 3E. When Banking Is Still Better

- **Gentle curves (>30° direction change at moderate speed):** Banking is smoother, doesn't disrupt the flight path
- **Low altitude with no room to dive:** Split-S needs altitude budget
- **Maintaining a racing line through sweeping curves:** Smooth banking carries speed better on gradual turns

The crossover point depends on speed, turn angle, and altitude budget. Roughly: if the required banking angle exceeds ~70° at the current speed, a flip maneuver becomes faster.

---

## 4. What Our Sim Currently Does

Our current physics model is well-suited as a foundation:

| Feature | Current State | Relevant to Maneuvers? |
|---------|--------------|----------------------|
| Thrust-through-body | Yes — thrust along body-up | Core requirement met |
| Cascaded PID (position → attitude) | Yes | Needs bypass during maneuvers |
| TWR | ~7:1 (55N, 0.8kg) | Solidly in the racing regime |
| Roll/pitch rate | 20 rad/s (~1150 deg/s) | Fast enough for racing flips |
| Motor lag | 25 ms tau | Realistic |
| Quadratic drag | Yes (k=0.025) | Correct |
| Max tilt clamp | 1.45 rad (~83°) | **Blocks flips** — must be overridden |
| AI path following | Spline-based, curvature-aware speed | No awareness of acrobatics |
| Position PID | Always active | Would fight against flip maneuvers |

### The Current Turn Behavior

Right now, when a drone approaches a tight corner:

1. `compute_racing_line` reads ahead on the spline, detects high curvature
2. It reduces `max_speed` based on curvature (slow down for tight turns)
3. The position PID drives the drone toward the next spline point
4. The attitude controller banks into the turn, clamped at ~83°
5. The drone flies a wide arc, slowing down as needed

This produces **conservative, car-like cornering** — slow in, bank through, accelerate out. It looks reasonable but doesn't match what real racing drones do on tight courses.

---

## 5. The Gap

What's missing to get realistic tight-turn behavior:

### 5A. No Maneuver State Machine

The AI has no concept of "I'm executing a Split-S" vs "I'm in normal flight." Every tick it just follows the position PID toward the next spline point. There's no mechanism for it to deliberately enter an inverted state, temporarily ignore the spline, or sequence through a multi-phase maneuver.

### 5B. Max Tilt Clamp Prevents Full Rotation

`max_tilt_angle` (1.45 rad / ~83°) in `position_pid` prevents the drone from ever going past ~83° of tilt. For a Split-S or power loop, the drone needs to pass through 180° (fully inverted). The clamp would need to be disabled during acrobatic maneuvers.

### 5C. Position PID Would Fight the Maneuver

During a Split-S, the drone's position intentionally diverges from the spline (it dives and swings through). The position PID would immediately try to correct this deviation, producing a tug-of-war between the maneuver and the controller. The PID needs to be temporarily overridden.

### 5D. No Altitude-Aware Turn Selection

The AI doesn't consider altitude when choosing how to turn. A real pilot picks Split-S vs power loop vs banking based on: how tight the turn is, what altitude they're at, and what altitude the next gate requires.

---

## 6. Implementation Approaches

Three approaches, in order of increasing fidelity and complexity.

### Approach A: Spline Warping (Simplest)

Don't change the physics or controller at all. Instead, warp the spline itself so that tight-turn sections include vertical loops.

**How it works:**
- During spline generation, detect high-curvature sections (hairpins)
- Replace the flat-arc control points with loop-shaped control points (add vertical displacement that forms a loop or split-S arc)
- The existing AI + PID follows the spline as usual — it just happens to be a loop shape now

**Pros:** Minimal code change. No new state machine. Existing controller handles it.
**Cons:** Not physically motivated — the PID will fight the loop shape at high tilt angles. The 83° tilt clamp still applies, so the drone can't actually go inverted. The "loop" would look more like a steep climb-and-dive than a real flip. No per-situation maneuver selection.

**Verdict:** Quick win for visual variety but won't look like real acrobatic turns.

### Approach B: Maneuver Override System (Recommended)

Add a maneuver state machine that temporarily takes over from the position PID during acrobatic turns.

**How it works:**
1. **Detection:** When the AI looks ahead and sees a turn tighter than a threshold (curvature × speed exceeds what banking can handle), it selects a maneuver
2. **Maneuver selection:** Based on turn angle, altitude budget, and next gate height:
   - Turn > ~120° + altitude to spare → **Split-S**
   - Turn > ~120° + need to gain altitude → **Power Loop**
   - Turn 70°–120° at high speed → **Aggressive bank** (raise tilt clamp)
   - Turn < 70° → normal banking (current behavior)
3. **Execution:** A `ManeuverPhase` component overrides the normal PID chain:
   - **Entry:** Sets a target attitude sequence (e.g., "roll 180°, then pitch back 180°")
   - **Ballistic:** Position PID disabled, attitude controller drives the flip, drone coasts
   - **Recovery:** Position PID re-engages, drone captures the spline on the exit side
4. **Exit point:** Pre-computed — the maneuver knows where on the spline it'll re-join

**Key implementation details:**
- New component: `ActiveManeuver { kind: ManeuverKind, phase: ManeuverPhase, progress: f32, entry_velocity: Vec3, exit_spline_t: f32 }`
- During maneuver, `position_pid` is skipped (check `!has::<ActiveManeuver>`)
- `attitude_controller` receives direct attitude targets from the maneuver system instead of from the PID
- Max tilt clamp removed during maneuver
- Thrust profile per maneuver phase (e.g., Split-S: reduce to ~30% while inverted, punch to 100% on exit)

**Pros:** Physically motivated. Looks correct — drones actually flip. Maneuver selection produces varied behavior. Per-drone personality can influence threshold (aggressive drones flip earlier, cautious drones bank wider). Existing PID chain is untouched outside maneuvers.
**Cons:** Most complex to implement. Needs careful tuning of entry/exit conditions. Edge cases at maneuver boundaries. Need to handle what happens if a drone is mid-flip when it reaches a gate trigger volume.

**Estimated scope:** New system + component, modifications to the FixedUpdate chain ordering, maneuver trigger logic in the AI lookahead, thrust/attitude profiles for 2–3 maneuver types.

### Approach C: Full Trajectory Optimization (Academic)

Pre-compute time-optimal trajectories through the full course, including acrobatic sections, as polynomial curves in both position and attitude space.

**How it works:** Based on the UZH/ETH research (Foehn et al., 2021). Solve a constrained optimization: minimize lap time subject to motor thrust limits, gate passage constraints, and collision avoidance. The solution naturally produces loops and aggressive maneuvers where they're time-optimal.

**Pros:** Physically optimal — produces the actual fastest path. Would look incredible.
**Cons:** Extremely complex to implement. Requires a trajectory optimizer (CasADi, IPOPT, or similar). Solve time could be multiple seconds per drone. Way beyond the scope of a game feature. Academic research teams spend months on this.

**Verdict:** Interesting reference but not practical for our use case.

### Recommendation

**Approach B** is the sweet spot. It gives visually correct acrobatic turns, allows per-drone personality variation, and builds on our existing physics model without replacing it. Start with just the Split-S (the most common and visually distinctive maneuver), then add power loops if the system works well.

---

## 7. Sources

### Pilot Technique & Racing Lines
- [How to Find the Perfect Drone Racing Line — GetFPV Learn](https://www.getfpv.com/learn/fpv-essentials/how-to-find-the-perfect-drone-racing-line/)
- [FPV Racing Mastery: Speed Control & Tight Turn Techniques — SkyDroneHQ](https://skydronehq.com/fpv-racing-mastery-speed-control-tight-turn-techniques/)
- [Learn How to Fly FPV Drones — Oscar Liang](https://oscarliang.com/learn-flying-fpv-multirotors/)
- [Drone Racing: The Eight Common Track Elements — GetFPV](https://www.getfpv.com/learn/fpv-flight-academy/drone-racing-the-eight-common-track-elements/)

### Specific Maneuvers
- [Split-S — The Drone Racing League](https://thedroneracingleague.com/trick-wiki/split-s/)
- [A Master List of FPV Tricks and Maneuvers — WREKD](https://wrekd.com/pages/a-master-list-of-fpv-tricks-and-manuevers-and-how-to-do-them)
- [How to Master the Power Loop — Mepsking](https://www.mepsking.shop/explore/how-to-master-the-power-loop.html)
- [Advanced FPV Flying Techniques — DroneHundred](https://dronehundred.com/blogs/advanced-guides/advanced-fpv-flying-techniques-freestyle-and-racing)

### Quadrotor Physics
- [Quadcopter Dynamics and Simulation — Andrew Gibiansky](https://andrew.gibiansky.com/blog/physics/quadcopter-dynamics/)
- [Multi-rotor Aircraft — Georgia Tech Robotics](https://www.roboticsbook.org/S72_drone_actions.html)
- [Lecture 6: Quadrotor Dynamics — MIT VNAV](https://vnav.mit.edu/material/06-Control1-notes.pdf)

### Academic Research on Aggressive/Time-Optimal Flight
- [Time-optimal planning for quadrotor waypoint flight — Science Robotics (2021)](https://www.science.org/doi/10.1126/scirobotics.abh1221) — Autonomous drone beat human pilots using time-optimal trajectories that push to motor saturation limits
- [Reaching the limit in autonomous racing — Science Robotics (2023)](https://www.science.org/doi/10.1126/scirobotics.adg1462) — "Swift" system beat world-champion human pilots
- [Time-Optimal Gate-Traversing Planner (2023)](https://arxiv.org/abs/2309.06837) — Shows optimal paths approach gates at steep angles and use full gate area
- [Polynomial Trajectory Planning for Aggressive Quadrotor — MIT CSAIL](https://groups.csail.mit.edu/rrg/papers/Richter_ISRR13.pdf)
- [Deep Drone Acrobatics — UZH RPG](https://rpg.ifi.uzh.ch/aggressive_flight.html) — Learned policies for autonomous power loops and barrel rolls
- [Thrust Mixing, Saturation, and Body-Rate Control — UZH RPG](https://rpg.ifi.uzh.ch/docs/RAL17_Faessler.pdf)
