use bevy::prelude::*;

use crate::course::data::{CourseData, ObstacleInstance, PropKind};
use crate::drone::ai::{cyclic_curvature, safe_speed_for_curvature};
use crate::common::POINTS_PER_GATE;
use crate::drone::components::AiTuningParams;
use crate::drone::paths::generate_race_path;
use crate::obstacle::library::ObstacleLibrary;
use crate::palette;

use super::{PlacedCamera, PlacedObstacle, PlacedProp};

// --- Gizmo group ---

#[derive(Default, Reflect, GizmoConfigGroup)]
pub(super) struct CourseGizmoGroup;

// --- Gate sequence lines ---

pub(super) fn draw_gate_sequence_lines(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    placed_query: Query<(&PlacedObstacle, &GlobalTransform)>,
) {
    let mut gates: Vec<(u32, Vec3)> = placed_query
        .iter()
        .filter_map(|(placed, gt)| placed.gate_order.map(|order| (order, gt.translation())))
        .collect();

    gates.sort_by_key(|(order, _)| *order);

    let Color::Srgba(gate_start) = palette::GREEN else { unreachable!() };
    let Color::Srgba(gate_end) = palette::SUNFLOWER else { unreachable!() };
    let count = gates.len();
    for (i, (_, pos)) in gates.iter().enumerate() {
        let t = if count > 1 {
            i as f32 / (count - 1) as f32
        } else {
            0.0
        };
        let color = Color::srgb(
            gate_start.red + (gate_end.red - gate_start.red) * t,
            gate_start.green + (gate_end.green - gate_start.green) * t,
            gate_start.blue + (gate_end.blue - gate_start.blue) * t,
        );
        let iso = Isometry3d::new(*pos, Quat::IDENTITY);
        gizmos.sphere(iso, 0.5, color);
    }
}

// --- Gate forward arrows ---

pub(super) fn draw_gate_forward_arrows(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    placed_query: Query<(&PlacedObstacle, &Transform)>,
    library: Res<ObstacleLibrary>,
) {
    for (placed, transform) in &placed_query {
        if placed.gate_order.is_none() {
            continue;
        }
        let Some(def) = library.get(&placed.obstacle_id) else {
            continue;
        };
        let Some(tv) = &def.trigger_volume else {
            continue;
        };
        let center = transform.translation + transform.rotation * (tv.offset * transform.scale);
        let local_fwd = if placed.gate_forward_flipped {
            -tv.forward
        } else {
            tv.forward
        };
        let world_fwd = transform.rotation * local_fwd;
        gizmos.arrow(center, center + world_fwd * 3.0, palette::CERULEAN);
    }
}

// --- Flight spline preview ---

const SPLINE_PREVIEW_STEP: f32 = 0.1;

#[derive(Default)]
pub(super) struct CachedSplinePreview {
    obstacle_count: usize,
    segments: Vec<(Vec3, Vec3, Color)>,
}

