use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::editor::undo::UndoStack;
use crate::palette;
use crate::states::AppState;
use crate::ui_theme;

use crate::dev_menu::color_picker_data::PALETTE_COLORS;

use super::canvas::{self, CANVAS_DISPLAY_HEIGHT, CANVAS_DISPLAY_WIDTH, CANVAS_HEIGHT, CANVAS_WIDTH};
use super::{
    BrushCursorPreview, BrushSizeButton, CanvasContainer, CursorBlinkTimer, PaintStroke,
    PosterAction, PosterColorCell, PosterEditorState, PosterStartRaceButton, PosterTextElement,
    PosterTool, ToolButtonMarker,
};

// --- Tool selection ---

pub fn handle_tool_selection(
    mut state: ResMut<PosterEditorState>,
    query: Query<(&Interaction, &ToolButtonMarker), Changed<Interaction>>,
    mut all_buttons: Query<(&ToolButtonMarker, &mut BackgroundColor, &mut BorderColor)>,
    mut text_query: Query<&mut Text, With<PosterTextElement>>,
) {
    for (interaction, marker) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Strip cursor from any active text before deselecting
        if let Some(entity) = state.editing_text {
            if let Ok(mut text) = text_query.get_mut(entity) {
                if text.0.ends_with('|') {
                    text.0.pop();
                }
            }
        }
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
    mut text_colors: Query<&mut TextColor, With<PosterTextElement>>,
) {
    for (interaction, cell) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let rgb = PALETTE_COLORS[cell.0].1;
        state.brush_color = [
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
            255,
        ];

        // Update active text element color
        if let Some(entity) = state.editing_text {
            if let Ok(mut tc) = text_colors.get_mut(entity) {
                tc.0 = Color::srgb(rgb[0], rgb[1], rgb[2]);
            }
        }

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
    container_query: Query<&RelativeCursorPosition, With<CanvasContainer>>,
    mut state: ResMut<PosterEditorState>,
    mut images: ResMut<Assets<Image>>,
    mut undo_stack: ResMut<UndoStack<PosterAction>>,
) {
    if state.active_tool != PosterTool::Paint {
        return;
    }

    paint_or_erase(
        &mouse,
        &container_query,
        &mut state,
        &mut images,
        Some(&mut undo_stack),
        false,
    );
}

// --- Erasing ---

pub fn handle_erase(
    mouse: Res<ButtonInput<MouseButton>>,
    container_query: Query<&RelativeCursorPosition, With<CanvasContainer>>,
    mut state: ResMut<PosterEditorState>,
    mut images: ResMut<Assets<Image>>,
) {
    if state.active_tool != PosterTool::Erase {
        return;
    }

    paint_or_erase(
        &mouse,
        &container_query,
        &mut state,
        &mut images,
        None,
        true,
    );
}

