use bevy::prelude::*;

use crate::editor::EditorTab;
use crate::editor::course_editor::{EditorUI};
use crate::ui_theme;

use super::types::*;

pub fn handle_tab_switch(
    mut editor_ui: ResMut<EditorUI>,
    obstacle_tab: Query<&Interaction, (Changed<Interaction>, With<ObstacleTabButton>)>,
    props_tab: Query<&Interaction, (Changed<Interaction>, With<PropsTabButton>)>,
    mut obstacle_content: Query<
        &mut Node,
        (With<ObstaclePaletteContent>, Without<PropPaletteContent>),
    >,
    mut prop_content: Query<
        &mut Node,
        (With<PropPaletteContent>, Without<ObstaclePaletteContent>),
    >,
    mut obstacle_tab_bg: Query<
        &mut BackgroundColor,
        (With<ObstacleTabButton>, Without<PropsTabButton>),
    >,
    mut props_tab_bg: Query<
        &mut BackgroundColor,
        (With<PropsTabButton>, Without<ObstacleTabButton>),
    >,
) {
    let mut new_tab = None;

    for interaction in &obstacle_tab {
        if *interaction == Interaction::Pressed {
            new_tab = Some(EditorTab::Obstacles);
        }
    }
    for interaction in &props_tab {
        if *interaction == Interaction::Pressed {
            new_tab = Some(EditorTab::Props);
        }
    }

    let Some(tab) = new_tab else { return };
    if tab == editor_ui.active_tab {
        return;
    }
    editor_ui.active_tab = tab;

    let (obs_display, prop_display) = match tab {
        EditorTab::Obstacles => (Display::Flex, Display::None),
        EditorTab::Props => (Display::None, Display::Flex),
    };

    if let Ok(mut node) = obstacle_content.single_mut() {
        node.display = obs_display;
    }
    if let Ok(mut node) = prop_content.single_mut() {
        node.display = prop_display;
    }

    let (obs_bg, prop_bg) = match tab {
        EditorTab::Obstacles => (ui_theme::BUTTON_SELECTED, ui_theme::BUTTON_NORMAL),
        EditorTab::Props => (ui_theme::BUTTON_NORMAL, ui_theme::BUTTON_SELECTED),
    };
    if let Ok(mut bg) = obstacle_tab_bg.single_mut() {
        *bg = BackgroundColor(obs_bg);
    }
    if let Ok(mut bg) = props_tab_bg.single_mut() {
        *bg = BackgroundColor(prop_bg);
    }
}
