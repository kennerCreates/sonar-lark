Research Report: Preparing Our System for Acrobatic Maneuvers
What the Industry Does (and Doesn't Do)
The research reveals a striking pattern: no commercial FPV racing simulator has solved algorithmic AI acrobatics. The most realistic sims sidestep the problem entirely:

Velocidrone (gold standard in realism) uses replay ghosts of recorded human flights — no algorithmic AI at all
Liftoff uses fixed AI ghosts that fly the exact same path every time — essentially replay data
DRL/DCL have basic AI opponents with no public evidence of acrobatic capability
Uncrashed has no AI opponents at all
This tells us something important: getting AI drones to fly acrobatically in a physically-accurate simulation is a genuinely hard problem. Even teams with significant resources chose to avoid it. The fact that our previous attempt failed is entirely consistent with industry experience.

The academic world HAS solved it, but with approaches that are overkill for a game:

UZH Swift (beat world champions) uses reinforcement learning trained over ~1 month of simulated flight. No splines — an end-to-end neural network maps sensor input to motor commands. The system learned Split-S maneuvers as emergent behavior from time-optimal racing.
Deep Drone Acrobatics (ETH Zurich) defines each maneuver as a reference trajectory of body rates and thrust (not positions), then trains a neural network to track those references. This architectural insight is directly relevant to us.
Why Our Previous Attempt Failed — Root Cause Analysis
Re-reading the postmortem alongside the Betaflight and academic research, the failures have a clear root cause: the maneuver system operated in a fundamentally different coordinate space than normal flight, and the handoff between them was a discontinuity.

The architecture was:


Normal flight:  DesiredPosition → position_pid → DesiredAttitude → attitude_controller
Maneuver:       ActiveManeuver  → execute_maneuvers → DesiredAttitude → attitude_controller
Two completely separate systems writing to DesiredAttitude, with a hard switch between them. This caused three specific failure modes:

The maneuver trajectory was in position space, but should have been in rate space. The Hermite spline trajectories computed desired positions/attitudes along a curve, then wrote orientation quaternions directly. But real acrobatic maneuvers are executed as angular rate commands — a pilot doing a Split-S commands "roll at max rate for 160ms, then pitch back at moderate rate" — they don't think in quaternions. By trying to track positions during a flip, the system fought against the drone's angular momentum rather than using it.

The position PID integral accumulated stale error. When the maneuver system took over, position_pid was skipped (via Without<ActiveManeuver>), but its integral term kept its old value. When control returned to the PID, the stale integral caused an immediate spike. This is the classic integral windup problem, well-documented in control theory.

The tilt clamp created a hard wall. max_tilt_angle at 83° is baked into position_pid. The maneuver system bypassed this by skipping position_pid entirely, but re-engaging the PID meant hitting the 83° wall while the drone was still at extreme orientation — causing violent corrections.

What Needs to Change Before Implementing Maneuvers
Based on all this research, I recommend four architectural changes to the control pipeline. These are prerequisites that will make maneuver implementation possible and also improve normal flight quality. None of them add maneuver logic themselves.

1. Decompose position_pid into Composable Stages
The problem: position_pid (physics.rs:51-126) is a monolithic function that does five things in one pass:

Computes position error and PID output
Adds gravity compensation
Computes desired body-up direction from the acceleration vector
Clamps tilt angle
Computes thrust magnitude and yaw orientation
This makes it impossible to replace just one stage. The previous attempt had to skip ALL of it.

The change: Split into three separate systems that chain together:


Stage 1: position_to_acceleration
  DesiredPosition + Transform + DroneDynamics → DesiredAcceleration (new component)
  Pure PID: error → acceleration vector (with gravity compensation)

Stage 2: acceleration_to_attitude  
  DesiredAcceleration → DesiredAttitude
  Maps world-frame acceleration to body orientation + thrust
  Applies per-entity tilt clamp
  Computes yaw from velocity_hint

Stage 3: attitude_controller (existing, unchanged)
  DesiredAttitude → torque → angular velocity → rotation
Why this matters for maneuvers: A maneuver can replace only Stage 1 — writing a DesiredAcceleration directly — while Stages 2 and 3 continue running normally. The tilt clamp, thrust computation, and attitude tracking all stay active. No hard bypass, no discontinuity. Alternatively, a more aggressive maneuver can replace Stages 1+2, writing directly to DesiredAttitude, while still having the attitude controller provide stable tracking.

Performance impact: Three systems instead of one, but they're trivially small. The query patterns are nearly identical. No measurable impact at 12 entities.

2. Add a Body-Rate Control Mode to attitude_controller
The problem: The current attitude controller (physics.rs:130-181) only accepts orientation targets (quaternions). It computes: error_quat = desired_orientation * current.inverse() → torque → angular velocity.

Real acrobatic flight doesn't work this way. A pilot doing a power loop commands a pitch rate — "pitch back at 12 rad/s" — not a target orientation. Our current system can't express this.

The change: Add a DesiredBodyRates component as an alternative input to attitude_controller:


#[derive(Component)]
pub struct DesiredBodyRates {
    pub roll_rate: f32,   // rad/s, body frame
    pub pitch_rate: f32,
    pub yaw_rate: f32,
    pub thrust: f32,      // Newtons
}
When DesiredBodyRates is present, attitude_controller switches to rate-tracking mode:

Error = desired_rate - current angular_velocity (per axis)
PD control on rate error (simpler than orientation PD)
No quaternion error computation needed
This directly mirrors Betaflight's architecture:

Normal flight = "angle mode": outer loop generates orientation targets, attitude controller tracks them
Acrobatic flight = "acro mode": maneuver system generates rate targets, attitude controller tracks them
The attitude controller is always running in both cases. Only its input source changes. This is exactly how Betaflight handles the angle-to-acro transition, and it's the reason real drones can switch modes mid-flight without crashing.

The key insight from Betaflight: The inner rate loop doesn't know or care whether its setpoint comes from an outer position loop or from direct rate commands. It just tracks rates. This makes transitions between control modes smooth by construction.

3. Make Tilt Clamping Per-Entity and State-Aware
The problem: Tilt clamping is a global parameter (AiTuningParams.max_tilt_angle = 1.45 rad), read once and applied identically to all drones. During a flip, the drone MUST exceed 90°. The previous attempt handled this by skipping position_pid entirely, which caused cascading problems.

The change: Add a per-entity tilt override component:


#[derive(Component)]
pub struct TiltClamp {
    pub max_angle: f32,  // radians, default from AiTuningParams
}
acceleration_to_attitude (Stage 2 from change #1) reads TiltClamp if present, otherwise falls back to the global tuning param
During a maneuver entry, the system smoothly raises TiltClamp.max_angle from 83° to 180° (or removes the component entirely)
During maneuver exit, smoothly restore it
This also enables per-drone personality: aggressive drones could have a slightly higher tilt clamp even in normal flight (95° instead of 83°), producing more dramatic banking that looks faster without needing the full maneuver system.

4. Add PID Integral Management
The problem: The position PID's integral term (components.rs:22-27) accumulates continuously. When a maneuver takes over and the drone intentionally diverges from the spline, the integral accumulates large error. When normal control resumes, this stale integral causes an immediate, violent correction.

The changes:

a) Anti-windup: When the position PID's output is saturated (the tilt clamp is active and limiting the desired acceleration direction), stop accumulating integral. This prevents integral buildup during normal aggressive turns too.


// In position_to_acceleration:
let is_saturated = tilt_angle > max_tilt * 0.95;  // near the clamp
if !is_saturated {
    pid.integral = (pid.integral + error * dt).clamp(...);
}
b) Integral decay on mode transition: When entering/exiting a maneuver, exponentially decay the integral over ~200ms rather than resetting it instantly. This avoids both the stale-integral spike AND the zero-integral undershoot.


