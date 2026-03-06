use bevy::prelude::*;

use super::components::*;

const WANDER_SPEED: f32 = 8.0;
const WANDER_HEIGHT_MIN: f32 = 3.0;
const WANDER_HEIGHT_MAX: f32 = 12.0;
const WANDER_ARRIVE_DIST: f32 = 3.0;

/// Bounding box for wander waypoints, computed from course obstacle positions.
#[derive(Resource)]
pub struct WanderBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl WanderBounds {
    pub fn from_course(course: &crate::course::data::CourseData) -> Self {
        let padding = 20.0;
        if course.instances.is_empty() {
            return Self {
                min: Vec3::new(-padding, WANDER_HEIGHT_MIN, -padding),
                max: Vec3::new(padding, WANDER_HEIGHT_MAX, padding),
            };
        }
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for inst in &course.instances {
            min = min.min(inst.translation);
            max = max.max(inst.translation);
        }
        min.x -= padding;
        min.z -= padding;
        min.y = WANDER_HEIGHT_MIN;
        max.x += padding;
        max.z += padding;
        max.y = WANDER_HEIGHT_MAX;
        Self { min, max }
    }
}

/// Deterministic waypoint from drone index + step counter.
pub(super) fn wander_waypoint(drone_index: u8, step: u32, bounds: &WanderBounds) -> Vec3 {
    // Fibonacci hashing for good distribution
    let hash = (drone_index as u32)
        .wrapping_mul(2654435769)
        .wrapping_add(step.wrapping_mul(1013904223));
    let fx = ((hash & 0xFFFF) as f32) / 65535.0;
    let fz = (((hash >> 16) & 0xFFFF) as f32) / 65535.0;
    let fy = ((hash.wrapping_mul(7) & 0xFFFF) as f32) / 65535.0;
    Vec3::new(
        bounds.min.x + fx * (bounds.max.x - bounds.min.x),
        bounds.min.y + fy * (bounds.max.y - bounds.min.y),
        bounds.min.z + fz * (bounds.max.z - bounds.min.z),
    )
}

pub fn update_wander_targets(
    time: Res<Time>,
    bounds: Option<Res<WanderBounds>>,
    mut query: Query<(
        &Transform,
        &Drone,
        &DronePhase,
        &mut WanderState,
        &mut DesiredPosition,
    )>,
) {
    let Some(bounds) = bounds else { return };
    let dt = time.delta_secs();

    for (transform, drone, phase, mut wander, mut desired) in &mut query {
        if *phase != DronePhase::Wandering {
            continue;
        }

        let dist = (transform.translation - wander.target).length();
        wander.dwell_timer -= dt;

        if dist < WANDER_ARRIVE_DIST || wander.dwell_timer <= 0.0 {
            wander.step += 1;
            wander.target = wander_waypoint(drone.index, wander.step, &bounds);
            wander.dwell_timer = 3.0 + (wander.step % 3) as f32;
        }

        let dir = (wander.target - transform.translation).normalize_or(Vec3::Y);
        desired.position = wander.target;
        desired.velocity_hint = dir * WANDER_SPEED;
        desired.max_speed = WANDER_SPEED;
    }
}

/// Transition VictoryLap drones to Wandering on Results entry.
/// Racing drones are left alone — they keep following choreography until they
/// finish naturally, then fire_scripted_events transitions them to Wandering.
pub fn transition_to_wandering(
    mut commands: Commands,
    mut query: Query<(Entity, &Drone, &Transform, &mut DronePhase)>,
    bounds: Option<Res<WanderBounds>>,
) {
    let bounds = bounds.as_deref();
    for (entity, drone, transform, mut phase) in &mut query {
        if matches!(*phase, DronePhase::VictoryLap) {
            *phase = DronePhase::Wandering;
            // Clean up choreography components if still present
            commands
                .entity(entity)
                .remove::<ChoreographyState>()
                .remove::<BallisticState>();
            let target = bounds.map_or(
                transform.translation + Vec3::Y * 5.0,
                |b| wander_waypoint(drone.index, 0, b),
            );
            commands.entity(entity).insert(WanderState {
                target,
                dwell_timer: 2.0 + (drone.index % 3) as f32,
                step: 0,
            });
        }
    }
}

/// Build WanderBounds from CourseData on Results entry.
pub fn build_wander_bounds(
    mut commands: Commands,
    course: Option<Res<crate::course::data::CourseData>>,
) {
    if let Some(course) = course {
        commands.insert_resource(WanderBounds::from_course(&course));
    }
}
