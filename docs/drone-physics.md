# Drone Physics Reference

Technical reference for the drone flight simulation. Covers real-world micro FPV racing drone behavior, how the simulation models it, and what remains unimplemented.

## Real-World Drone Flight Dynamics

### Fundamental Principle

A quadrotor can **only** produce thrust along its body-up axis. To move laterally, it must tilt. This is the single most important constraint:

```
Lateral acceleration = g * tan(tilt_angle)
Required thrust to maintain altitude while tilted = weight / cos(tilt_angle)

 30deg tilt:  5.66 m/s^2 lateral,  1.15x weight thrust
 45deg tilt:  9.81 m/s^2 lateral,  1.41x weight thrust
 60deg tilt: 16.99 m/s^2 lateral,  2.00x weight thrust
 75deg tilt: 36.61 m/s^2 lateral,  3.86x weight thrust
 80deg tilt: 55.60 m/s^2 lateral,  5.76x weight thrust
```

Thrust-to-weight ratio directly limits max tilt angle at constant altitude.

### Hover Behavior

A hovering drone is never truly still. The flight controller (typically running at 4-8 kHz) makes continuous corrections:

- **Micro-corrections**: 0.5-2 deg tilts on roll/pitch, several times per second
- **Altitude bobbing**: 1-3 cm amplitude at 1-3 Hz (throttle PID is slower than attitude PID)
- **Lateral drift-and-correct**: 5-20 cm/s drift, then correction with slight overshoot
- **Yaw creep**: 1-5 deg/s drift that the gyro corrects

The system behaves as a **damped spring with noise** -- always slightly behind the setpoint, always correcting, always slightly overshooting.

#### PID Effects on Visible Motion

| Term | Role | Visual signature |
|------|------|-----------------|
| P (Proportional) | Push toward target proportional to error | Higher = snappier but can oscillate (10-30 Hz jello) |
| I (Integral) | Eliminate steady-state offset over time | Too high = slow 0.5-2 Hz wobble, windup causes overshoot |
| D (Derivative) | Dampen rate of change | Smooths P oscillation. Too high = motor noise. Too low = bouncy |

#### Disturbance Response

When hit by a gust or prop wash:
1. Gyro detects tilt (5-20 ms)
2. PID commands correction (1-2 ms)
3. Motors spool to correct (10-30 ms)
4. Drone rotates back, **overshoots 10-30%**
5. 1-3 oscillations before settling (well-tuned) or 4-8+ (poorly tuned)

Total recovery: 100-400 ms for a well-tuned racing quad.

### Movement Dynamics

#### Acceleration Sequence

1. Pitch forward (50-100 ms to reach target angle)
2. Increase throttle to compensate lost vertical thrust
3. Accelerate: `a = g * tan(pitch) - drag`
4. Drag increases with v^2, eventually reaching terminal velocity

Typical racing micro numbers:
- 0 to 100 km/h: ~1-2 seconds
- Max speed: 130-180 km/h
- Max sustained acceleration: 2-6g (brief peaks higher)

#### Banking and Turning

A coordinated turn:
1. Roll into the turn (banking)
2. Pitch to pull through (like pulling back on a stick)
3. Minimal yaw in coordinated turns, more in flat turns

Turn radius: `r = v^2 / (g * tan(bank_angle))`

```
At 100 km/h, 45deg bank:  78.7 m radius
At 100 km/h, 60deg bank:  45.4 m radius
At 100 km/h, 80deg bank:  14.2 m radius
At  50 km/h, 60deg bank:  11.4 m radius
```

Key visual detail: in a fast banked turn, the nose points inward and slightly down from the velocity vector. There is always some sideslip, especially during turn entry/exit.

#### Braking

1. Pitch up sharply (30-70 deg depending on aggression)
2. Lose altitude (vertical component drops), pilot adds throttle
3. Speed decreases from drag + redirected thrust
4. Pitch back toward level as speed drops
5. **Overshoot**: 1-3 oscillation settling over 200-800 ms

The braking pitch-up is one of the most visually distinctive multirotor behaviors.

### Characteristic Pitch Angles

| Maneuver | Pitch |
|----------|-------|
| Aggressive acceleration | 40-60 deg nose-down |
| Cruise | 10-30 deg nose-down (speed-dependent) |
| Braking | 20-60 deg nose-up |
| Hover | Near level, +/-2 deg oscillation |

