mod data;
mod discover;
mod left_panel;
mod load;
mod right_panel;
mod save_delete;
mod systems;
mod types;

pub use discover::discover_existing_courses;
pub use left_panel::build_course_editor_ui;
pub use load::{auto_load_pending_course, handle_load_button};
pub use save_delete::{
    handle_back_to_menu, handle_back_to_workshop, handle_cancel_delete,
    handle_clear_gate_orders_button, handle_confirm_delete, handle_delete_button,
    handle_gate_order_toggle, handle_new_course_button, handle_save_button,
};
pub use systems::{
    handle_button_hover, handle_camera_placement, handle_camera_primary_toggle,
    handle_name_field_focus, handle_name_text_input, handle_palette_selection,
    handle_prop_color_cycle, handle_prop_palette_selection, handle_tab_switch,
    handle_transform_mode_buttons, setup_camera_editor_meshes, setup_prop_editor_meshes,
    update_camera_primary_label, update_display_values, update_gate_count_display,
    update_prop_color_label, update_transform_mode_ui,
};
pub use types::*;
