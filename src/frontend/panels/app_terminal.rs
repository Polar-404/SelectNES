use std::sync::atomic::Ordering;

use crate::engine::{config::EmulatorConfig, terminal::print_terminal::{LogType, TERMINAL}};
use egui::{Color32, RichText, ScrollArea, Ui};




pub fn render_terminal(settings: &mut EmulatorConfig, ui: &mut Ui) {
    let (show_info, show_warning, show_debug) = &mut settings.terminal_types;

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

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if let Ok(logs) = TERMINAL.lock() {
                for log in logs.iter() {
                    
                    let should_show = match log.log_type {
                        LogType::Info => &show_info,
                        LogType::Warning => &show_warning,
                        LogType::Debug => &show_debug,
                    };

                    if **should_show {
                        let (color, txt_type) = match log.log_type {
                            LogType::Info => (Color32::WHITE, "[Info]"),
                            LogType::Warning => (Color32::YELLOW, "[Warning]"),
                            LogType::Debug => (Color32::LIGHT_GRAY, "[Debug]"),
                        };

                        ui.label(RichText::new(format!("{} {}", txt_type, &log.log_msg)).color(color));
                    }
                }
            }
        }
    );
}