/// Shared logic for painting and erasing.
fn paint_or_erase(
    mouse: &ButtonInput<MouseButton>,
    container_query: &Query<&RelativeCursorPosition, With<CanvasContainer>>,
    state: &mut PosterEditorState,
    images: &mut Assets<Image>,
    undo_stack: Option<&mut UndoStack<PosterAction>>,
    is_erase: bool,
) {
    let Ok(rel_cursor) = container_query.single() else {
        return;
    };

    let Some((img_x, img_y)) = cursor_to_canvas_image(rel_cursor) else {
        // Cursor outside canvas — only finalize if mouse was released
        if mouse.just_released(MouseButton::Left) {
            finalize_stroke(state, undo_stack);
        }
        return;
    };

    let color = if is_erase {
        [255, 255, 255, 255]
    } else {
        state.brush_color
    };
    let radius = state.brush_radius;

    let start_new = mouse.just_pressed(MouseButton::Left)
        || (mouse.pressed(MouseButton::Left) && state.current_stroke.is_none());

    if start_new {
        let stroke = PaintStroke {
            points: vec![[img_x, img_y]],
            color,
            radius,
        };

        if let Some(image) = images.get_mut(&state.canvas_handle) {
            let data = image.data.as_mut().unwrap();
            canvas::paint_circle(data, CANVAS_WIDTH, CANVAS_HEIGHT, img_x, img_y, radius, color);
        }

        state.current_stroke = Some(stroke);
    } else if mouse.pressed(MouseButton::Left)
        && let Some(ref mut stroke) = state.current_stroke
    {
        let prev = stroke.points.last().copied().unwrap_or([img_x, img_y]);
        stroke.points.push([img_x, img_y]);

        if let Some(image) = images.get_mut(&state.canvas_handle) {
            let data = image.data.as_mut().unwrap();
            let seg = PaintStroke {
                points: vec![prev, [img_x, img_y]],
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

/// Convert `RelativeCursorPosition` to canvas image pixel coordinates.
/// Returns `None` if cursor is outside the canvas container.
fn cursor_to_canvas_image(rel: &RelativeCursorPosition) -> Option<(f32, f32)> {
    if !rel.cursor_over {
        return None;
    }
    // normalized: center=(0,0), bottom-right=(0.5,0.5)
    let norm = rel.normalized?;
    let img_x = (norm.x + 0.5) * CANVAS_WIDTH as f32;
    let img_y = (norm.y + 0.5) * CANVAS_HEIGHT as f32;
    Some((img_x, img_y))
}

// --- Text placement ---

pub fn handle_text_placement(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    canvas_query: Query<(&RelativeCursorPosition, Entity), With<CanvasContainer>>,
    mut state: ResMut<PosterEditorState>,
    text_interactions: Query<(Entity, &Interaction), (With<PosterTextElement>, Changed<Interaction>)>,
) {
    if state.active_tool != PosterTool::Text {
        return;
    }

    // Check if an existing text element was clicked (reselect it)
    for (entity, interaction) in &text_interactions {
        if *interaction == Interaction::Pressed {
            state.editing_text = Some(entity);
            return;
        }
    }

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((rel_cursor, container_entity)) = canvas_query.single() else {
        return;
    };

    if !rel_cursor.cursor_over {
        return;
    }
    let Some(norm) = rel_cursor.normalized else {
        return;
    };

    // Convert to container-local display coordinates
    let local_x = (norm.x + 0.5) * CANVAS_DISPLAY_WIDTH;
    let local_y = (norm.y + 0.5) * CANVAS_DISPLAY_HEIGHT;

    // Finalize any previous text
    state.editing_text = None;

    // Use brush color for text
    let [r, g, b, _] = state.brush_color;
    let text_color = Color::srgb_u8(r, g, b);

    // Spawn a new text entity as child of the canvas container
    // ZIndex(1) ensures text renders above the canvas ImageNode
    let text_entity = commands
        .spawn((
            PosterTextElement,
            Interaction::default(),
            Text::new(""),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(text_color),
            ZIndex(1),
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
    mut blink: ResMut<CursorBlinkTimer>,
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

        // Reset blink on any keypress so cursor stays visible while typing
        blink.visible = true;
        blink.timer.reset();

        match &event.logical_key {
            Key::Enter | Key::Escape => {
                // Strip cursor before finalizing
                if let Ok(mut text) = text_query.get_mut(entity) {
                    if text.0.ends_with('|') {
                        text.0.pop();
                    }
                }
                state.editing_text = None;
                return;
            }
            Key::Backspace => {
                if let Ok(mut text) = text_query.get_mut(entity) {
                    // Strip cursor if present before deleting real content
                    let had_cursor = text.0.ends_with('|');
                    if had_cursor {
                        text.0.pop();
                    }
                    // Delete last real character
                    let s = text.0.clone();
                    if let Some((idx, _)) = s.char_indices().last() {
                        text.0 = s[..idx].to_string();
                    }
                    // Re-add cursor
                    if had_cursor {
                        text.0.push('|');
                    }
                }
            }
            Key::Space => {
                if let Ok(mut text) = text_query.get_mut(entity) {
                    insert_before_cursor(&mut text.0, ' ');
                }
            }
            Key::Character(c) => {
                if let Ok(mut text) = text_query.get_mut(entity) {
                    for ch in c.chars() {
                        if ch.is_alphanumeric() || ch.is_ascii_punctuation() {
                            insert_before_cursor(&mut text.0, ch);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Insert a character before a trailing "|" cursor, or at the end if no cursor.
fn insert_before_cursor(s: &mut String, ch: char) {
    if s.ends_with('|') {
        s.pop();
        s.push(ch);
        s.push('|');
    } else {
        s.push(ch);
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

// --- Brush cursor preview ---

const DISPLAY_SCALE: f32 = CANVAS_DISPLAY_WIDTH / CANVAS_WIDTH as f32;

pub fn update_brush_cursor(
    state: Res<PosterEditorState>,
    container_query: Query<&RelativeCursorPosition, With<CanvasContainer>>,
    mut cursor_query: Query<
        (&mut Node, &mut Visibility, &mut BorderColor),
        With<BrushCursorPreview>,
    >,
) {
    let Ok(rel_cursor) = container_query.single() else {
        return;
    };
    let Ok((mut node, mut vis, mut border)) = cursor_query.single_mut() else {
        return;
    };

    let show = matches!(state.active_tool, PosterTool::Paint | PosterTool::Erase)
        && rel_cursor.cursor_over;

    if !show {
        *vis = Visibility::Hidden;
        return;
    }

    let Some(norm) = rel_cursor.normalized else {
        *vis = Visibility::Hidden;
        return;
    };

    *vis = Visibility::Inherited;

    let diameter = state.brush_radius * 2.0 * DISPLAY_SCALE;
    let local_x = (norm.x + 0.5) * CANVAS_DISPLAY_WIDTH;
    let local_y = (norm.y + 0.5) * CANVAS_DISPLAY_HEIGHT;

    node.width = Val::Px(diameter);
    node.height = Val::Px(diameter);
    node.left = Val::Px(local_x - diameter * 0.5);
    node.top = Val::Px(local_y - diameter * 0.5);

    // Color: brush color for paint, gray for erase
    let color = if state.active_tool == PosterTool::Erase {
        Color::srgba(0.4, 0.4, 0.4, 0.8)
    } else {
        let [r, g, b, _] = state.brush_color;
        Color::srgba_u8(r, g, b, 200)
    };
    *border = BorderColor::all(color);
}

// --- Text cursor blink ---

/// Manages a blinking "|" cursor by appending/stripping it directly in the
/// text string. This avoids child-entity lifecycle issues (deferred despawns
/// causing ghost artifacts) that occur with the TextSpan approach.
pub fn update_text_cursor_blink(
    time: Res<Time>,
    state: Res<PosterEditorState>,
    mut blink: ResMut<CursorBlinkTimer>,
    mut text_query: Query<&mut Text, With<PosterTextElement>>,
) {
    blink.timer.tick(time.delta());

    if blink.timer.just_finished() {
        blink.visible = !blink.visible;
    }

    // Strip cursor from all text elements, then re-add to the active one
    for mut text in &mut text_query {
        if text.0.ends_with('|') {
            text.0.pop();
        }
    }

    let Some(editing) = state.editing_text else {
        return;
    };

    if blink.visible {
        if let Ok(mut text) = text_query.get_mut(editing) {
            text.0.push('|');
        }
    }
}
