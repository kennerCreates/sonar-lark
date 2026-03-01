pub mod ai;
pub mod components;
pub mod debug_draw;
pub mod dev_dashboard;
pub mod droning;
pub mod explosion;
pub mod fireworks;
pub mod interpolation;
pub mod paths;
pub mod physics;
pub mod spawning;
pub mod wander;

use bevy::prelude::*;

use crate::course::data::CourseData;
use crate::race::lifecycle::drones_are_active;
use crate::states::AppState;
use components::AiTuningParams;

/// Run condition: true during Race or Results (drones fly in both states).
fn in_race_or_results(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Race | AppState::Results)
}

pub struct DronePlugin;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app
            // AI tuning params persist across race restarts
            .init_resource::<AiTuningParams>()
            // Start loading drone glTF, explosion, and firework assets when entering Race
            .add_systems(OnEnter(AppState::Race), (
                spawning::load_drone_gltf,
                explosion::load_explosion_assets,
                fireworks::load_firework_assets,
                droning::load_droning_sounds,
            ))
            // Restore authoritative physics transforms before snapshotting
            .add_systems(
                FixedFirst,
                interpolation::restore_physics_transforms
                    .run_if(in_race_or_results),
            )
            // Snapshot transforms before physics for camera interpolation
            .add_systems(
                FixedPreUpdate,
                interpolation::save_previous_transforms
                    .run_if(in_race_or_results),
            )
            // Extract drone meshes once glTF is loaded
            .add_systems(
                Update,
                spawning::setup_drone_assets
                    .run_if(in_state(AppState::Race))
                    .run_if(spawning::drone_gltf_ready)
                    .run_if(not(resource_exists::<spawning::DroneAssets>)),
            )
            // Spawn drones once assets and course data are available
            .add_systems(
                Update,
                spawning::spawn_drones
                    .run_if(in_state(AppState::Race))
                    .run_if(resource_exists::<spawning::DroneAssets>)
                    .run_if(resource_exists::<CourseData>),
            )
            // Physics chain in FixedUpdate.
            // AI systems only run when racing; physics always runs so drones hover at start.
            .add_systems(
                FixedUpdate,
                (
                    ai::update_ai_targets.run_if(drones_are_active),
                    ai::compute_racing_line.run_if(drones_are_active),
                    ai::proximity_avoidance.run_if(drones_are_active),
                    wander::update_wander_targets.run_if(drones_are_active),
                    physics::hover_target.run_if(not(drones_are_active)),
                    physics::position_pid,
                    physics::attitude_controller,
                    physics::dirty_air_perturbation,
                    physics::motor_lag,
                    physics::apply_forces,
                    physics::integrate_motion,
                    physics::clamp_transform,
                )
                    .chain()
                    .run_if(in_race_or_results),
            )
            // Save authoritative physics state after the physics chain
            .add_systems(
                FixedPostUpdate,
                interpolation::save_physics_transforms
                    .run_if(in_race_or_results),
            )
            // Interpolate drone transforms for smooth rendering between physics ticks
            .add_systems(
                PostUpdate,
                interpolation::interpolate_visual_transforms
                    .run_if(in_race_or_results),
            )
            // Flight debug visualization (F3 to toggle)
            .add_systems(
                Update,
                (
                    debug_draw::toggle_debug_draw,
                    debug_draw::draw_spline_path,
                    debug_draw::draw_gate_markers,
                    debug_draw::draw_gate_planes,
                    debug_draw::draw_drone_state,
                    debug_draw::draw_progress_indicators,
                )
                    .run_if(in_state(AppState::Race)),
            )
            // Dev dashboard (F4 to toggle)
            .add_systems(
                Update,
                (
                    dev_dashboard::toggle_dev_dashboard,
                    dev_dashboard::handle_param_buttons,
                    dev_dashboard::handle_reset_button,
                    dev_dashboard::update_param_labels,
                    dev_dashboard::update_button_colors,
                )
                    .run_if(in_state(AppState::Race)),
            )
            // Ambient droning sound (overlapping crossfade)
            .add_systems(
                Update,
                droning::update_droning
                    .run_if(in_race_or_results),
            )
            // Explosion particle update
            .add_systems(
                Update,
                explosion::update_explosion_particles
                    .run_if(in_race_or_results),
            )
            // Fireworks: detect first finish, tick pending shells, update particles
            .add_systems(
                Update,
                fireworks::detect_first_finish
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(
                Update,
                (fireworks::tick_firework_shells, fireworks::update_firework_particles)
                    .run_if(in_race_or_results),
            )
            // Transition drones to wandering on Results entry
            .add_systems(
                OnEnter(AppState::Results),
                (wander::build_wander_bounds, wander::transition_to_wandering).chain(),
            )
            // Cleanup resources when leaving Results (drones persist Race → Results)
            .add_systems(OnExit(AppState::Results), (
                spawning::cleanup_drone_resources,
                explosion::cleanup_explosion_assets,
                fireworks::cleanup_firework_assets,
                droning::cleanup_droning,
                cleanup_wander_bounds,
            ));
    }
}

fn cleanup_wander_bounds(mut commands: Commands) {
    commands.remove_resource::<wander::WanderBounds>();
}
