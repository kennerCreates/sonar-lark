use bevy::prelude::*;

use crate::course::data::PropKind;
use crate::editor::workshop::{CollisionVolumeData, WorkshopState};
use crate::obstacle::definition::ObstacleId;

// --- Generic Undo Stack ---

const MAX_UNDO: usize = 50;

#[derive(Resource)]
pub struct UndoStack<A> {
    undo: Vec<A>,
    redo: Vec<A>,
}

impl<A> Default for UndoStack<A> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl<A> UndoStack<A> {
    pub fn push(&mut self, action: A) {
        self.undo.push(action);
        self.redo.clear();
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
    }

    pub fn pop_undo(&mut self) -> Option<A> {
        self.undo.pop()
    }

    pub fn pop_redo(&mut self) -> Option<A> {
        self.redo.pop()
    }

    pub fn push_redo(&mut self, action: A) {
        self.redo.push(action);
    }

    pub fn push_undo_only(&mut self, action: A) {
        self.undo.push(action);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

// --- Course Editor Actions ---

#[derive(Clone)]
pub struct CameraSnapshot {
    pub offset: Vec3,
    pub rotation: Quat,
    pub is_primary: bool,
    #[allow(dead_code)]
    pub label: Option<String>,
}

#[derive(Clone)]
pub enum CourseEditorAction {
    TransformChange {
        entity: Entity,
        before: Transform,
        after: Transform,
    },
    SpawnObstacle {
        entity: Entity,
        obstacle_id: ObstacleId,
        transform: Transform,
        gate_order: Option<u32>,
        gate_forward_flipped: bool,
        camera: Option<CameraSnapshot>,
    },
    SpawnProp {
        entity: Entity,
        kind: PropKind,
        transform: Transform,
        color_override: Option<[f32; 4]>,
    },
    SpawnCamera {
        camera_entity: Entity,
        parent_gate: Entity,
        offset: Vec3,
        rotation: Quat,
        is_primary: bool,
    },
    DeleteObstacle {
        old_entity: Entity,
        obstacle_id: ObstacleId,
        transform: Transform,
        gate_order: Option<u32>,
        gate_forward_flipped: bool,
        camera: Option<CameraSnapshot>,
    },
    DeleteProp {
        old_entity: Entity,
        kind: PropKind,
        transform: Transform,
        color_override: Option<[f32; 4]>,
    },
    DeleteCamera {
        old_entity: Entity,
        parent_gate: Entity,
        offset: Vec3,
        rotation: Quat,
        is_primary: bool,
    },
    FlipGate {
        entity: Entity,
    },
    PropColorChange {
        entity: Entity,
        before: Option<[f32; 4]>,
        after: Option<[f32; 4]>,
    },
}

impl CourseEditorAction {
    /// Replace all references to `old` entity with `new` in this action.
    pub fn remap_entity(&mut self, old: Entity, new: Entity) {
        match self {
            Self::TransformChange { entity, .. } if *entity == old => *entity = new,
            Self::SpawnObstacle { entity, .. } if *entity == old => *entity = new,
            Self::SpawnProp { entity, .. } if *entity == old => *entity = new,
            Self::SpawnCamera {
                camera_entity,
                parent_gate,
                ..
            } => {
                if *camera_entity == old {
                    *camera_entity = new;
                }
                if *parent_gate == old {
                    *parent_gate = new;
                }
            }
            Self::DeleteObstacle { old_entity, .. } if *old_entity == old => *old_entity = new,
            Self::DeleteProp { old_entity, .. } if *old_entity == old => *old_entity = new,
            Self::DeleteCamera {
                old_entity,
                parent_gate,
                ..
            } => {
                if *old_entity == old {
                    *old_entity = new;
                }
                if *parent_gate == old {
                    *parent_gate = new;
                }
            }
            Self::FlipGate { entity } if *entity == old => *entity = new,
            Self::PropColorChange { entity, .. } if *entity == old => *entity = new,
            _ => {}
        }
    }
}

/// Remap all entity references in the undo stack from `old` to `new`.
pub fn remap_entity_in_stack(stack: &mut UndoStack<CourseEditorAction>, old: Entity, new: Entity) {
    for action in stack.undo.iter_mut() {
        action.remap_entity(old, new);
    }
    for action in stack.redo.iter_mut() {
        action.remap_entity(old, new);
    }
}

// --- Workshop Actions ---

#[derive(Clone)]
pub struct WorkshopSnapshot {
    pub model_offset: Vec3,
    pub model_rotation: Quat,
    pub has_trigger: bool,
    pub trigger_offset: Vec3,
    pub trigger_half_extents: Vec3,
    pub trigger_rotation: Quat,
    pub has_collision: bool,
    pub collision_offset: Vec3,
    pub collision_half_extents: Vec3,
    pub collision_rotation: Quat,
    pub collision_volumes: Vec<CollisionVolumeData>,
    pub active_collision_idx: usize,
    pub has_camera: bool,
    pub camera_offset: Vec3,
    pub camera_rotation: Quat,
}

impl WorkshopSnapshot {
    pub fn capture(state: &WorkshopState) -> Self {
        Self {
            model_offset: state.model_offset,
            model_rotation: state.model_rotation,
            has_trigger: state.has_trigger,
            trigger_offset: state.trigger_offset,
            trigger_half_extents: state.trigger_half_extents,
            trigger_rotation: state.trigger_rotation,
            has_collision: state.has_collision,
            collision_offset: state.collision_offset,
            collision_half_extents: state.collision_half_extents,
            collision_rotation: state.collision_rotation,
            collision_volumes: state.collision_volumes.clone(),
            active_collision_idx: state.active_collision_idx,
            has_camera: state.has_camera,
            camera_offset: state.camera_offset,
            camera_rotation: state.camera_rotation,
        }
    }

    pub fn restore_to(&self, state: &mut WorkshopState) {
        state.model_offset = self.model_offset;
        state.model_rotation = self.model_rotation;
        state.has_trigger = self.has_trigger;
        state.trigger_offset = self.trigger_offset;
        state.trigger_half_extents = self.trigger_half_extents;
        state.trigger_rotation = self.trigger_rotation;
        state.has_collision = self.has_collision;
        state.collision_offset = self.collision_offset;
        state.collision_half_extents = self.collision_half_extents;
        state.collision_rotation = self.collision_rotation;
        state.collision_volumes = self.collision_volumes.clone();
        state.active_collision_idx = self.active_collision_idx;
        state.has_camera = self.has_camera;
        state.camera_offset = self.camera_offset;
        state.camera_rotation = self.camera_rotation;
    }
}

#[derive(Clone)]
pub enum WorkshopAction {
    StateChange {
        before: WorkshopSnapshot,
        after: WorkshopSnapshot,
    },
}

// --- Input helpers ---

pub fn ctrl_held(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_undo() {
        let mut stack = UndoStack::<i32>::default();
        stack.push(1);
        stack.push(2);
        assert_eq!(stack.pop_undo(), Some(2));
        assert_eq!(stack.pop_undo(), Some(1));
        assert_eq!(stack.pop_undo(), None);
    }

    #[test]
    fn push_clears_redo() {
        let mut stack = UndoStack::<i32>::default();
        stack.push(1);
        stack.push(2);
        let popped = stack.pop_undo().unwrap();
        stack.push_redo(popped);
        assert_eq!(stack.pop_redo(), Some(2));

        // Push a new action — redo should clear
        stack.push_redo(2);
        stack.push(3);
        assert_eq!(stack.pop_redo(), None);
    }

    #[test]
    fn max_capacity() {
        let mut stack = UndoStack::<i32>::default();
        for i in 0..60 {
            stack.push(i);
        }
        // Should only have the last MAX_UNDO entries
        let mut count = 0;
        while stack.pop_undo().is_some() {
            count += 1;
        }
        assert_eq!(count, MAX_UNDO);
    }

    #[test]
    fn clear_empties_both() {
        let mut stack = UndoStack::<i32>::default();
        stack.push(1);
        stack.push(2);
        let popped = stack.pop_undo().unwrap();
        stack.push_redo(popped);
        stack.clear();
        assert_eq!(stack.pop_undo(), None);
        assert_eq!(stack.pop_redo(), None);
    }

    #[test]
    fn push_undo_only_does_not_clear_redo() {
        let mut stack = UndoStack::<i32>::default();
        stack.push(1);
        let popped = stack.pop_undo().unwrap();
        stack.push_redo(popped);
        stack.push_undo_only(2);
        assert_eq!(stack.pop_redo(), Some(1));
    }
}