// Exponential decay: half-life ~100ms at 64Hz
pid.integral *= 0.96;  // per tick, or time-based
These are standard anti-windup techniques from control engineering. Betaflight implements similar protections (I-term relax, I-term limit).

What About the Spline System?
You mentioned wanting to extend the spline creation system. Here's my thinking on that:

The previous attempt detected maneuvers at runtime — scanning the spline ahead for sharp turns and triggering maneuvers on-the-fly. This caused timing problems: trigger too early and the drone starts flipping before it needs to; trigger too late and there's no time to execute.

A better approach: embed maneuver intent into the spline at generation time. The spline generator (paths/generation.rs) already knows where the tight turns are (it places approach/departure/midleg control points based on gate geometry). Instead of generating flat-arc control points and hoping the runtime detects the need for a maneuver, shape the spline itself to include the 3D trajectory of the maneuver.

For example, for a Split-S between gates:

Currently: approach → flat midleg → approach (produces a flat curve, drone banks hard)
With maneuver intent: approach → dive-and-swoop arc → approach (produces a 3D path that the PID follows naturally)
The key difference from the postmortem's "Approach A: Spline Warping" (which was dismissed): with changes #1-4 above, the control system CAN handle the warped spline. The tilt clamp is raised for maneuver segments, the PID handles extreme orientations gracefully, and the attitude controller can switch to rate mode for the most aggressive portions.

This is a hybrid of Approaches A and B from the research doc — the spline encodes the maneuver trajectory, but the control system adapts its behavior based on spline segment metadata. No dual-path architecture, no handoff discontinuity.

Specific spline extensions to add later (after the control changes):

Maneuver segment metadata on control points (enum: Normal, SplitS, PowerLoop, BarrelRoll)
3D control point generation for maneuver segments (vertical displacement curves)
Per-segment tilt clamp and control mode hints
Per-drone maneuver aggressiveness (from pilot personality: some drones do the flip, some take the wide arc)
Recommended Order of Implementation
Phase	What	Why first
Phase 1	Decompose position_pid into 3 stages	Foundation for everything else. Purely structural refactor, zero behavior change
Phase 2	Per-entity tilt clamp component	Small change, immediately useful (per-drone personality)
Phase 3	PID anti-windup + integral decay	Improves normal flight stability too (less oscillation after hard turns)
Phase 4	Body-rate control mode in attitude_controller	Enables acro-mode maneuvers
Phase 5	3D spline generation with maneuver segments	The actual maneuver content
Phase 6	Maneuver-aware AI (trigger and selection logic)	Ties it all together
Phases 1-4 are pure control-architecture improvements that make the system more robust regardless of maneuvers. They can each be implemented and tested independently, with no visible behavior change until Phase 5.

Summary
The previous attempt failed because it tried to bolt a parallel control system onto a monolithic pipeline. The research (Betaflight, academic racers, game AI patterns) all point to the same solution: make the existing control pipeline more modular and composable, then maneuvers become a matter of swapping inputs to the inner loops rather than bypassing them.

The four changes I'm recommending (decompose PID, add rate control mode, per-entity tilt clamp, integral management) are all standard control engineering practices that the real drone firmware world has already solved. They set up the architecture so that maneuver implementation becomes a content problem (defining cool spline shapes and rate profiles) rather than a control problem (fighting the physics).

