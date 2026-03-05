pub(crate) mod canvas;
mod tools;
mod ui;

use bevy::prelude::*;
use bevy::time::Timer;

use crate::states::HypeMode;

pub struct PosterEditorPlugin;

impl Plugin for PosterEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(HypeMode::PosterEditor), ui::setup_poster_editor)
            .add_systems(OnExit(HypeMode::PosterEditor), cleanup_poster_editor)
            .add_systems(
                Update,
                (
                    tools::handle_tool_selection,
                    tools::handle_color_selection,
                    tools::handle_brush_size,
                    tools::handle_paint,
                    tools::handle_erase,
                    tools::handle_text_placement,
                    tools::handle_text_input,
                    tools::handle_delete_text,
                    tools::handle_undo_redo,
                    tools::handle_start_race,
                    tools::update_brush_cursor,
                    tools::update_text_cursor_blink,
                )
                    .run_if(in_state(HypeMode::PosterEditor)),
            );
    }
}

// --- Tool enum ---

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum PosterTool {
    #[default]
    Paint,
    Text,
    Erase,
}

// --- Stroke data ---

#[derive(Clone)]
pub struct PaintStroke {
    pub points: Vec<[f32; 2]>,
    pub color: [u8; 4],
    pub radius: f32,
}

// --- Undo action ---

#[derive(Clone)]
pub enum PosterAction {
    Stroke(PaintStroke),
}

// --- Editor state ---

#[derive(Resource)]
pub struct PosterEditorState {
    pub active_tool: PosterTool,
    pub brush_color: [u8; 4],
    pub brush_radius: f32,
    pub canvas_handle: Handle<Image>,
    pub strokes: Vec<PaintStroke>,
    pub current_stroke: Option<PaintStroke>,
    pub editing_text: Option<Entity>,
}

// --- Component markers ---

#[derive(Component)]
pub struct PosterCanvas;

#[derive(Component)]
pub struct CanvasContainer;

#[derive(Component)]
pub struct PaintToolButton;

#[derive(Component)]
pub struct TextToolButton;

#[derive(Component)]
pub struct EraseToolButton;

#[derive(Component)]
pub struct BrushSizeButton(pub f32);

#[derive(Component)]
pub struct PosterColorCell(pub usize);

#[derive(Component)]
pub struct PosterStartRaceButton;

#[derive(Component)]
pub struct PosterTextElement;

#[derive(Component)]
pub struct ToolButtonMarker(pub PosterTool);

#[derive(Component)]
pub struct BrushCursorPreview;

#[derive(Component)]
pub struct TextCursorBar;

#[derive(Resource)]
pub struct CursorBlinkTimer {
    pub timer: Timer,
    pub visible: bool,
}

fn cleanup_poster_editor(mut commands: Commands) {
    commands.remove_resource::<PosterEditorState>();
    commands.remove_resource::<crate::editor::undo::UndoStack<PosterAction>>();
    commands.remove_resource::<CursorBlinkTimer>();
}
