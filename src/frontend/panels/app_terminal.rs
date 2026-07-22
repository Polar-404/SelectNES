use std::sync::atomic::Ordering;
use crate::engine::{
    config::EmulatorConfig, 
    terminal::struct_terminal::{LogType, LuaScript, TERMINAL, print_logs}
};
use egui::{Color32, RichText, ScrollArea, Ui, TextEdit, Key, Context};
use mlua::Lua;

pub fn render_terminal(settings: &mut EmulatorConfig, ui: &mut Ui, lua: &Lua, active_scripts: &mut Vec<LuaScript>) {

    let manager_id = ui.id().with("lua_script_manager_open");
    let mut show_manager = ui.ctx().data(|d| d.get_temp::<bool>(manager_id).unwrap_or(false));

    render_top_bar(ui, settings);
    ui.separator();

    render_log_area(ui, settings);
    ui.separator();

    render_input_bar(ui, settings, lua, &mut show_manager);

    if show_manager {
        render_script_manager_window(ui.ctx(), lua, active_scripts, &mut show_manager);
    }

    ui.ctx().data_mut(|d| d.insert_temp(manager_id, show_manager));
}

/// renders the upper buttons for filtering and clearing the terminal
fn render_top_bar(ui: &mut Ui, settings: &mut EmulatorConfig) {
    let (show_info, show_warning, show_debug, show_code) = &mut settings.terminal_types;

    ui.horizontal(|ui| {
        if ui.checkbox(show_info, "Info").changed() {
            crate::engine::terminal::struct_terminal::LOG_INFO_ENABLED.store(*show_info, Ordering::Relaxed);
        }
        if ui.checkbox(show_warning, "Warning").changed() {
            crate::engine::terminal::struct_terminal::LOG_WARNING_ENABLED.store(*show_warning, Ordering::Relaxed);
        }
        if ui.checkbox(show_code, "Code").changed() {
            crate::engine::terminal::struct_terminal::LOG_CODE_ENABLED.store(*show_code, Ordering::Relaxed);
        }
        if ui.checkbox(show_debug, "Debug").changed() {
            crate::engine::terminal::struct_terminal::LOG_DEBUG_ENABLED.store(*show_debug, Ordering::Relaxed);
        }


        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear").clicked() {
                if let Ok(mut logs) = TERMINAL.lock() {
                    logs.clear();
                }
            }
        });
    });
}

/// renders the history of log message area
fn render_log_area(ui: &mut Ui, settings: &EmulatorConfig) {
    let (show_info, show_warning, show_debug, show_code) = settings.terminal_types;
    
    let spacing = ui.spacing().item_spacing.y;
    let input_row_height = ui.text_style_height(&egui::TextStyle::Body) + spacing * 3.0 + 10.0;
    let max_scroll_height = (ui.available_height() - input_row_height).max(0.0);

    ScrollArea::vertical()
        .max_height(max_scroll_height)
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if let Ok(logs) = TERMINAL.lock() {
                for log in logs.iter() {
                    let should_show = match log.log_type {
                        LogType::Info => show_info,
                        LogType::Warning => show_warning,
                        LogType::Debug => show_debug,
                        LogType::Code => show_code,
                    };

                    if should_show {
                        let (color, txt_type) = match log.log_type {
                            LogType::Info => (Color32::WHITE, "[Info]"),
                            LogType::Warning => (Color32::YELLOW, "[Warning]"),
                            LogType::Debug => (Color32::LIGHT_GRAY, "[Debug]"),
                            LogType::Code => (Color32::LIGHT_GREEN, "[Code]")
                        };

                        ui.add(
                            egui::Label::new(RichText::new(format!("{} {}", txt_type, &log.log_msg)).color(color))
                                .selectable(true)
                        );
                    }
                }
            }
        });
}

