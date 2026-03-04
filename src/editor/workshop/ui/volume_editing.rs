use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::palette;
use crate::ui_theme;

use super::build::*;
use crate::editor::workshop::{CollisionVolumeData, EditTarget, WorkshopState};

pub fn handle_trigger_toggle(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<HasTriggerToggle>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.has_trigger = !state.has_trigger;
        }
    }
}

pub fn handle_collision_toggle(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<HasCollisionToggle>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.has_collision = !state.has_collision;
            if state.has_collision && state.collision_volumes.is_empty() {
                let vol = CollisionVolumeData::default();
                state.collision_offset = vol.offset;
                state.collision_half_extents = vol.half_extents;
                state.collision_rotation = vol.rotation;
                state.collision_volumes.push(vol);
                state.active_collision_idx = 0;
            }
            if !state.has_collision {
                state.collision_volumes.clear();
                state.active_collision_idx = 0;
            }
        }
    }
}

pub fn handle_edit_target_toggle(
    mut state: ResMut<WorkshopState>,
    model_query: Query<&Interaction, (Changed<Interaction>, With<EditTargetRadioModel>)>,
    trigger_query: Query<&Interaction, (Changed<Interaction>, With<EditTargetRadioTrigger>)>,
    collision_query: Query<&Interaction, (Changed<Interaction>, With<EditTargetRadioCollision>)>,
) {
    for interaction in &model_query {
        if *interaction == Interaction::Pressed {
            state.edit_target = EditTarget::Model;
        }
    }
    for interaction in &trigger_query {
        if *interaction == Interaction::Pressed {
            state.edit_target = EditTarget::Trigger;
        }
    }
    for interaction in &collision_query {
        if *interaction == Interaction::Pressed {
            state.edit_target = EditTarget::Collision;
        }
    }
}

pub fn handle_name_field_focus(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<NameFieldButton>)>,
    mut border: Query<&mut BorderColor, With<NameFieldButton>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            state.editing_name = true;
            if let Ok(mut b) = border.single_mut() {
                *b = BorderColor::all(palette::SKY);
            }
        }
    }
}

pub fn handle_name_text_input(
    mut state: ResMut<WorkshopState>,
    mut events: MessageReader<KeyboardInput>,
    mut border: Query<&mut BorderColor, With<NameFieldButton>>,
) {
    if !state.editing_name {
        return;
    }

    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter | Key::Escape => {
                state.editing_name = false;
                if let Ok(mut b) = border.single_mut() {
                    *b = BorderColor::all(palette::STEEL);
                }
            }
            Key::Backspace => {
                state.obstacle_name.pop();
            }
            Key::Space => {
                state.obstacle_name.push('_');
            }
            Key::Character(c) => {
                for ch in c.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                        state.obstacle_name.push(ch.to_ascii_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn update_display_values(
    state: Res<WorkshopState>,
    mut name_text: Query<&mut Text, (With<NameDisplayText>, Without<HasTriggerText>, Without<HasCollisionText>, Without<CollisionShapeLabel>)>,
    mut trigger_text: Query<&mut Text, (With<HasTriggerText>, Without<NameDisplayText>, Without<HasCollisionText>, Without<CollisionShapeLabel>)>,
    mut collision_text: Query<&mut Text, (With<HasCollisionText>, Without<NameDisplayText>, Without<HasTriggerText>, Without<CollisionShapeLabel>)>,
    mut shape_label: Query<&mut Text, (With<CollisionShapeLabel>, Without<NameDisplayText>, Without<HasTriggerText>, Without<HasCollisionText>)>,
    mut bgs: ParamSet<(
        Query<&mut BackgroundColor, With<HasTriggerToggle>>,
        Query<&mut BackgroundColor, With<HasCollisionToggle>>,
        Query<&mut BackgroundColor, With<EditTargetRadioModel>>,
        Query<&mut BackgroundColor, With<EditTargetRadioTrigger>>,
        Query<&mut BackgroundColor, With<EditTargetRadioCollision>>,
    )>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut text) = name_text.single_mut() {
        let display = if state.obstacle_name.is_empty() {
            if state.editing_name {
                "|".to_string()
            } else {
                "(type a name)".to_string()
            }
        } else if state.editing_name {
            format!("{}|", state.obstacle_name)
        } else {
            state.obstacle_name.clone()
        };
        **text = display;
    }

    if let Ok(mut text) = trigger_text.single_mut() {
        **text = if state.has_trigger { "ON" } else { "OFF" }.to_string();
    }
    if let Ok(mut text) = collision_text.single_mut() {
        **text = if state.has_collision { "ON" } else { "OFF" }.to_string();
    }
    if let Ok(mut text) = shape_label.single_mut() {
        let total = state.collision_volumes.len();
        if total == 0 {
            **text = "Shape 0/0".to_string();
        } else {
            **text = format!("Shape {}/{}", state.active_collision_idx + 1, total);
        }
    }

    if let Ok(mut bg) = bgs.p0().single_mut() {
        *bg = BackgroundColor(if state.has_trigger { ui_theme::TOGGLE_ON } else { ui_theme::TOGGLE_OFF });
    }
    if let Ok(mut bg) = bgs.p1().single_mut() {
        *bg = BackgroundColor(if state.has_collision { ui_theme::TOGGLE_ON } else { ui_theme::TOGGLE_OFF });
    }

    if let Ok(mut bg) = bgs.p2().single_mut() {
        *bg = BackgroundColor(if state.edit_target == EditTarget::Model { RADIO_ACTIVE } else { RADIO_INACTIVE });
    }
    if let Ok(mut bg) = bgs.p3().single_mut() {
        *bg = BackgroundColor(if state.edit_target == EditTarget::Trigger { RADIO_ACTIVE } else { RADIO_INACTIVE });
    }
    if let Ok(mut bg) = bgs.p4().single_mut() {
        *bg = BackgroundColor(if state.edit_target == EditTarget::Collision { RADIO_ACTIVE } else { RADIO_INACTIVE });
    }
}

pub fn handle_add_collision_shape(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<AddCollisionShapeButton>)>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !state.has_collision {
            continue;
        }
        // Save current working copy before adding
        state.sync_active_to_vec();
        let new_vol = CollisionVolumeData::default();
        state.collision_volumes.push(new_vol);
        state.active_collision_idx = state.collision_volumes.len() - 1;
        state.load_active_from_vec();
        state.edit_target = EditTarget::Collision;
    }
}

pub fn handle_remove_collision_shape(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<RemoveCollisionShapeButton>)>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if state.collision_volumes.len() <= 1 {
            continue; // Keep at least one shape while collision is enabled
        }
        let idx = state.active_collision_idx;
        state.collision_volumes.remove(idx);
        if state.active_collision_idx >= state.collision_volumes.len() {
            state.active_collision_idx = state.collision_volumes.len() - 1;
        }
        state.load_active_from_vec();
    }
}

pub fn handle_prev_collision_shape(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<PrevCollisionShapeButton>)>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if state.collision_volumes.len() <= 1 || state.active_collision_idx == 0 {
            continue;
        }
        state.sync_active_to_vec();
        state.active_collision_idx -= 1;
        state.load_active_from_vec();
    }
}

pub fn handle_next_collision_shape(
    mut state: ResMut<WorkshopState>,
    query: Query<&Interaction, (Changed<Interaction>, With<NextCollisionShapeButton>)>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if state.active_collision_idx + 1 >= state.collision_volumes.len() {
            continue;
        }
        state.sync_active_to_vec();
        state.active_collision_idx += 1;
        state.load_active_from_vec();
    }
}
