use bevy::prelude::*;

const START_DISTANCE_BEHIND_GATE: f32 = 5.0;
const HOVER_HEIGHT: f32 = 1.5;
/// Fraction of gate width used for the start line (leaves margin at edges).
const GATE_WIDTH_USAGE: f32 = 0.8;
/// Minimum lateral distance between drones in the start grid.
const MIN_LATERAL_SPACING: f32 = 1.0;
/// Distance between rows (along the through-gate axis).
const ROW_DEPTH_SPACING: f32 = 2.0;

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

    let usable_width = gate_half_width * 2.0 * GATE_WIDTH_USAGE;

    // How many drones fit in one row with minimum spacing?
    let cols = if count <= 1 {
        count as usize
    } else {
        let max_per_row = (usable_width / MIN_LATERAL_SPACING) as usize + 1;
        max_per_row.max(1).min(count as usize)
    };
    let rows = (count as usize + cols - 1) / cols;

    let mut positions = Vec::with_capacity(count as usize);
    for row in 0..rows {
        let row_start = row * cols;
        let row_count = (count as usize - row_start).min(cols);

        let spacing = if row_count > 1 {
            usable_width / (row_count as f32 - 1.0)
        } else {
            0.0
        };
        let center_offset = (row_count as f32 - 1.0) / 2.0;
        let row_offset = -through_dir * (row as f32 * ROW_DEPTH_SPACING);

        for i in 0..row_count {
            let col_offset = (i as f32 - center_offset) * spacing;
            let pos =
                start_center + lateral * col_offset + Vec3::Y * HOVER_HEIGHT + row_offset;
            positions.push(pos);
        }
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
    fn narrow_gate_uses_multiple_rows() {
        // Half-width 2.0 → usable_width = 3.2, fits 4 per row, so 12 drones → 3 rows
        let positions = compute_start_positions(
            Vec3::ZERO,
            Quat::IDENTITY,
            2.0,
            Vec3::NEG_Z,
            12,
        );
        assert_eq!(positions.len(), 12);

        // All drones within lateral bounds
        let usable = 2.0 * 2.0 * GATE_WIDTH_USAGE;
        for pos in &positions {
            assert!(
                pos.x.abs() <= usable / 2.0 + 0.01,
                "drone at x={} exceeds usable width {}",
                pos.x,
                usable,
            );
        }

        // Back rows should be further behind (higher Z since through_dir is -Z)
        // Row 0: z ≈ 5.0, Row 1: z ≈ 7.0, Row 2: z ≈ 9.0
        let row0_z = positions[0].z;
        let row1_z = positions[4].z;
        let row2_z = positions[8].z;
        assert!(
            row1_z > row0_z + 1.0,
            "row 1 (z={row1_z}) should be well behind row 0 (z={row0_z})"
        );
        assert!(
            row2_z > row1_z + 1.0,
            "row 2 (z={row2_z}) should be well behind row 1 (z={row1_z})"
        );

        // No overlap
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