pub(super) fn draw_flight_spline_preview(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    placed_query: Query<(&PlacedObstacle, &Transform)>,
    library: Res<ObstacleLibrary>,
    tuning: Res<AiTuningParams>,
    mut cache: Local<CachedSplinePreview>,
    changed_query: Query<
        (),
        (
            Or<(Changed<Transform>, Changed<PlacedObstacle>)>,
            With<PlacedObstacle>,
        ),
    >,
) {
    let obstacle_count = placed_query.iter().count();
    let needs_rebuild = cache.segments.is_empty()
        || cache.obstacle_count != obstacle_count
        || !changed_query.is_empty()
        || tuning.is_changed();

    if needs_rebuild {
        cache.obstacle_count = obstacle_count;
        cache.segments.clear();

        let instances: Vec<ObstacleInstance> = placed_query
            .iter()
            .map(|(placed, transform)| ObstacleInstance {
                obstacle_id: placed.obstacle_id.clone(),
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
                gate_order: placed.gate_order,
                gate_forward_flipped: placed.gate_forward_flipped,
                camera: None,
                color_override: placed.color_override,
            })
            .collect();

        let course = CourseData {
            name: String::new(),
            instances,
            props: vec![],
            cameras: vec![],
            location: String::new(),
        };

        let Some(race_path) = generate_race_path(&course, &library) else {
            return;
        };

        let gate_count = race_path.gate_positions.len() as f32;
        let cycle_t = gate_count * POINTS_PER_GATE;
        let spline = &race_path.spline;
        let speed_range = (tuning.max_speed - tuning.min_curvature_speed).max(0.001);
        let Color::Srgba(slow_c) = palette::NEON_RED else { unreachable!() };
        let Color::Srgba(fast_c) = palette::GREEN else { unreachable!() };

        let mut t = 0.0f32;
        let mut prev_pos = spline.position(0.0);
        t += SPLINE_PREVIEW_STEP;

        while t <= cycle_t {
            let pos = spline.position(t.rem_euclid(cycle_t));
            let k = cyclic_curvature(spline, t, cycle_t);
            let v_safe = safe_speed_for_curvature(k, &tuning);
            let ratio = ((v_safe - tuning.min_curvature_speed) / speed_range).clamp(0.0, 1.0);
            let color = Color::srgb(
                slow_c.red + (fast_c.red - slow_c.red) * ratio,
                slow_c.green + (fast_c.green - slow_c.green) * ratio,
                slow_c.blue + (fast_c.blue - slow_c.blue) * ratio,
            );
            cache.segments.push((prev_pos, pos, color));
            prev_pos = pos;
            t += SPLINE_PREVIEW_STEP;
        }

        // Close the loop
        let start_pos = spline.position(0.0);
        let k = cyclic_curvature(spline, 0.0, cycle_t);
        let v_safe = safe_speed_for_curvature(k, &tuning);
        let ratio = ((v_safe - tuning.min_curvature_speed) / speed_range).clamp(0.0, 1.0);
        let color = Color::srgb(
            slow_c.red + (fast_c.red - slow_c.red) * ratio,
            slow_c.green + (fast_c.green - slow_c.green) * ratio,
            slow_c.blue + (fast_c.blue - slow_c.blue) * ratio,
        );
        cache.segments.push((prev_pos, start_pos, color));
    }

    // Draw cached segments
    for &(start, end, color) in &cache.segments {
        gizmos.line(start, end, color);
    }
}

// --- Camera Gizmos ---

pub(super) fn draw_camera_gizmos(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    camera_query: Query<(&PlacedCamera, &GlobalTransform)>,
) {
    for (camera, gt) in &camera_query {
        let (_, rotation, pos) = gt.to_scale_rotation_translation();
        let forward = rotation * Vec3::NEG_Z;
        let right = rotation * Vec3::X;
        let up = rotation * Vec3::Y;

        let color = if camera.is_primary {
            palette::SUNSHINE
        } else {
            palette::SKY
        };

        // Camera body sphere
        let iso = Isometry3d::new(pos, Quat::IDENTITY);
        gizmos.sphere(iso, 0.35, color);

        // Frustum wireframe
        let dist = 2.0;
        let half_h = 0.6;
        let half_w = half_h * 1.778; // 16:9

        let center = pos + forward * dist;
        let corners = [
            center + right * half_w + up * half_h,
            center - right * half_w + up * half_h,
            center - right * half_w - up * half_h,
            center + right * half_w - up * half_h,
        ];

        for i in 0..4 {
            gizmos.line(corners[i], corners[(i + 1) % 4], color);
            gizmos.line(pos, corners[i], color);
        }

        // Up indicator
        gizmos.arrow(pos, pos + up * 0.8, color);
    }
}

// --- Prop Gizmos ---

pub(super) fn draw_prop_gizmos(
    mut gizmos: Gizmos<CourseGizmoGroup>,
    prop_query: Query<(&PlacedProp, &Transform)>,
) {
    for (prop, transform) in &prop_query {
        let pos = transform.translation;
        let forward = transform.rotation * Vec3::NEG_Z;
        let up = transform.rotation * Vec3::Y;

        match prop.kind {
            PropKind::ConfettiEmitter => {
                let iso = Isometry3d::new(pos, Quat::IDENTITY);
                gizmos.sphere(iso, 0.6, palette::SUNSHINE);
                // Upward arrows showing burst direction
                for offset in [-0.3f32, 0.0, 0.3] {
                    let base = pos + forward * offset;
                    gizmos.arrow(base, base + Vec3::Y * 1.5, palette::SUNSHINE);
                }
            }
            PropKind::ShellBurstEmitter => {
                let iso = Isometry3d::new(pos, Quat::IDENTITY);
                gizmos.sphere(iso, 0.6, palette::TANGERINE);
                // Starburst lines above showing detonation area
                let burst_center = pos + up * 3.0;
                for angle_deg in (0..360).step_by(45) {
                    let angle = (angle_deg as f32).to_radians();
                    let dir = Vec3::new(angle.cos(), 0.5, angle.sin()).normalize();
                    gizmos.line(burst_center, burst_center + dir * 1.5, palette::TANGERINE);
                }
                gizmos.line(pos, burst_center, palette::DANDELION);
            }
        }
    }
}
