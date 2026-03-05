use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::editor::undo::UndoStack;
use crate::palette;
use crate::states::AppState;
use crate::ui_theme;

use super::canvas::{self, CANVAS_DISPLAY_HEIGHT, CANVAS_DISPLAY_WIDTH, CANVAS_HEIGHT, CANVAS_WIDTH};
use super::ui::POSTER_COLORS;
use super::{
    BrushSizeButton, CanvasContainer, PaintStroke, PosterAction, PosterCanvas, PosterColorCell,
    PosterEditorState, PosterStartRaceButton, PosterTextElement, PosterTool, ToolButtonMarker,
};

// --- Tool selection ---

pub fn handle_tool_selection(
    mut state: ResMut<PosterEditorState>,
    query: Query<(&Interaction, &ToolButtonMarker), Changed<Interaction>>,
    mut all_buttons: Query<(&ToolButtonMarker, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (interaction, marker) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // If we were editing text, finalize it
        state.editing_text = None;
        state.active_tool = marker.0;

        // Update button visuals
        for (btn_marker, mut bg, mut border) in &mut all_buttons {
            if btn_marker.0 == marker.0 {
                *bg = BackgroundColor(ui_theme::BUTTON_SELECTED);
                *border = BorderColor::all(palette::VANILLA);
            } else {
                *bg = BackgroundColor(ui_theme::BUTTON_NORMAL);
                *border = BorderColor::all(ui_theme::BORDER_NORMAL);
            }
        }
    }
}

// --- Color selection ---

pub fn handle_color_selection(
    mut state: ResMut<PosterEditorState>,
    query: Query<(&Interaction, &PosterColorCell), Changed<Interaction>>,
    mut all_cells: Query<(&PosterColorCell, &mut BorderColor)>,
) {
    for (interaction, cell) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let color = POSTER_COLORS[cell.0].1;
        let rgba = color.to_srgba();
        state.brush_color = [
            (rgba.red * 255.0) as u8,
            (rgba.green * 255.0) as u8,
            (rgba.blue * 255.0) as u8,
            255,
        ];

        // Highlight selected cell
        for (c, mut border) in &mut all_cells {
            if c.0 == cell.0 {
                *border = BorderColor::all(palette::VANILLA);
            } else {
                *border = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.3));
            }
        }
    }
}

// --- Brush size ---

pub fn handle_brush_size(
    mut state: ResMut<PosterEditorState>,
    query: Query<(&Interaction, &BrushSizeButton), Changed<Interaction>>,
    mut all_buttons: Query<(&BrushSizeButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.brush_radius = btn.0;

        for (b, mut bg, mut border) in &mut all_buttons {
            if (b.0 - btn.0).abs() < 0.1 {
                *bg = BackgroundColor(ui_theme::BUTTON_SELECTED);
                *border = BorderColor::all(palette::VANILLA);
            } else {
                *bg = BackgroundColor(ui_theme::BUTTON_NORMAL);
                *border = BorderColor::all(ui_theme::BORDER_NORMAL);
            }
        }
    }
}

// --- Painting ---

pub fn handle_paint(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    canvas_query: Query<&GlobalTransform, With<PosterCanvas>>,
    mut state: ResMut<PosterEditorState>,
    mut images: ResMut<Assets<Image>>,
    mut undo_stack: ResMut<UndoStack<PosterAction>>,
) {
    if state.active_tool != PosterTool::Paint {
        return;
    }

    paint_or_erase(
        &mouse,
        &windows,
        &canvas_query,
        &mut state,
        &mut images,
        Some(&mut undo_stack),
        false,
    );
}

// --- Erasing ---

pub fn handle_erase(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    canvas_query: Query<&GlobalTransform, With<PosterCanvas>>,
    mut state: ResMut<PosterEditorState>,
    mut images: ResMut<Assets<Image>>,
) {
    if state.active_tool != PosterTool::Erase {
        return;
    }

    paint_or_erase(
        &mouse,
        &windows,
        &canvas_query,
        &mut state,
        &mut images,
        None,
        true,
    );
}

