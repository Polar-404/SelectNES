use mlua::prelude::*;

pub fn get_mouse_pos(
    lua: &Lua,
    pointer_pos: Option<egui::Pos2>, 
    image_rect: egui::Rect, 
    hide_overscan: bool,
    is_clicking: bool
) {
    if let Some(pos) = pointer_pos {
        if image_rect.contains(pos) {
            let width = if hide_overscan { 240.0 } else { 256.0 };
            let height = if hide_overscan { 224.0 } else { 240.0 };

            let normalized_x = (pos.x - image_rect.min.x) / image_rect.width();
            let normalized_y = (pos.y - image_rect.min.y) / image_rect.height();

            let mut nes_x = (normalized_x * width) as i32;
            let mut nes_y = (normalized_y * height) as i32;

            if hide_overscan {
                nes_x += 8;
                nes_y += 8;
            }

            let globals = lua.globals();
            let inpt_table: mlua::Table = match globals.get("inpt") {
                Ok(table) => table,
                Err(_) => {
                    let table = lua.create_table().unwrap();
                    let _ = globals.set("inpt", table.clone());
                    table
                }
            };

            let was_clicking: bool = inpt_table.get("_was_clicking").unwrap_or(false);
            let just_clicked = is_clicking && !was_clicking;
            let _ = inpt_table.set("_was_clicking", is_clicking);
            let _ = inpt_table.set("just_clicked", just_clicked);

            let _ = inpt_table.set("xmouse", nes_x);
            let _ = inpt_table.set("ymouse", nes_y);
            let _ = inpt_table.set("leftclick", is_clicking);

        }
    }
}