Approximation: `pitch = -atan(lateral_accel / (g + vertical_accel))`

### Prop Wash and Turbulence

Prop wash is the downward column of turbulent air from the propellers.

- **Hover near ground** (within ~2x frame diameter): Ground effect -- more efficient but more turbulent. Increased oscillation and bobbing.
- **Descent through own wash**: Violent oscillations, 5-15 deg irregular tilts at 3-10 Hz. Pilots avoid vertical descent for this reason.
- **Following another drone**: Trailing drone hits lead's prop wash, causing sudden wobble/drop.

### Altitude Change Asymmetry

**Climbing**: Clean air (moving into undisturbed air). Stable, predictable. Max climb 15-30 m/s. Visually smooth.

**Descending**: Turbulent (own prop wash). Oscillatory. Pilots descend at angles. Practical max vertical descent ~5-10 m/s before control degrades. Visually rough.

### Motor Dynamics

- **Spool-up** (0 to full): 30-80 ms (motor inductance + ESC rate + rotor inertia)
- **Spool-down**: 60-150 ms (slower -- relies on drag/active braking)
- **Mid-range change** (50% to 80%): 15-40 ms

Asymmetry: spool-up faster than spool-down. Creates subtle oscillation asymmetry -- drone spends slightly more time above target altitude than below.

For simulation, a first-order lag is sufficient:
```
actual_thrust += (commanded - actual) * (1 - e^(-dt / tau))
tau = 20-50 ms for racing motors
```

### Yaw Behavior

Yaw rate in a coordinated banked turn: `yaw_rate = g * sin(bank) / velocity`

Yaw is the slowest axis on most quads (400-800 deg/s max vs 1200-1800 on roll/pitch) because it relies on differential motor torque rather than thrust vectoring.

### Stopping Overshoot

```
overshoot_distance  ~= 0.1-0.3 x braking_distance
settling_time       ~= 200-600 ms
oscillation_freq    ~= 2-5 Hz
damping_ratio       ~= 0.3-0.7 (underdamped by design)
```

## Key Parameters

### Physical Properties

| Parameter | 3" Micro | 5" Racer | Our Sim (0.8 kg) |
|-----------|----------|----------|-------------------|
| Mass | 0.25 kg | 0.5-0.65 kg | 0.8 kg |
| TWR | 5:1 - 8:1 | 7:1 - 12:1 | ~7:1 (55 N max) |
| Max speed | 130-160 km/h | 150-180 km/h | ~166 km/h (46 m/s) |
| Max sustained accel | 2-4g | 3-6g | ~6g |

### Angular Rates

| Axis | Typical Racing | Max Achievable | Our Sim |
|------|---------------|----------------|---------|
| Roll | 800-1200 deg/s | 1800-2200 deg/s | ~1150 deg/s (20 rad/s) |
| Pitch | 800-1200 deg/s | 1800-2000 deg/s | ~1150 deg/s (20 rad/s) |
| Yaw | 400-800 deg/s | 800-1200 deg/s | ~573 deg/s (10 rad/s) |

Time to rotate 90 deg on roll at 1200 deg/s: ~75 ms.

### Drag Model

Quadratic drag: `F_drag = -k * |v| * v`

For practical use, combine `0.5 * rho * Cd * A` into a single constant `k`.

| Drone Size | Frontal Area | Combined k |
|-----------|-------------|-----------|
| 3" micro | 0.015-0.03 m^2 | 0.005-0.02 |
| 5" racer | 0.03-0.06 m^2 | 0.01-0.04 |
| Our sim | -- | 0.025 |

Verification: at 75 deg tilt, horizontal thrust = 55 * sin(75) = 53.1 N.
Terminal velocity = sqrt(53.1 / 0.025) = 46.1 m/s = 166 km/h. Matches real racing speeds.

#### Anisotropic Drag (not yet implemented)

Real drones have different drag per direction:
```
k_forward  = 0.02    (arms/frame relatively streamlined)
k_sideways = 0.03    (1.5x -- full frame width exposed)
k_up       = 0.015   (0.75x -- small top profile)
k_down     = 0.04    (2x -- prop disks act as parachute)
```

### Moment of Inertia

For a 0.8 kg drone with 0.5 m frame:
- Roll/pitch: ~0.003 kg*m^2
- Yaw: ~0.005 kg*m^2 (higher because mass is spread on horizontal plane)

### PID Tuning Targets

