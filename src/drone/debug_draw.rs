use bevy::prelude::*;

use super::components::*;

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

/// Draw the full spline path as a polyline, color-coded by segment.
pub fn draw_spline_path(
    mut gizmos: Gizmos,
    debug: Option<Res<FlightDebugDraw>>,
    query: Query<&AIController, With<Drone>>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    // Use the first drone's spline (all drones share the same path)
    let Some(ai) = query.iter().next() else {
        return;
    };

    let total_t = ai.gate_count as f32 * POINTS_PER_GATE;
    let samples = (total_t * 20.0) as usize; // ~20 samples per spline unit
    if samples < 2 {
        return;
    }

    for i in 0..samples {
        let t0 = (i as f32 / samples as f32) * total_t;
        let t1 = ((i + 1) as f32 / samples as f32) * total_t;
        let p0 = ai.spline.position(t0);
        let p1 = ai.spline.position(t1);

        // Color by which half of the gate segment we're in:
        // approach half = yellow, departure half = cyan.
        let local_t = t0 % POINTS_PER_GATE;
        let color = if local_t < 1.0 {
            Color::srgb(1.0, 0.9, 0.2) // approach → gate midpoint: yellow
        } else {
            Color::srgb(0.2, 0.8, 1.0) // gate midpoint → departure: cyan
        };

        gizmos.line(p0, p1, color);
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

    const APPROACH_OFFSET: f32 = 8.0;

    for (i, (pos, fwd)) in ai
        .gate_positions
        .iter()
        .zip(ai.gate_forwards.iter())
        .enumerate()
    {
        let approach = *pos - *fwd * APPROACH_OFFSET;
        let departure = *pos + *fwd * APPROACH_OFFSET;

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
        &DesiredPosition,
        &DroneDynamics,
    )>,
) {
    let Some(debug) = debug else { return };
    if !debug.enabled {
        return;
    }

    for (transform, drone, ai, desired, dynamics) in &query {
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

        // Where the drone is on the spline (orange sphere)
        if ai.spline_t < total_t {
            let curve_pos = ai.spline.position(ai.spline_t);
            gizmos.sphere(
                Isometry3d::from_translation(curve_pos),
                0.2,
                Color::srgb(1.0, 0.5, 0.0),
            );

            // Line from drone to its spline position (orange, shows deviation)
            gizmos.line(pos, curve_pos, Color::srgba(1.0, 0.5, 0.0, 0.5));

            // Tangent at current spline position (yellow arrow)
            let tangent = ai.spline.velocity(ai.spline_t).normalize_or(Vec3::ZERO) * 2.0;
            gizmos.arrow(curve_pos, curve_pos + tangent, Color::srgb(1.0, 1.0, 0.0));
        }
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