/// Shared logic for painting and erasing.
fn paint_or_erase(
    mouse: &ButtonInput<MouseButton>,
    windows: &Query<&Window>,
    canvas_query: &Query<&GlobalTransform, With<PosterCanvas>>,
    state: &mut PosterEditorState,
    images: &mut Assets<Image>,
    undo_stack: Option<&mut UndoStack<PosterAction>>,
    is_erase: bool,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(canvas_gt) = canvas_query.single() else {
        return;
    };

    let Some((local_x, local_y)) = cursor_to_canvas(cursor_pos, canvas_gt) else {
        // Cursor outside canvas — finalize any in-progress stroke
        finalize_stroke(state, undo_stack);
        return;
    };

    let color = if is_erase {
        [255, 255, 255, 255]
    } else {
        state.brush_color
    };
    let radius = state.brush_radius;

    if mouse.just_pressed(MouseButton::Left) {
        let stroke = PaintStroke {
            points: vec![[local_x, local_y]],
            color,
            radius,
        };

        if let Some(image) = images.get_mut(&state.canvas_handle) {
            let data = image.data.as_mut().unwrap();
            canvas::paint_circle(data, CANVAS_WIDTH, CANVAS_HEIGHT, local_x, local_y, radius, color);
        }

        state.current_stroke = Some(stroke);
    } else if mouse.pressed(MouseButton::Left)
        && let Some(ref mut stroke) = state.current_stroke
    {
        let prev = stroke.points.last().copied().unwrap_or([local_x, local_y]);
        stroke.points.push([local_x, local_y]);

        if let Some(image) = images.get_mut(&state.canvas_handle) {
            let data = image.data.as_mut().unwrap();
            let seg = PaintStroke {
                points: vec![prev, [local_x, local_y]],
                color,
                radius,
            };
            canvas::paint_stroke(data, CANVAS_WIDTH, CANVAS_HEIGHT, &seg);
        }
    }

    if mouse.just_released(MouseButton::Left) {
        finalize_stroke(state, undo_stack);
    }
}

/// Move current stroke into history and undo stack (if provided).
fn finalize_stroke(
    state: &mut PosterEditorState,
    undo_stack: Option<&mut UndoStack<PosterAction>>,
) {
    if let Some(stroke) = state.current_stroke.take()
        && let Some(undo) = undo_stack
    {
        state.strokes.push(stroke.clone());
        undo.push(PosterAction::Stroke(stroke));
    }
}

/// Convert window cursor position to canvas-local image coordinates.
/// Returns None if cursor is outside canvas bounds.
fn cursor_to_canvas(cursor_pos: Vec2, canvas_gt: &GlobalTransform) -> Option<(f32, f32)> {
    let canvas_center = canvas_gt.translation().truncate();
    let half_w = CANVAS_DISPLAY_WIDTH / 2.0;
    let half_h = CANVAS_DISPLAY_HEIGHT / 2.0;

    let local_x = cursor_pos.x - (canvas_center.x - half_w);
    let local_y = cursor_pos.y - (canvas_center.y - half_h);

    if local_x < 0.0 || local_y < 0.0 || local_x >= CANVAS_DISPLAY_WIDTH || local_y >= CANVAS_DISPLAY_HEIGHT {
        return None;
    }

    // Scale from display coords to image coords
    let img_x = local_x / CANVAS_DISPLAY_WIDTH * CANVAS_WIDTH as f32;
    let img_y = local_y / CANVAS_DISPLAY_HEIGHT * CANVAS_HEIGHT as f32;

    Some((img_x, img_y))
}

// --- Text placement ---

