use bevy::prelude::*;

#[derive(Component)]
pub struct GateIndex(pub u32);

/// World-space forward direction of the gate (the expected approach direction).
#[derive(Component)]
pub struct GateForward(pub Vec3);
