mod camera_interaction;
mod data;
mod display_updates;
mod gate_color;
mod left_panel;
mod load;
mod obstacle_interaction;
mod prop_interaction;
mod right_panel;
mod save_delete;
mod types;

pub use left_panel::build_course_editor_ui;
pub(crate) use data::build_course_data;
pub use load::auto_load_pending_course;
pub(crate) use load::load_course_into_editor;
pub use save_delete::{
    PendingRaceTransition, PendingThumbnailSave,
    check_pending_race_transition,
    handle_back_to_menu, handle_clear_gate_orders_button,
    handle_gate_order_toggle, handle_save_button, handle_start_race,
};
pub use obstacle_interaction::{handle_palette_selection, handle_tab_switch};
pub use prop_interaction::{
    handle_prop_color_cycle, handle_prop_palette_selection, setup_prop_editor_meshes,
    update_prop_color_label,
};
pub use camera_interaction::{
    setup_camera_editor_meshes, spawn_gate_camera,
};
pub use gate_color::{
    handle_gate_color_click, handle_gate_color_default, update_gate_color_label,
};
pub use display_updates::{
    handle_button_hover,
    handle_transform_mode_buttons, update_display_values, update_gate_count_display,
    update_money_display, update_transform_mode_ui,
};
pub use types::*;