For attitude control with `torque = kp * error - kd * angular_velocity`:
- Natural frequency: `omega_n = sqrt(kp / I)`
- Damping ratio: `zeta = kd / (2 * sqrt(kp * I))`
- Settling time (2%): `~4 / (zeta * omega_n)`

A damping ratio of 0.4-0.7 (underdamped) produces the visible overshoot that looks realistic.

## Current Implementation

### Architecture

Thrust-through-body model with cascaded control:

```
DesiredPosition (from AI or hover_target)
       |
  position_pid        Outer loop: position error -> desired acceleration
       |                 -> desired body orientation + thrust magnitude
  DesiredAttitude
       |
  attitude_controller  Inner loop: orientation error -> torque
       |                 -> angular velocity -> rotation (on Transform)
  motor_lag            First-order filter on thrust (40 ms tau)
       |
  apply_forces         Thrust along body-up + gravity + quadratic drag -> velocity
       |
  integrate_motion     Velocity -> position
       |
  clamp_transform      Floor collision
```

### Parameters Used

```
mass:                0.8 kg
max_thrust:          55.0 N (TWR ~7:1)
drag_constant:       0.025 (quadratic, F = k*v^2)
max_speed:           45.0 m/s (safety clamp, rarely reached)
max_tilt_angle:      1.3 rad (75 deg)
moment_of_inertia:   (0.003, 0.005, 0.003) kg*m^2
motor_time_constant: 0.040 s

Position PID:        kp (6, 8, 6)  ki (0.1, 0.2, 0.1)  kd (4, 5, 4)
Attitude PD:         kp_roll_pitch 25.0  kd_roll_pitch 8.0
                     kp_yaw 15.0  kd_yaw 5.0
Max angular rate:    (20, 20, 10) rad/s

Hover noise amp:     0.01-0.03 m per axis
Hover noise freq:    0.3-2.0 Hz per axis
Per-drone PID var:   +/- 15%
```

## Not Yet Implemented

Potential additions ranked by visual impact:

### High Impact

**Prop wash / turbulence during descent**
Extra noise/oscillation when descending vertically or trailing another drone. Multiply hover oscillation by 2-4x during prop wash encounters. Would need spatial proximity checks (O(n^2) but only 12 drones).

**Anisotropic drag**
Different drag coefficients per body-frame axis. Transform velocity to body frame, apply per-axis drag, transform back. Makes sideways drift feel different from forward flight.

### Medium Impact

**Asymmetric motor spool**
Spool-up faster (30 ms) than spool-down (80 ms). Two time constants with a branch in motor_lag. Creates subtle asymmetry -- drone spends more time above target than below.

**Per-drone CG offset**
Small constant attitude bias (0.5-2 deg per axis, randomized). Simulates imperfect center of gravity. Makes each drone's hover look unique beyond PID variation.

**Ground effect**
Within ~0.5 m of ground: thrust efficiency increases ~10-15% but turbulence increases. Modify thrust and add extra noise based on floor proximity.

### Low Impact

**Body-frame attitude error**
Compute orientation error in body frame instead of world frame. Matters for extreme orientations (near-inverted) but negligible below 75 deg tilt.

**Angular drag**
Resistance to rotation from air resistance on props/arms. Currently implicit in the PD damping term but could be modeled explicitly for more accurate coast-down of angular velocity.

## Common Simulation Mistakes

Documented here to avoid regression:

1. **Over-damped hover**: Drone holds position perfectly. Real drones always oscillate +/- 0.5-2 deg. Use damping ratio 0.4-0.7, not >= 1.0.

2. **Missing rotational inertia**: Orientation changes instantly. Must model angular velocity explicitly with torque/inertia.

3. **No momentum**: Drone stops when thrust redirects. Must track velocity separately from orientation -- drones carry momentum.

4. **Identical drones**: All AI drones behave the same. Vary PID gains (+/- 10-20%), motor response (+/- 15%), max rates (+/- 10%).

5. **No altitude coupling**: Tilting doesn't affect altitude. Vertical force = `thrust * cos(tilt) - weight`. The PID must actively compensate.

6. **Velocity-driven rotation**: Rotation follows velocity (drone translates then tilts). Real drones tilt first, then the resulting thrust creates lateral velocity. Rotation should lead, not follow.

7. **Linear drag**: `F = -k*v` feels syrupy at low speed and insufficient at high speed. Use quadratic: `F = -k*|v|*v`.
