use bevy::prelude::*;

const START_DISTANCE_BEHIND_GATE: f32 = 5.0;
const HOVER_HEIGHT: f32 = 1.5;
/// Fraction of gate width used for the start line (leaves margin at edges).
const GATE_WIDTH_USAGE: f32 = 0.8;

pub fn compute_start_positions(
    gate_translation: Vec3,
    gate_rotation: Quat,
    gate_half_width: f32,
    gate_forward: Vec3,
    count: u8,
) -> Vec<Vec3> {
    // Use the explicit gate forward direction (projected to XZ plane).
    let through_dir = Vec3::new(gate_forward.x, 0.0, gate_forward.z).normalize_or(Vec3::NEG_Z);

    // Start line center: behind the gate at ground level
    let start_center = Vec3::new(gate_translation.x, 0.0, gate_translation.z)
        - through_dir * START_DISTANCE_BEHIND_GATE;

    // Gate's local X axis projected to XZ = lateral spread direction
    let gate_x = gate_rotation * Vec3::X;
    let lateral = Vec3::new(gate_x.x, 0.0, gate_x.z).normalize_or(Vec3::X);

    // Fit drones within a fraction of the gate width
    let usable_width = gate_half_width * 2.0 * GATE_WIDTH_USAGE;
    let spacing = if count > 1 {
        usable_width / (count as f32 - 1.0)
    } else {
        0.0
    };

    let center_offset = (count as f32 - 1.0) / 2.0;
    let mut positions = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let col_offset = (i as f32 - center_offset) * spacing;
        let pos = start_center + lateral * col_offset + Vec3::Y * HOVER_HEIGHT;
        positions.push(pos);
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_start_positions_correct_count() {
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            5.0,
            Vec3::NEG_Z,
            12,
        );
        assert_eq!(positions.len(), 12);
    }

    #[test]
    fn compute_start_positions_behind_first_gate() {
        // Gate at origin, identity rotation, next gate at -Z.
        // Through direction is -Z, so drones should be at +Z (behind).
        let gate_pos = Vec3::ZERO;
        let positions = compute_start_positions(
            gate_pos,
            Quat::IDENTITY,
            5.0,
            Vec3::NEG_Z,
            12,
        );

        for pos in &positions {
            assert!(
                pos.z > gate_pos.z,
                "drone at z={} should be behind gate at z={}",
                pos.z,
                gate_pos.z
            );
        }
    }

    #[test]
    fn compute_start_positions_at_hover_height() {
        let positions = compute_start_positions(
            Vec3::new(0.0, 10.0, 0.0), // elevated gate
            Quat::IDENTITY,
            5.0,
            Vec3::NEG_Z,
            4,
        );

        for pos in &positions {
            assert!(
                (pos.y - HOVER_HEIGHT).abs() < 0.01,
                "drone at y={} should be at HOVER_HEIGHT={}, not gate elevation",
                pos.y,
                HOVER_HEIGHT,
            );
        }
    }

    #[test]
    fn compute_start_positions_fits_within_gate_width() {
        let half_width = 5.0;
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            half_width,
            Vec3::NEG_Z,
            12,
        );

        // With identity rotation, lateral axis is X
        let usable = half_width * 2.0 * GATE_WIDTH_USAGE;
        for pos in &positions {
            assert!(
                pos.x.abs() <= usable / 2.0 + 0.01,
                "drone at x={} exceeds usable width {}",
                pos.x,
                usable,
            );
        }
    }

    #[test]
    fn compute_start_positions_no_overlap() {
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            8.0,
            Vec3::NEG_Z,
            12,
        );

        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let dist = (positions[i] - positions[j]).length();
                assert!(dist > 0.5, "drones {} and {} too close: {:.2}", i, j, dist);
            }
        }
    }

    #[test]
    fn compute_start_positions_respects_gate_rotation() {
        // Gate rotated 90 degrees around Y — lateral axis becomes world Z
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let positions = compute_start_positions(
            Vec3::ZERO,
            rotation,
            5.0,
            Vec3::NEG_X,
            3,
        );

        // All drones should share the same X (spread along Z, not X)
        let first_x = positions[0].x;
        for pos in &positions {
            assert!(
                (pos.x - first_x).abs() < 0.01,
                "expected drones aligned on X, got x={}",
                pos.x,
            );
        }
        // But Z should differ
        assert!(
            (positions[0].z - positions[2].z).abs() > 1.0,
            "drones should be spread along Z"
        );
    }
}
