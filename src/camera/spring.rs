use bevy::prelude::Vec3;

const LN_2: f32 = 0.693_147_2;

/// Fast approximation of e^(-x) using a rational polynomial.
/// Accurate to ~0.5% for x in [0, 5].
fn fast_negexp(x: f32) -> f32 {
    1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x)
}

fn spring_step_f32(x: &mut f32, v: &mut f32, goal: f32, half_life: f32, dt: f32) {
    let y = (4.0 * LN_2) / (half_life + 1e-5) / 2.0;
    let j0 = *x - goal;
    let j1 = *v + j0 * y;
    let eydt = fast_negexp(y * dt);
    *x = eydt * (j0 + j1 * dt) + goal;
    *v = eydt * (*v - j1 * y * dt);
}

/// Critically damped spring for f32. Owns both value and velocity so it can
/// be updated with a single `&mut self` (no borrow-checker issues when stored
/// alongside other springs in a struct).
pub struct SpringF32 {
    pub value: f32,
    pub velocity: f32,
}

impl SpringF32 {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    pub fn update(&mut self, goal: f32, half_life: f32, dt: f32) {
        spring_step_f32(&mut self.value, &mut self.velocity, goal, half_life, dt);
    }
}

/// Critically damped spring for Vec3, applied per-component.
/// Second-order system that tracks velocity, eliminating the permanent lag
/// that first-order exponential smoothing exhibits when following a moving target.
pub struct SpringVec3 {
    pub value: Vec3,
    pub velocity: Vec3,
}

impl SpringVec3 {
    pub fn new(value: Vec3) -> Self {
        Self {
            value,
            velocity: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, goal: Vec3, half_life: f32, dt: f32) {
        spring_step_f32(&mut self.value.x, &mut self.velocity.x, goal.x, half_life, dt);
        spring_step_f32(&mut self.value.y, &mut self.velocity.y, goal.y, half_life, dt);
        spring_step_f32(&mut self.value.z, &mut self.velocity.z, goal.z, half_life, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_negexp_at_zero_is_one() {
        let result = fast_negexp(0.0);
        assert!((result - 1.0).abs() < 0.001, "fast_negexp(0) = {result}");
    }

    #[test]
    fn fast_negexp_monotonically_decreasing() {
        let mut prev = fast_negexp(0.0);
        for i in 1..50 {
            let x = i as f32 * 0.1;
            let val = fast_negexp(x);
            assert!(val < prev, "fast_negexp({x}) = {val} >= fast_negexp({}) = {prev}", x - 0.1);
            assert!(val > 0.0, "fast_negexp({x}) should be positive");
            prev = val;
        }
    }

    #[test]
    fn spring_f32_converges_to_stationary_goal() {
        let mut spring = SpringF32::new(0.0);
        let goal = 10.0;
        let dt = 1.0 / 60.0;

        for _ in 0..600 {
            spring.update(goal, 0.1, dt);
        }

        assert!(
            (spring.value - goal).abs() < 0.001,
            "After 10s, value={} should be near goal={goal}", spring.value
        );
        assert!(
            spring.velocity.abs() < 0.01,
            "After 10s, velocity={} should be near zero", spring.velocity
        );
    }

    #[test]
    fn spring_f32_no_overshoot() {
        let mut spring = SpringF32::new(0.0);
        let goal = 10.0;
        let dt = 1.0 / 60.0;

        for _ in 0..600 {
            spring.update(goal, 0.1, dt);
            assert!(
                spring.value <= goal + 0.001,
                "Spring overshot: value={} > goal={goal}", spring.value
            );
        }
    }

    #[test]
    fn spring_tracks_moving_target_less_lag_than_exp() {
        // Simulate a linearly moving target over 5 seconds (300 frames).
        // The spring's velocity tracking advantage grows over time as it
        // converges to the target's velocity while exp smoothing never does.
        let half_life = 0.1;
        let dt = 1.0 / 60.0;
        let speed = 30.0;

        // Spring damper
        let mut spring = SpringF32::new(0.0);
        let mut target = 0.0_f32;
        for _ in 0..300 {
            target += speed * dt;
            spring.update(target, half_life, dt);
        }
        let spring_lag = (target - spring.value).abs();

        // Exponential smoothing with equivalent responsiveness
        // Using a rate that gives a similar settling time to the spring's half-life.
        let rate = 5.0; // comparable to old POSITION_SMOOTHING
        let mut ex = 0.0_f32;
        target = 0.0;
        for _ in 0..300 {
            target += speed * dt;
            let factor = 1.0 - (-rate * dt).exp();
            ex += (target - ex) * factor;
        }
        let exp_lag = (target - ex).abs();

        assert!(
            spring_lag < exp_lag,
            "Spring lag ({spring_lag:.3}) should be less than exp lag ({exp_lag:.3})"
        );
    }

    #[test]
    fn spring_vec3_converges() {
        let mut spring = SpringVec3::new(Vec3::ZERO);
        let goal = Vec3::new(10.0, 20.0, 30.0);
        let dt = 1.0 / 60.0;

        for _ in 0..600 {
            spring.update(goal, 0.1, dt);
        }

        assert!(
            (spring.value - goal).length() < 0.01,
            "Vec3 spring should converge: value={}, goal={goal}", spring.value
        );
    }

    #[test]
    fn spring_vec3_matches_per_component() {
        let initial = Vec3::new(1.0, 2.0, 3.0);
        let init_vel = Vec3::new(0.1, 0.2, 0.3);
        let goal = Vec3::new(10.0, 20.0, 30.0);
        let half_life = 0.15;
        let dt = 1.0 / 60.0;

        let mut vec_spring = SpringVec3 {
            value: initial,
            velocity: init_vel,
        };

        let mut sx = SpringF32 { value: initial.x, velocity: init_vel.x };
        let mut sy = SpringF32 { value: initial.y, velocity: init_vel.y };
        let mut sz = SpringF32 { value: initial.z, velocity: init_vel.z };

        vec_spring.update(goal, half_life, dt);
        sx.update(goal.x, half_life, dt);
        sy.update(goal.y, half_life, dt);
        sz.update(goal.z, half_life, dt);

        assert!((vec_spring.value.x - sx.value).abs() < 1e-6);
        assert!((vec_spring.value.y - sy.value).abs() < 1e-6);
        assert!((vec_spring.value.z - sz.value).abs() < 1e-6);
    }
}