/// renders the terminal command line
fn render_input_bar(ui: &mut Ui, settings: &mut EmulatorConfig, lua: &Lua, show_manager: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(">").color(Color32::GREEN).strong());

        let response = ui.add(
            TextEdit::singleline(&mut settings.terminal_input)
                .desired_width(ui.available_width() - 45.0)
        );
        
        if ui.button("📁").on_hover_text("Gerenciar scripts Lua carregados").clicked() {
            *show_manager = !*show_manager;
        }

        if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let command = settings.terminal_input.trim();
            if !command.is_empty() {
                print_logs(LogType::Code, format!("> {}", command));

                if let Err(err) = lua.load(command).eval::<()>() {
                    print_logs(LogType::Warning, format!("Lua Error: {}", err));
                }

                settings.terminal_input.clear();
                response.request_focus();
            }
        }
    });
}

fn render_script_manager_window(ctx: &Context, lua: &Lua, active_scripts: &mut Vec<LuaScript>, open: &mut bool) {
    egui::Window::new("Lua Script Manager")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER) 
        .default_pos(ctx.screen_rect().center()) 
        .default_size([320.0, 240.0])
        .collapsible(false)
        .vscroll(true)
        .show(ctx, |ui| {

            for (_, font_id) in ui.style_mut().text_styles.iter_mut() {
                font_id.size += 2.0;
            }

            ui.label(RichText::new("Current Scripts:").strong());
            ui.allocate_space(egui::vec2(0.0, 4.0));

            if active_scripts.is_empty() {
                ui.weak("No Scripts running currently...");
            }

            active_scripts.retain_mut(|script| {
                let mut manter = true;
                ui.horizontal(|ui| {
                    if ui.button("❌").on_hover_text("Stop execution").clicked() {
                        manter = false;
                        if let Some(key) = script.on_frame_key.take() {
                            let _ = lua.remove_registry_value(key);
                        }
                    }
                    ui.label(&script.name).highlight();
                });
                manter
            });

            ui.allocate_space(ui.available_size() - egui::vec2(0.0, 32.0));
            ui.separator();

            if ui.button("➕ Import New Script").clicked() {
                execute_script_import_dialog(lua, active_scripts);
            }
        });
}

///Launches the native file explorer and compiles the script for concurrent registration.
fn execute_script_import_dialog(lua: &Lua, active_scripts: &mut Vec<LuaScript>) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Lua Script", &["lua"])
        .pick_file() 
    {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                print_logs(LogType::Info, format!("Loading script: {}", file_name));

                let script_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                let current_file = file_name.clone();

                let _ = lua.set_hook(
                    mlua::HookTriggers {
                        every_line: true,
                        ..Default::default()
                    },
                    move |_, debug| {
                        let line_num = debug.curr_line();
                        if line_num > 0 {
                            let idx = (line_num - 1) as usize;
                            if idx < script_lines.len() {
                                let line_code = script_lines[idx].trim();
                                if !line_code.is_empty() && !line_code.starts_with("--") {
                                    print_logs(
                                        LogType::Code, 
                                        format!("  -> [{}:{}] {}", current_file, line_num, line_code)
                                    );
                                }
                            }
                        }
                        Ok(mlua::VmState::Continue)
                    }
                );

                let path_clone = path.clone();
                let run_result = lua.load(&content).set_name(&file_name).exec().and_then(|_| {
                    let globals = lua.globals();
                    
                    if let Ok(on_init) = globals.get::<mlua::Function>("on_init") {
                        on_init.call::<()>(())?;
                    }

                    if let Ok(on_frame_fn) = globals.get::<mlua::Function>("on_frame") {
                        if let Ok(key) = lua.create_registry_value(on_frame_fn) {
                            active_scripts.push(LuaScript {
                                name: file_name.clone(),
                                path: path_clone,
                                on_frame_key: Some(key),
                            });
                            print_logs(LogType::Info, format!("Script '{}' successfully injected.", file_name));
                        }
                    }

                    let _ = globals.set("on_frame", mlua::Value::Nil);
                    Ok(())
                });

                lua.remove_hook();
                
                if let Err(err) = run_result {
                    print_logs(LogType::Warning, format!("Lua Error: {}", err));
                }
            }
            Err(err) => {
                print_logs(LogType::Warning, format!("Error while reading file: {}", err));
            }
        }
    }
}