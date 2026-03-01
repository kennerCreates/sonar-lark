/// Spline control points per gate: approach, departure, midleg-to-next.
pub const POINTS_PER_GATE: f32 = 3.0;

/// How far past a full cycle the race extends. Drones must fly through
/// the start/finish gate again (completing a full lap) before transitioning.
/// 1.5 puts the finish well past gate 0's departure (at cycle + 1.0).
pub const FINISH_EXTENSION: f32 = 1.5;
