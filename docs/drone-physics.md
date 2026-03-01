# Drone Physics — Quick Reference

Simulation parameters and architecture. For real-world flight dynamics background, see [`drone-physics-deep-dive.md`](drone-physics-deep-dive.md).

## Architecture

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

## Parameters

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
