# Drone Physics — Quick Reference

Simulation parameters and architecture. For real-world flight dynamics background, see [`drone-physics-deep-dive.md`](drone-physics-deep-dive.md).

## Architecture

Thrust-through-body model with 3-stage cascaded control. The former monolithic `position_pid` was decomposed into `position_to_acceleration` → `acceleration_to_attitude` for composability (allows maneuver systems to inject at either stage).

```
DesiredPosition (from AI or hover_target)
       |
  sync_tilt_clamp           Sync TiltClamp component from AiTuningParams
       |
  position_to_acceleration  Stage 1: position error → DesiredAcceleration
       |                      PID with gravity compensation + anti-windup
  DesiredAcceleration
       |
  acceleration_to_attitude  Stage 2: acceleration → DesiredAttitude
       |                      Maps world-frame accel to body orientation + thrust
       |                      Applies per-entity TiltClamp
  DesiredAttitude
       |
  attitude_controller       Stage 3: orientation error → torque → angular velocity
       |                      Also supports DesiredBodyRates (rate-tracking mode)
  motor_lag                 First-order filter on thrust (25 ms tau)
       |
  apply_forces              Thrust along body-up + gravity + quadratic drag → velocity
       |
  integrate_motion          Velocity → position
       |
  clamp_transform           Floor collision
```

**Note:** During races, the physics chain only processes non-Racing drones (Idle, Wandering). Racing drones are handled by the choreography chain. See [`drone-system.md`](drone-system.md).

## Parameters

```
mass:                0.8 kg
max_thrust:          55.0 N (TWR ~7:1)
drag_constant:       0.025 (quadratic, F = k*v^2)
max_speed:           45.0 m/s (safety clamp, rarely reached)
max_tilt_angle:      1.3 rad (75 deg)
moment_of_inertia:   (0.003, 0.005, 0.003) kg*m^2
motor_time_constant: 0.025 s

Position PID:        kp (6, 8, 6)  ki (0.1, 0.2, 0.1)  kd (4, 5, 4)
Attitude PD:         kp_roll_pitch 7.0  kd_roll_pitch 0.20
                     kp_yaw 3.0  kd_yaw 0.25
Max angular rate:    (8, 8, 6) rad/s

Hover noise amp:     0.01-0.03 m per axis
Hover noise freq:    0.3-2.0 Hz per axis
Per-drone PID var:   +/- 15%
```

## Key Physical Limits

| Parameter | Value |
|-----------|-------|
| Max speed | ~166 km/h (46 m/s terminal) |
| Max sustained accel | ~6g |
| Roll/pitch rate | ~1150 deg/s (20 rad/s) |
| Yaw rate | ~573 deg/s (10 rad/s) |
| 90-deg roll time | ~75 ms |

Drag verification: at 75 deg tilt, horizontal thrust = 55 * sin(75) = 53.1 N. Terminal velocity = sqrt(53.1 / 0.025) = 46.1 m/s = 166 km/h.

## PID Tuning Targets

For attitude control with `torque = kp * error - kd * angular_velocity`:
- Natural frequency: `omega_n = sqrt(kp / I)`
- Damping ratio: `zeta = kd / (2 * sqrt(kp * I))`
- Target damping: 0.4-0.7 (underdamped, produces realistic overshoot)

## Common Simulation Mistakes

1. **Over-damped hover**: Real drones always oscillate +/- 0.5-2 deg. Use damping ratio 0.4-0.7, not >= 1.0.
2. **Missing rotational inertia**: Must model angular velocity explicitly with torque/inertia.
3. **No momentum**: Drones carry momentum — track velocity separately from orientation.
4. **Identical drones**: Vary PID gains (+/- 10-20%), motor response (+/- 15%), max rates (+/- 10%).
5. **No altitude coupling**: Vertical force = `thrust * cos(tilt) - weight`. PID must compensate.
6. **Velocity-driven rotation**: Real drones tilt first, then thrust creates lateral velocity. Rotation leads, not follows.
7. **Linear drag**: Use quadratic `F = -k*|v|*v`, not linear `F = -k*v`.
