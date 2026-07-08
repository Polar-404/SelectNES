use egui_dock::{DockArea, DockState, Style};
use egui_glow::EguiGlow;
use glow::HasContext;

use mlua::Lua;
use winit::{
    application::ApplicationHandler, 
    event::{ElementState, WindowEvent}, 
    event_loop::ActiveEventLoop, 
    keyboard::{KeyCode, PhysicalKey}, 
    window::{Window, WindowId}
};

use crate::{
    apu::audio::AudioOutput, 
    engine::{
        config::EmulatorConfig, 
        terminal::struct_terminal::*, 
        input::*, 
        instance::EmulatorInstance
    }, 
    frontend::{
        dock_state::{NesTabViewer, Tab}, 
        glstate::GLState, 
        nes_texture::NesTexture, 
        panels::{
            create_initial_dock_state, 
            ppu_viewer::*
        }
    }
};

use std::{
    panic::{AssertUnwindSafe, catch_unwind}, 
    path::PathBuf, sync::{Arc, Mutex}, 
    time::{Duration, Instant}
};

pub struct App {
    window: Option<Arc<Window>>,
    gl_state: Option<GLState>,        // glutin + glow
    egui_glow: Option<EguiGlow>,      // egui rendering
    dock_state: DockState<Tab>,

    nes: Option<Arc<Mutex<EmulatorInstance>>>,
    nes_texture: Option<NesTexture>,
    rom_path: Option<PathBuf>,

    audio: Option<(AudioOutput, u32)>,
    input_state: ControllerState,

    config: EmulatorConfig,
    instant: Instant,

