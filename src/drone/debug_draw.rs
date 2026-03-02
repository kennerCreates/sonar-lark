use bevy::prelude::*;

use super::components::*;
use super::maneuver::{ManeuverKind, ManeuverTrajectory, TiltOverride};
use super::paths::adaptive_approach_offset;
use crate::common::POINTS_PER_GATE;
use crate::race::gate::GatePlanes;

/// Toggle resource for flight debug visualization. Press F3 during race to toggle.
#[derive(Resource)]
pub struct FlightDebugDraw {
    pub enabled: bool,
}

pub fn toggle_debug_draw(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug: Option<ResMut<FlightDebugDraw>>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        if let Some(ref mut d) = debug {
            d.enabled = !d.enabled;
            info!("Flight debug draw: {}", if d.enabled { "ON" } else { "OFF" });
        } else {
            commands.insert_resource(FlightDebugDraw { enabled: true });
            info!("Flight debug draw: ON");
        }
    }
}

/// Draw each drone's unique spline path as a polyline, color-coded per drone.
pub fn draw_spline_path(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    query: Query<(&Drone, &AIController)>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    for (drone, ai) in &query {
        let total_t = ai.gate_count as f32 * POINTS_PER_GATE;
        // ~10 samples per spline unit (reduced from 20 since we draw 12 splines)
        let samples = (total_t * 10.0) as usize;
        if samples < 2 {
            continue;
        }

        let hue = (drone.index as f32 / 12.0) * 360.0;
        let color = Color::hsl(hue, 0.8, 0.6);

        for i in 0..samples {
            let t0 = (i as f32 / samples as f32) * total_t;
            let t1 = ((i + 1) as f32 / samples as f32) * total_t;
            let p0 = ai.spline.position(t0);
            let p1 = ai.spline.position(t1);
            gizmos.line(p0, p1, color);
        }
    }
}

/// Draw control point markers: approach (red), departure (blue), gate center (green).
pub fn draw_gate_markers(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    query: Query<&AIController, With<Drone>>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    let Some(ai) = query.iter().next() else {
        return;
    };

    let n = ai.gate_positions.len();

    for (i, (pos, fwd)) in ai
        .gate_positions
        .iter()
        .zip(ai.gate_forwards.iter())
        .enumerate()
    {
        let next = (i + 1) % n;
        let gate_dist = (ai.gate_positions[next] - *pos).length();
        let offset = adaptive_approach_offset(gate_dist);
        let approach = *pos - *fwd * offset;
        let departure = *pos + *fwd * offset;

        // Gate center (physical position, not a control point): green sphere
        gizmos.sphere(Isometry3d::from_translation(*pos), 0.6, Color::srgb(0.1, 1.0, 0.1));

        // Approach control point: red sphere
        gizmos.sphere(
            Isometry3d::from_translation(approach),
            0.4,
            Color::srgb(1.0, 0.2, 0.2),
        );

        // Departure control point: blue sphere
        gizmos.sphere(
            Isometry3d::from_translation(departure),
            0.4,
            Color::srgb(0.2, 0.4, 1.0),
        );

        // Gate forward arrow (magenta)
        gizmos.arrow(*pos, *pos + *fwd * 3.0, Color::srgb(1.0, 0.0, 1.0));

        // Lines from gate center to each control point
        gizmos.line(*pos, approach, Color::srgb(0.5, 0.5, 0.5));
        gizmos.line(*pos, departure, Color::srgb(0.5, 0.5, 0.5));

        // Gate index: stacked orange dots above gate center
        let label_pos = *pos + Vec3::Y * 1.5;
        for dot in 0..=i.min(9) {
            gizmos.sphere(
                Isometry3d::from_translation(label_pos + Vec3::Y * (dot as f32 * 0.3)),
                0.1,
                Color::srgb(1.0, 0.6, 0.0),
            );
        }
    }
}

/// For each drone: draw line to desired position, velocity vector, and spline position.
pub fn draw_drone_state(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    query: Query<(
        &Transform,
        &Drone,
        &AIController,
        &DronePhase,
        &DesiredPosition,
        &DroneDynamics,
    )>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    for (transform, drone, ai, phase, desired, dynamics) in &query {
        let pos = transform.translation;
        let total_t = ai.gate_count as f32 * POINTS_PER_GATE;

        // Drone-specific color (cycle through hues)
        let hue = (drone.index as f32 / 12.0) * 360.0;
        let drone_color = Color::hsl(hue, 0.8, 0.6);

        // Line from drone to its desired target position (white, thin)
        gizmos.line(pos, desired.position, Color::srgba(1.0, 1.0, 1.0, 0.4));

        // Target position marker (small sphere in drone color)
        gizmos.sphere(
            Isometry3d::from_translation(desired.position),
            0.15,
            drone_color,
        );

        // Velocity vector (red arrow, scaled down)
        let vel_end = pos + dynamics.velocity * 0.3;
        gizmos.arrow(pos, vel_end, Color::srgb(1.0, 0.3, 0.3));

        match phase {
            DronePhase::Racing | DronePhase::VictoryLap => {
                // Where the drone is on the race spline (orange sphere)
                if ai.spline_t < total_t {
                    let curve_pos = ai.spline.position(ai.spline_t);
                    gizmos.sphere(
                        Isometry3d::from_translation(curve_pos),
                        0.2,
                        Color::srgb(1.0, 0.5, 0.0),
                    );
                    gizmos.line(pos, curve_pos, Color::srgba(1.0, 0.5, 0.0, 0.5));

                    let tangent =
                        ai.spline.velocity(ai.spline_t).normalize_or(Vec3::ZERO) * 2.0;
                    gizmos.arrow(curve_pos, curve_pos + tangent, Color::srgb(1.0, 1.0, 0.0));
                }
            }
            DronePhase::Idle | DronePhase::Crashed | DronePhase::Wandering => {}
        }
    }
}

