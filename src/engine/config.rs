use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering};

use serde::{Serialize, Deserialize};
use crate::{engine::terminal::print_terminal::{LogType, print_logs}, ppu::palettes::*};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct EmulatorConfig {
    pub volume: f32,
    pub hide_overscan: bool,
    pub terminal_types: (bool, bool, bool),
    pub multiply_resolution: i32,
    pub allow_opposite_directions: bool,
    pub palette: PaletteTheme,
    pub custom_palettes: HashMap<String, Vec<NESColor>>,
    
}
impl EmulatorConfig {
    pub fn load() -> Self {
        let path = Self::get_config_path();
        let config = match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                eprintln!("[WARNING] Structural failiure at the config.json file, ({}). Will re-write using default configs", e);
                Self::default()
            }),
            Err(_) => Self::default(),
        };
        crate::engine::terminal::print_terminal::LOG_INFO_ENABLED.store(config.terminal_types.0, Ordering::Relaxed);
        crate::engine::terminal::print_terminal::LOG_WARNING_ENABLED.store(config.terminal_types.1, Ordering::Relaxed);
        crate::engine::terminal::print_terminal::LOG_DEBUG_ENABLED.store(config.terminal_types.2, Ordering::Relaxed);
        
        config
    }
    fn get_config_path() -> PathBuf {
        let mut config_path = PathBuf::new();
        config_path.push("config");
        config_path.set_extension("json");
        config_path
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let config_path = Self::get_config_path();
            let _ = std::fs::write(config_path, content);
        }
        print_logs(LogType::Info,format!("Saved Settings."));
    }
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            volume: 10.0,
            hide_overscan: true,
            terminal_types: (true, true, false),
            multiply_resolution: 2,
            allow_opposite_directions: true,
            custom_palettes: HashMap::new(),
            palette: PaletteTheme::DefaultNtsc,
        }
    }
}