pub fn handle_text_placement(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    canvas_query: Query<(&GlobalTransform, Entity), With<CanvasContainer>>,
    mut state: ResMut<PosterEditorState>,
) {
    if state.active_tool != PosterTool::Text {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((container_gt, container_entity)) = canvas_query.single() else {
        return;
    };

    // Convert to container-local position
    let center = container_gt.translation().truncate();
    let half_w = CANVAS_DISPLAY_WIDTH / 2.0;
    let half_h = CANVAS_DISPLAY_HEIGHT / 2.0;
    let local_x = cursor_pos.x - (center.x - half_w);
    let local_y = cursor_pos.y - (center.y - half_h);

    if local_x < 0.0 || local_y < 0.0 || local_x >= CANVAS_DISPLAY_WIDTH || local_y >= CANVAS_DISPLAY_HEIGHT {
        return;
    }

    // Finalize any previous text
    state.editing_text = None;

    // Spawn a new text entity as child of the canvas container
    let text_entity = commands
        .spawn((
            PosterTextElement,
            Text::new(""),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::BLACK),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(local_x),
                top: Val::Px(local_y),
                ..default()
            },
        ))
        .id();

    commands.entity(container_entity).add_child(text_entity);
    state.editing_text = Some(text_entity);
}

// --- Text input ---

pub fn handle_text_input(
    mut state: ResMut<PosterEditorState>,
    mut events: MessageReader<KeyboardInput>,
    mut text_query: Query<&mut Text, With<PosterTextElement>>,
) {
    let Some(entity) = state.editing_text else {
        // Consume events so they don't pile up
        events.read().count();
        return;
    };

    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match &event.logical_key {
            Key::Enter | Key::Escape => {
                state.editing_text = None;
                return;
            }
            Key::Backspace => {
                if let Ok(mut text) = text_query.get_mut(entity) {
                    let s = text.0.clone();
                    if let Some((idx, _)) = s.char_indices().last() {
                        text.0 = s[..idx].to_string();
                    }
                }
            }
            Key::Space => {
                if let Ok(mut text) = text_query.get_mut(entity) {
                    text.0.push(' ');
                }
            }
            Key::Character(c) => {
                if let Ok(mut text) = text_query.get_mut(entity) {
                    for ch in c.chars() {
                        if ch.is_alphanumeric() || ch.is_ascii_punctuation() {
                            text.0.push(ch);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// --- Delete text ---

pub fn handle_delete_text(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<PosterEditorState>,
    mut commands: Commands,
    text_query: Query<Entity, With<PosterTextElement>>,
) {
    if state.editing_text.is_some() {
        return;
    }
    if !keyboard.just_pressed(KeyCode::Delete) {
        return;
    }

    // Delete the most recently spawned text element (last entity in iteration)
    if let Some(entity) = text_query.iter().last() {
        commands.entity(entity).despawn();
    }
}

// --- Undo / Redo ---

pub fn handle_undo_redo(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<PosterEditorState>,
    mut undo_stack: ResMut<UndoStack<PosterAction>>,
    mut images: ResMut<Assets<Image>>,
) {
    let ctrl =
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl {
        return;
    }

    let is_undo = keyboard.just_pressed(KeyCode::KeyZ);
    let is_redo = keyboard.just_pressed(KeyCode::KeyY);
    if !is_undo && !is_redo {
        return;
    }

    if is_undo {
        if let Some(action) = undo_stack.pop_undo() {
            // Remove last stroke from state
            state.strokes.pop();
            undo_stack.push_redo(action);
        }
    } else if let Some(action) = undo_stack.pop_redo() {
        // Re-add the stroke
        let PosterAction::Stroke(ref stroke) = action;
        state.strokes.push(stroke.clone());
        undo_stack.push_undo_only(action);
    }

    // Replay all strokes onto fresh canvas
    let new_data = canvas::replay_strokes(&state.strokes, CANVAS_WIDTH, CANVAS_HEIGHT);
    if let Some(image) = images.get_mut(&state.canvas_handle) {
        *image.data.as_mut().unwrap() = new_data;
    }
}

// --- Start Race ---

pub fn handle_start_race(
    query: Query<&Interaction, (Changed<Interaction>, With<PosterStartRaceButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Race);
        }
    }
}