    lua: Lua,
    active_scripts: Vec<LuaScript>,

}
impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            gl_state: None,
            egui_glow: None,
            dock_state: create_initial_dock_state(),

            nes: None,
            nes_texture: None,
            rom_path: None,

            audio: AudioOutput::new(44100),
            input_state: ControllerState {
                a: false, b: false,
                up: false, down: false,
                left: false, right: false,
                start: false, select: false,
            },

            config: EmulatorConfig::load(),
            instant: Instant::now(),

            lua: Lua::new(),
            active_scripts: Vec::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let icon_bytes = include_bytes!("../../assets/icon.ico");

        let image_data = image::load_from_memory(icon_bytes)
            .expect("Failed to load icon")
            .into_rgba8();

        let (width, height) = image_data.dimensions();

        let icon = winit::window::Icon::from_rgba(image_data.into_raw(), width, height)
            .expect("Failed to create winit icon");

        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                .with_title("NES Emulator")
                .with_window_icon(Some(icon))
                .with_inner_size(
                    winit::dpi::LogicalSize::new(1280, 720)
                )
            )
            .unwrap()
        );
        let gl_state = GLState::new(event_loop, &window);
        let egui_glow = EguiGlow::new(event_loop, Arc::clone(&gl_state.gl), None, None, true);
        
        
        self.window = Some(window);
        self.gl_state = Some(gl_state);
        self.egui_glow = Some(egui_glow);
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {

        // 1. CAPTURING KEYBOARD STATE
        if let WindowEvent::KeyboardInput { event, .. } = &event {
            if let PhysicalKey::Code(keycode) = event.physical_key {
                let is_pressed = event.state == ElementState::Pressed;
                
                match keycode {
                    KeyCode::KeyZ => self.input_state.a = is_pressed,
                    KeyCode::KeyX => self.input_state.b = is_pressed,
                    KeyCode::KeyC => self.input_state.start = is_pressed,
                    KeyCode::KeyV => self.input_state.select = is_pressed,
                    KeyCode::ArrowUp => self.input_state.up = is_pressed,
                    KeyCode::ArrowDown => self.input_state.down = is_pressed,
                    KeyCode::ArrowLeft => self.input_state.left = is_pressed,
                    KeyCode::ArrowRight => self.input_state.right = is_pressed,
                    _ => {}
                }
            }
        }

        // 2. PASSING IT TO THE GRAPHIC INTERFACE
        if let Some(egui_glow) = &mut self.egui_glow {
            let response = egui_glow.on_window_event(self.window.as_ref().unwrap(), &event);
            if response.consumed { return; }
        }
        
        // 3. PROCESSING WINDOW AND EMULATOR EVENTS
        match event {
            WindowEvent::Resized(physical_size) => {
                let (w, h) = (physical_size.width, physical_size.height);
                if let Some(gl_state) = &self.gl_state {
                    gl_state.resize(w, h);
                    unsafe {
                        gl_state.gl.viewport(0, 0, w as i32, h as i32);
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let target_frame_time = Duration::from_secs_f64(1.0 / 60.0988);
                let elapsed = self.instant.elapsed();

                if elapsed < target_frame_time {
                    std::thread::sleep(target_frame_time - elapsed);
                }
                
                self.instant = Instant::now();

                let gl_state = self.gl_state.as_ref().unwrap();
                let egui_glow = self.egui_glow.as_mut().unwrap();
                let window = self.window.as_ref().unwrap();
                let gl = &gl_state.gl;

                unsafe {
                    gl.clear_color(0.1, 0.1, 0.1, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                if self.nes_texture.is_none() && !self.nes.is_none() {
                    self.nes_texture = Some(NesTexture::new(gl, egui_glow));
                }

                let mut engine_crashed: bool = false;

                if let Some(nes_arc) = &mut self.nes {
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        if let Ok(mut emu) = nes_arc.lock() {
                        
                            //TODO: probably optimize this
                            emu.cpu.bus.ppu.color_palette = self.config.palette.clone();
                            emu.cpu.bus.apu.volume = self.config.volume / 100.0;

                            apply_input(&mut emu.cpu.bus.joypad_1, &self.input_state, &self.config);

                            emu.run_frame(&mut self.audio);
                            
                            let texture = self.nes_texture.as_ref().unwrap();
                            texture.update(gl, emu.frame_buffer());
                        }
                    }));

                    if result.is_err() {
                        engine_crashed = true
                    }

                }

                let mut open_rom_requested = false;
                let mut pause_requested = false; 
                let mut reset_requested = false;
                let dock = &mut self.dock_state;
                let nes_ref = self.nes.as_ref();

                let texture_opt = self.nes_texture.as_ref().map(|nt| nt.egui_texture_id);

                let _repaint_after = egui_glow.run(window, |ctx| { 
                    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                        egui::menu::bar(ui, |ui| {
                            ui.menu_button("File", |ui| {
                                if ui.button("Open ROM...").clicked() {
                                    open_rom_requested = true;
                                    ui.close_menu();
                                }
                                if ui.button("Exit").clicked() { event_loop.exit(); }
                            });
                            ui.menu_button("NES", |ui| {
                                if ui.button("Reset").clicked() {
                                    reset_requested = true;
                                    ui.close_menu();
                                 }
                                if ui.button("Pause").clicked() { 
                                    pause_requested = true; 
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("View", |ui| {
                                if ui.button("NES Screen").clicked() {
                                    if let Some(nes_game_viewer) = dock.find_tab(&Tab::Emulator) {
                                        dock.remove_tab(nes_game_viewer);
                                    } else {
                                        dock.main_surface_mut().push_to_first_leaf(Tab::Emulator);
                                    }
                                }
                                if ui.button("Settings").clicked() {
                                    if let Some(settings) = dock.find_tab(&Tab::Settings) {
                                        dock.remove_tab(settings);
                                    } else {
                                        dock.main_surface_mut().push_to_first_leaf(Tab::Settings);
                                    }
                                }
                                if ui.button("Terminal").clicked() {
                                    if let Some(terminal) = dock.find_tab(&Tab::Terminal) {
                                        dock.remove_tab(terminal);
                                    } else {
                                        dock.main_surface_mut().push_to_first_leaf(Tab::Terminal);
                                    }
                                }
                                if ui.button("CPU Viewer").clicked() {
                                    if let Some(cpu_viewer) = dock.find_tab(&Tab::CpuViewer) {
                                        dock.remove_tab(cpu_viewer);
                                    } else {
                                        dock.main_surface_mut().push_to_first_leaf(Tab::CpuViewer);
                                    }
                                }
                                if ui.button("PPU Viewer").clicked() {
                                    if let Some(ppu_viewer) = dock.find_tab(&Tab::PpuViewer) {
                                        dock.remove_tab(ppu_viewer);
                                    } else {
                                        dock.main_surface_mut().push_to_first_leaf(Tab::PpuViewer);
                                    }
                                }
                                if ui.button("Memory Viewer").clicked() {
                                    if let Some(memory_viewer) = dock.find_tab(&Tab::MemoryEditor) {
                                        dock.remove_tab(memory_viewer);
                                    } else {
                                        dock.main_surface_mut().push_to_first_leaf(Tab::MemoryEditor);
                                    }
                                }
                            });
                        });
                    });
                    
                    DockArea::new(dock)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show(ctx, &mut NesTabViewer {
                        nes_texture: texture_opt,
                        emulator: nes_ref,
                        config: &mut self.config,
                        pattern_viewer: &mut pattern_viewer::PatternTableViewer::new(),
                        nametable_viewer: &mut palette_viewer::PaletteViewer::new(),
                        lua: &self.lua,
                        active_scripts: &mut self.active_scripts,
                    });
                });

                if engine_crashed {
                    print_logs(
                        LogType::Warning, 
                        "⚠️THE NES ENGINE SUFFERED A CRITICAL FAILIURE. RESTARTING GAME..."
                    );
                    reset_requested = true;
                }

                if open_rom_requested {
                    if let Some(path) = crate::frontend::panels::open_rom::open_rom_dialog() {
                        match crate::engine::instance::EmulatorInstance::new(path.clone()) {
                            Ok(emu) => {
                                self.rom_path = Some(path);

                                let emu_shared = Arc::new(Mutex::new(emu));
                                self.nes = Some(emu_shared.clone());

                                self.lua = Lua::new();

                                if let Err(err) = crate::engine::terminal::lua::setup_lua_functions::lua_api_setup(&self.lua, emu_shared) {
                                    print_logs(LogType::Warning, format!("Lua API Setup Error: {}", err));
                                }
                            }
                            Err(e) => {
                                print_logs(LogType::Warning, format!("Failed to load ROM: {}", e));
                            }
                        }
                    }
                }

                if pause_requested {
                    if let Some(nes_arc) = &self.nes {
                        if let Ok(mut emu) = nes_arc.lock() {
                            emu.is_paused = !emu.is_paused;
                        }
                    }
                }

                if reset_requested {
                    if self.nes.is_some() {
                        if let Some(path) = &self.rom_path {
                            match crate::engine::instance::EmulatorInstance::new(path.clone()) {
                                Ok(emu) => {
                                    let emu_shared = std::sync::Arc::new(std::sync::Mutex::new(emu));
                                    self.nes = Some(emu_shared.clone());

                                    self.lua = mlua::Lua::new();

                                    if let Err(err) = crate::engine::terminal::lua::setup_lua_functions::lua_api_setup(&self.lua, emu_shared) {
                                        print_logs(LogType::Warning, format!("Lua API Setup Error (Reset): {}", err));
                                    }
                                }
                                Err(e) => {
                                    print_logs(LogType::Warning, format!("Failed to load ROM during reset: {}", e));
                                }
                            }
                        }
                    }
                }

                egui_glow.paint(window);
                gl_state.swap_buffers();
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                self.config.save();
                event_loop.exit();
            }
            _ => { }
        }
    }
}