# Drone Physics — Deep Dive

Real-world micro FPV racing drone behavior and future simulation improvements. For current sim parameters and architecture, see [`drone-physics.md`](drone-physics.md).

## Real-World Drone Flight Dynamics

### Fundamental Principle

A quadrotor can **only** produce thrust along its body-up axis. To move laterally, it must tilt.

```
Lateral acceleration = g * tan(tilt_angle)
Required thrust to maintain altitude while tilted = weight / cos(tilt_angle)

 30deg tilt:  5.66 m/s^2 lateral,  1.15x weight thrust
 45deg tilt:  9.81 m/s^2 lateral,  1.41x weight thrust
 60deg tilt: 16.99 m/s^2 lateral,  2.00x weight thrust
 75deg tilt: 36.61 m/s^2 lateral,  3.86x weight thrust
 80deg tilt: 55.60 m/s^2 lateral,  5.76x weight thrust
```

### Hover Behavior

A hovering drone is never truly still. The flight controller makes continuous corrections:

- **Micro-corrections**: 0.5-2 deg tilts on roll/pitch, several times per second
- **Altitude bobbing**: 1-3 cm amplitude at 1-3 Hz
- **Lateral drift-and-correct**: 5-20 cm/s drift, then correction with slight overshoot
- **Yaw creep**: 1-5 deg/s drift that the gyro corrects

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

Key visual detail: in a fast banked turn, the nose points inward and slightly down from the velocity vector.

#### Braking

1. Pitch up sharply (30-70 deg depending on aggression)
2. Lose altitude, pilot adds throttle
3. Speed decreases from drag + redirected thrust
4. Pitch back toward level as speed drops
5. **Overshoot**: 1-3 oscillation settling over 200-800 ms

### Characteristic Pitch Angles

| Maneuver | Pitch |
|----------|-------|
| Aggressive acceleration | 40-60 deg nose-down |
| Cruise | 10-30 deg nose-down (speed-dependent) |
| Braking | 20-60 deg nose-up |
| Hover | Near level, +/-2 deg oscillation |

Approximation: `pitch = -atan(lateral_accel / (g + vertical_accel))`

### Prop Wash and Turbulence

- **Hover near ground** (within ~2x frame diameter): Ground effect — more efficient but more turbulent.
- **Descent through own wash**: Violent oscillations, 5-15 deg irregular tilts at 3-10 Hz.
- **Following another drone**: Trailing drone hits lead's prop wash, causing sudden wobble/drop.

### Altitude Change Asymmetry

- **Climbing**: Clean air, stable, predictable. Max climb 15-30 m/s. Visually smooth.
- **Descending**: Turbulent (own prop wash). Oscillatory. Practical max ~5-10 m/s before control degrades.

### Motor Dynamics

- **Spool-up** (0 to full): 30-80 ms
- **Spool-down**: 60-150 ms (slower)
- **Mid-range change** (50% to 80%): 15-40 ms

Asymmetry creates subtle oscillation asymmetry — drone spends slightly more time above target altitude than below.

First-order lag model: `actual_thrust += (commanded - actual) * (1 - e^(-dt / tau))`, tau = 20-50 ms.

### Yaw Behavior

Yaw rate in a coordinated banked turn: `yaw_rate = g * sin(bank) / velocity`

Yaw is the slowest axis (400-800 deg/s max vs 1200-1800 on roll/pitch) — relies on differential motor torque.

### Stopping Overshoot

```
overshoot_distance  ~= 0.1-0.3 x braking_distance
settling_time       ~= 200-600 ms
oscillation_freq    ~= 2-5 Hz
damping_ratio       ~= 0.3-0.7 (underdamped by design)
```

## Physical Properties Comparison

| Parameter | 3" Micro | 5" Racer | Our Sim (0.8 kg) |
|-----------|----------|----------|-------------------|
| Mass | 0.25 kg | 0.5-0.65 kg | 0.8 kg |
| TWR | 5:1 - 8:1 | 7:1 - 12:1 | ~7:1 (55 N max) |
| Max speed | 130-160 km/h | 150-180 km/h | ~166 km/h (46 m/s) |
| Max sustained accel | 2-4g | 3-6g | ~6g |

## Drag Model Details

Quadratic drag: `F_drag = -k * |v| * v`

| Drone Size | Frontal Area | Combined k |
|-----------|-------------|-----------|
| 3" micro | 0.015-0.03 m^2 | 0.005-0.02 |
| 5" racer | 0.03-0.06 m^2 | 0.01-0.04 |
| Our sim | -- | 0.025 |

### Anisotropic Drag (not yet implemented)

```
k_forward  = 0.02    (arms/frame relatively streamlined)
k_sideways = 0.03    (1.5x -- full frame width exposed)
k_up       = 0.015   (0.75x -- small top profile)
k_down     = 0.04    (2x -- prop disks act as parachute)
```

## Moment of Inertia

For a 0.8 kg drone with 0.5 m frame:
- Roll/pitch: ~0.003 kg*m^2
- Yaw: ~0.005 kg*m^2

## Not Yet Implemented

Potential additions ranked by visual impact:

### High Impact

**Prop wash / turbulence during descent** — Extra noise/oscillation when descending vertically or trailing another drone. Multiply hover oscillation by 2-4x. Would need spatial proximity checks (O(n^2) but only 12 drones).

**Anisotropic drag** — Different drag coefficients per body-frame axis. Transform velocity to body frame, apply per-axis drag, transform back.

### Medium Impact

**Asymmetric motor spool** — Spool-up faster (30 ms) than spool-down (80 ms). Two time constants with a branch in motor_lag.

**Per-drone CG offset** — Small constant attitude bias (0.5-2 deg per axis, randomized). Makes each drone's hover look unique.

**Ground effect** — Within ~0.5 m of ground: thrust +10-15% but turbulence increases.

### Low Impact

**Body-frame attitude error** — Compute orientation error in body frame. Matters for extreme orientations but negligible below 75 deg tilt.

**Angular drag** — Explicit air resistance on rotation. Currently implicit in PD damping term.
