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
                    tools::handle_text_drag,
                    tools::handle_text_input,
                    tools::handle_delete_text,
                    tools::handle_undo_redo,
                    tools::handle_start_race,
                    tools::update_brush_cursor,
                    tools::handle_text_size,
                    tools::handle_text_font,
                    tools::update_brush_panel_visibility,
                    tools::update_text_panel_visibility,
                    tools::update_text_cursor_blink,
                    tools::handle_poster_count,
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

/// Sentinel value for `brush_radius` indicating flood-fill mode.
pub const BRUSH_FILL: f32 = -1.0;

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
    Fill { x: u32, y: u32, color: [u8; 4] },
}

// --- Editor state ---

#[derive(Resource)]
pub struct PosterEditorState {
    pub active_tool: PosterTool,
    pub brush_color: [u8; 4],
    pub brush_radius: f32,
    pub canvas_handle: Handle<Image>,
    pub actions: Vec<PosterAction>,
    pub current_stroke: Option<PaintStroke>,
    pub editing_text: Option<Entity>,
    pub text_size: f32,
    pub text_font_index: usize,
    /// When true, ignore mouse input until the button is released (prevents the
    /// click that opened the editor from painting on the canvas).
    pub skip_initial_click: bool,
}

/// Font entries available in the poster editor: (display name, asset path).
pub const POSTER_FONTS: &[(&str, &str)] = &[
    ("Asimovian", "fonts/Asimovian/Asimovian-Regular.ttf"),
    ("Megrim", "fonts/Megrim/Megrim-Regular.ttf"),
    ("Syne Mono", "fonts/Syne_Mono/SyneMono-Regular.ttf"),
];

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
pub struct BrushSizePanel;

#[derive(Component)]
pub struct TextSizeButton(pub f32);

#[derive(Component)]
pub struct TextSizePanel;

#[derive(Component)]
pub struct TextFontButton(pub usize);

#[derive(Component)]
pub struct TextFontPanel;

#[derive(Resource)]
pub struct CursorBlinkTimer {
    pub timer: Timer,
    pub visible: bool,
}

/// Poster print order: count in multiples of 25, $5 per 25.
#[derive(Resource, Default)]
pub struct PosterOrder {
    pub count: u32,
}

#[derive(Component)]
pub struct PosterCountText;

#[derive(Component)]
pub struct PosterCountUpButton;

#[derive(Component)]
pub struct PosterCountDownButton;

/// Tracks where the poster editor was opened from, so DONE returns to the right place.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosterEditorOrigin {
    #[default]
    CampaignSelector,
    Menu,
}

/// Persisted canvas data so the poster can be re-opened after leaving the editor.
#[derive(Resource)]
pub struct SavedPosterData {
    pub actions: Vec<PosterAction>,
    pub image_data: Vec<u8>,
}

/// Tracks an in-progress text drag operation.
#[derive(Resource, Default)]
pub struct TextDragState {
    /// The text entity being dragged.
    pub dragging: Option<Entity>,
    /// Cursor position (in canvas-container-relative pixels) when drag started.
    pub start_cursor: [f32; 2],
    /// Original `left`/`top` of the text node when drag started.
    pub start_pos: [f32; 2],
    /// Whether the cursor has moved far enough to count as a drag (vs a click).
    pub moved: bool,
}

fn cleanup_poster_editor(
    mut commands: Commands,
    state: Option<Res<PosterEditorState>>,
    images: Res<Assets<Image>>,
) {
    if let Some(state) = state
        && let Some(image) = images.get(&state.canvas_handle)
        && let Some(data) = image.data.as_ref()
    {
        commands.insert_resource(SavedPosterData {
            actions: state.actions.clone(),
            image_data: data.clone(),
        });
    }
    commands.remove_resource::<PosterEditorState>();
    commands.remove_resource::<crate::editor::undo::UndoStack<PosterAction>>();
    commands.remove_resource::<CursorBlinkTimer>();
    commands.remove_resource::<TextDragState>();
}