/// Draw gate detection planes as cyan wireframe rectangles with a normal arrow.
pub fn draw_gate_planes(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    gate_planes: Option<Res<GatePlanes>>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }
    let Some(gate_planes) = gate_planes else {
        return;
    };

    let plane_color = Color::srgba(0.0, 1.0, 1.0, 0.5);
    let normal_color = Color::srgb(0.0, 1.0, 1.0);

    for plane in &gate_planes.0 {
        // Four corners of the gate opening
        let r = plane.right * plane.half_width;
        let u = plane.up * plane.half_height;
        let corners = [
            plane.center - r - u, // bottom-left
            plane.center + r - u, // bottom-right
            plane.center + r + u, // top-right
            plane.center - r + u, // top-left
        ];

        // Wireframe rectangle
        for i in 0..4 {
            gizmos.line(corners[i], corners[(i + 1) % 4], plane_color);
        }

        // Cross lines for visibility
        gizmos.line(corners[0], corners[2], plane_color);
        gizmos.line(corners[1], corners[3], plane_color);

        // Normal arrow (points toward approach side)
        gizmos.arrow(plane.center, plane.center + plane.normal * 2.0, normal_color);
    }
}

/// Draw a HUD-like ground projection showing where each drone's spline_t is
/// relative to total_t (progress rings on the ground near each gate).
pub fn draw_progress_indicators(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    query: Query<(&Drone, &AIController)>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    // Only draw for drone 0 to keep it clean — shows the "lead drone" progress
    let Some((_, ai)) = query.iter().find(|(d, _)| d.index == 0) else {
        return;
    };

    let total_t = ai.gate_count as f32 * POINTS_PER_GATE;
    if total_t <= 0.0 {
        return;
    }

    // Draw progress as colored segments along each gate center
    for i in 0..ai.gate_count as usize {
        let center_t = i as f32 * POINTS_PER_GATE + 0.5;
        let passed = ai.spline_t >= center_t;
        let color = if passed {
            Color::srgb(0.1, 1.0, 0.1) // green = passed
        } else {
            Color::srgb(1.0, 0.1, 0.1) // red = not yet
        };

        let gate_pos = ai.gate_positions[i];
        // Ring at gate height
        gizmos.circle(
            Isometry3d::new(gate_pos, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            1.5,
            color,
        );
    }
}

/// Draw maneuver state for drones with a trajectory or tilt override.
/// - Trajectory curve as a polyline (20 samples)
/// - Colored ring: red = Split-S, blue = Power Loop, yellow = Aggressive Bank
/// - Line from drone to exit point on spline
pub fn draw_maneuver_state(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    traj_query: Query<(
        &Transform,
        &ManeuverTrajectory,
        &AIController,
    )>,
    tilt_query: Query<(&Transform, &AIController, &TiltOverride)>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    // Drones with maneuver trajectory (Split-S / Power Loop)
    for (transform, traj, ai) in &traj_query {
        let pos = transform.translation;
        let color = maneuver_color(traj.kind);
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;

        // Draw the trajectory curve as a polyline
        let samples = 20;
        for i in 0..samples {
            let t0 = (i as f32 / samples as f32) * traj.curve_len;
            let t1 = ((i + 1) as f32 / samples as f32) * traj.curve_len;
            let p0 = traj.curve.position(t0);
            let p1 = traj.curve.position(t1);
            gizmos.line(p0, p1, color);
        }

        // Colored ring around the drone
        gizmos.circle(
            Isometry3d::new(pos, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            2.0,
            color,
        );

        // Line from drone to exit point on spline
        let exit_t = traj.exit_spline_t.rem_euclid(cycle_t);
        let exit_pos = ai.spline.position(exit_t);
        gizmos.line(pos, exit_pos, Color::srgba(1.0, 1.0, 1.0, 0.3));
        gizmos.sphere(Isometry3d::from_translation(exit_pos), 0.3, color);
    }

    // Drones with tilt override (Aggressive Bank)
    for (transform, ai, tilt) in &tilt_query {
        let pos = transform.translation;
        let color = maneuver_color(ManeuverKind::AggressiveBank);
        let cycle_t = ai.gate_count as f32 * POINTS_PER_GATE;

        // Yellow ring
        gizmos.circle(
            Isometry3d::new(pos, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            2.0,
            color,
        );

        // Line to exit point
        let exit_t = tilt.exit_spline_t.rem_euclid(cycle_t);
        let exit_pos = ai.spline.position(exit_t);
        gizmos.line(pos, exit_pos, Color::srgba(1.0, 1.0, 0.0, 0.3));
        gizmos.sphere(Isometry3d::from_translation(exit_pos), 0.3, color);
    }
}

fn maneuver_color(kind: ManeuverKind) -> Color {
    match kind {
        ManeuverKind::SplitS => Color::srgb(1.0, 0.2, 0.2),       // red
        ManeuverKind::PowerLoop => Color::srgb(0.2, 0.4, 1.0),    // blue
        ManeuverKind::AggressiveBank => Color::srgb(1.0, 1.0, 0.2), // yellow
    }
}
