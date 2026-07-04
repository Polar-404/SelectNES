use egui::Ui;
use mlua::prelude::*;

pub fn render_lua_draws(lua: &Lua, ui: &mut Ui, image_rect: egui::Rect, hide_overscan: bool) {
    let globals = lua.globals();

    if let Ok(commands) = globals.get::<mlua::Table>("_draw_commands") {
        let width = if hide_overscan { 240.0 } else { 256.0 };
        let height = if hide_overscan { 224.0 } else { 240.0 };

        let project = |nes_x: i32, nes_y: i32| -> egui::Pos2 {
            let mut x = nes_x as f32;
            let mut y = nes_y as f32;

            if hide_overscan {
                x -= 8.0;
                y -= 8.0;
            }

            egui::pos2(
                image_rect.min.x + (x / width) * image_rect.width(),
                image_rect.min.y + (y / height) * image_rect.height(),
            )
        };

        let parse_color = |hex: String| -> egui::Color32 {
            if hex.starts_with('#') && hex.len() == 7 {
                if let Ok(r) = u8::from_str_radix(&hex[1..3], 16) {
                    if let Ok(g) = u8::from_str_radix(&hex[3..5], 16) {
                        if let Ok(b) = u8::from_str_radix(&hex[5..7], 16) {
                            return egui::Color32::from_rgb(r, g, b);
                        }
                    }
                }
            }
            egui::Color32::WHITE
        };

        let pairs = commands.pairs::<i32, mlua::Table>();
        for pair in pairs.flatten() {
            let cmd = pair.1;
            if let Ok(cmd_type) = cmd.get::<String>("type") {
                match cmd_type.as_str() {
                    "box" => {
                        let x1 = cmd.get::<i32>("x1").unwrap_or(0);
                        let x2 = cmd.get::<i32>("x2").unwrap_or(0);
                        let y1 = cmd.get::<i32>("y1").unwrap_or(0);
                        let y2 = cmd.get::<i32>("y2").unwrap_or(0);

                        let color_str = cmd.get::<String>("color").unwrap_or_default();

                        let p1 = project(x1, y1);
                        let p2 = project(x2, y2);
                        let rect = egui::Rect::from_two_pos(p1, p2);

                        ui
                        .painter()
                        .rect_stroke(
                            rect, 
                            0.0, 
                            egui::Stroke::new(1.2, parse_color(color_str)), 
                            egui::StrokeKind::Middle
                        );
                    }
                    "text" => {
                        let x = cmd.get::<i32>("x").unwrap_or(0);
                        let y = cmd.get::<i32>("y").unwrap_or(0);
                        let text_str = cmd.get::<String>("text").unwrap_or_default();

                        let p = project(x, y);

                        ui.painter().text(
                            p, 
                            egui::Align2::LEFT_TOP, 
                            text_str, 
                            egui::FontId::monospace(11.0), 
                            egui::Color32::GREEN
                        );
                    }
                    _ => {}
                }
            }
        }
        let _ = globals.set("_draw_commands", lua.create_table().unwrap());
    }
}