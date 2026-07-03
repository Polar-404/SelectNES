use std::sync::atomic::Ordering;

use crate::engine::{config::EmulatorConfig, terminal::print_terminal::{LogType, TERMINAL, print_logs}};
use egui::{Color32, RichText, ScrollArea, Ui, TextEdit, Key};
use mlua::Lua;


pub fn render_terminal(settings: &mut EmulatorConfig, ui: &mut Ui, lua: &Lua) {
    let (show_info, show_warning, show_debug, show_code) = &mut settings.terminal_types;

    ui.horizontal(|ui| {
        if ui.checkbox(show_info, "Info").changed() {
            crate::engine::terminal::print_terminal::LOG_INFO_ENABLED.store(*show_info, Ordering::Relaxed);
        }
        if ui.checkbox(show_warning, "Warning").changed() {
            crate::engine::terminal::print_terminal::LOG_WARNING_ENABLED.store(*show_warning, Ordering::Relaxed);
        }
        if ui.checkbox(show_debug, "Debug").changed() {
            crate::engine::terminal::print_terminal::LOG_DEBUG_ENABLED.store(*show_debug, Ordering::Relaxed);
        }

        if ui.button("Clear").clicked() {
            if let Ok(mut logs) = TERMINAL.lock() {
                logs.clear();
            }
        }
    });

    ui.separator();

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
                        LogType::Info => &show_info,
                        LogType::Warning => &show_warning,
                        LogType::Debug => &show_debug,
                        LogType::Code => &show_code,

                    };

                    if **should_show {
                        let (color, txt_type) = match log.log_type {
                            LogType::Info => (Color32::WHITE, "[Info]"),
                            LogType::Warning => (Color32::YELLOW, "[Warning]"),
                            LogType::Debug => (Color32::LIGHT_GRAY, "[Debug]"),
                            LogType::Code => (Color32::LIGHT_GREEN, "[Code]")
                        };

                        ui.label(RichText::new(format!("{} {}", txt_type, &log.log_msg)).color(color));
                    }
                }
            }
        }
    );

    ui.separator();

    // -- COMMAND INPUT --

    ui.horizontal(|ui| {
        ui.label(RichText::new(">").color(Color32::GREEN).strong());

        let response = ui.add(
        TextEdit::singleline(&mut settings.terminal_input)
                .desired_width(ui.available_width() - 45.0)
        );

        if ui.button("📁").on_hover_text("Import Lua script file").clicked() {
            if let Some(path) = rfd::FileDialog::new()
            .add_filter("Lua Script", &["lua"])
            .pick_file() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                        print_logs(LogType::Info, format!("Loading script... {}", file_name));

                        let script_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let current_file = file_name.clone();

                        lua.set_hook(
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

                        // Executa o script principal e o on_init em sequência
                        let run_result = lua.load(&content).set_name(&file_name).exec().and_then(|_| {
                            if let Ok(on_init) = lua.globals().get::<mlua::Function>("on_init") {
                                on_init.call::<()>(())?;
                            }
                            Ok(())
                        });

                        // Remove o hook para não afetar os comandos normais do terminal
                        lua.remove_hook();
                        
                        if let Err(err) = run_result {
                            print_logs(LogType::Warning, format!("Lua Error: {}", err));
                        } else {
                            print_logs(LogType::Info, "Script ended successfully");
                        }
                    }
                    Err(err) => {
                        print_logs(LogType::Warning, format!("Erro ao ler arquivo: {}", err));
                    }
                }
            }
        }

        if response.lost_focus() && ui.input(|i|  i.key_pressed(Key::Enter)) {
            let command = settings.terminal_input.trim();
            if !command.is_empty() {
                print_logs(LogType::Code, format!("> {}", command));

                match lua.load(command).eval::<()>() {
                    Ok(_) => {}
                    Err(err) => {
                        print_logs(LogType::Warning, format!("Lua Error: {}", err));
                    }
                }

                settings.terminal_input.clear();

                response.request_focus();
            }
        }
    });
}
