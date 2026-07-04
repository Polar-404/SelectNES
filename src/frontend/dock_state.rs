use egui_dock::{TabViewer};
use mlua::Lua;

use crate::engine::{
    config::EmulatorConfig,
    instance::EmulatorInstance,
    terminal::struct_terminal::LuaScript,
};

use crate::frontend::panels::{
    app_terminal::render_terminal,
    settings_panel::render_settings,
    cpu_viewer::render_cpu_viewer,
    memory_viewer::MemViewer,
    ppu_viewer::*,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Emulator,   
    CpuViewer,
    PpuViewer,  
    MemoryEditor,
    ApuWaveform,
    Settings,
    Terminal,
}

pub struct NesTabViewer<'a> {
    pub nes_texture: Option<egui::TextureId>,
    pub emulator: Option<&'a std::sync::Arc<std::sync::Mutex<EmulatorInstance>>>,
    pub config: &'a mut EmulatorConfig,

    pub lua: &'a Lua,
    pub active_scripts: &'a mut Vec<LuaScript>,

    pub pattern_viewer: &'a mut pattern_viewer::PatternTableViewer,
    pub nametable_viewer: &'a mut palette_viewer::PaletteViewer,
}

impl TabViewer for NesTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        match tab {
            Tab::Emulator    => "NES".into(),
            Tab::CpuViewer   => "CPU".into(),
            Tab::PpuViewer   => "PPU".into(),
            Tab::MemoryEditor => "Memory".into(),
            Tab::ApuWaveform => "APU".into(),
            Tab::Settings => "Settings".into(),
            Tab::Terminal => "Terminal".into(),
        }
    }
    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Emulator => {
                if let Some(tex_id) = self.nes_texture {
                    let available = ui.available_size();

                    let (width, height) = if self.config.hide_overscan {
                        (240.0f32, 224.0f32)
                    } else {
                        (256.0f32, 240.0f32)
                    };

                    let scale = (available.x / width).min(available.y / height);
                    let size = egui::vec2(width * scale, height * scale);

                    let uv = if self.config.hide_overscan {
                        egui::Rect::from_min_max(
                            egui::pos2(8.0 / 256.0,   8.0 / 240.0),
                            egui::pos2(248.0 / 256.0, 232.0 / 240.0),
                        )
                    } else {
                        egui::Rect::from_min_max(
                            egui::pos2(0.0, 0.0),
                            egui::pos2(1.0, 1.0)
                        )
                    };

                    let (rect, _response) = ui.allocate_exact_size(available, egui::Sense::hover());
                    
                    let offset = (available - size) / 2.0;
                    let image_rect = egui::Rect::from_min_size(rect.min + offset, size);

                    ui.painter().image(
                        tex_id,
                        image_rect,
                        uv,
                        egui::Color32::WHITE
                    );

                    crate::engine::terminal::lua::auxiliary_functions::mouse_pos::get_mouse_pos(
                        self.lua,
                        ui.ctx().input(|i| i.pointer.hover_pos()),
                        image_rect,
                        self.config.hide_overscan,
                        ui.input(|i| i.pointer.primary_down())
                    );

                    crate::engine::terminal::lua::auxiliary_functions::draw_render::render_lua_draws(
                        self.lua, 
                        ui, 
                        image_rect, 
                        self.config.hide_overscan
                    );

                    //running lua script stack on_frame() functions
                    for script in self.active_scripts.iter() {
                        if let Some(ref key) = script.on_frame_key {
                            if let Ok(on_frame_fn) = self.lua.registry_value::<mlua::Function>(key) {
                                if let Err(e) = on_frame_fn.call::<()>(()) {
                                    crate::engine::terminal::struct_terminal::print_logs(
                                        crate::engine::terminal::struct_terminal::LogType::Warning,
                                        format!("Script Error [{}]: {}", script.name, e)
                                    );
                                }
                            }
                        }

                    }
                    
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Waiting to start video system...");
                    });
                }
            }
            Tab::CpuViewer => {
                if let Some(emu_arc) = self.emulator {
                    if let Ok(emu) = emu_arc.lock() {
                        render_cpu_viewer(ui, &emu);
                    }
                } else {
                    ui.label("No Game loaded, insert a ROM to view the CPU");
                }
                
            }
            Tab::PpuViewer => {
                if let Some(emu_arc) = self.emulator {
                    if let Ok(emu) = emu_arc.lock() {
                        self.pattern_viewer.render(ui, &emu);
                        self.nametable_viewer.render(ui, &emu);
                    }
                } else {
                    ui.label("No loaded ROM");
                }
            }
            Tab::MemoryEditor => {
                if let Some(emu_arc) = self.emulator {
                    if let Ok(emu) = emu_arc.lock() {
                        MemViewer::render_memory_viewer(ui, &emu, 0x00, 0x07FF);
                    }
                } else {
                    ui.label("No loaded ROM");
                }
                
            }
            Tab::ApuWaveform => {
                // waveform plot
            }
            Tab::Settings => {
                render_settings(self.config, ui);
            }
            Tab::Terminal => {
                render_terminal(self.config, ui, self.lua, self.active_scripts);
            }
        }
    }
}