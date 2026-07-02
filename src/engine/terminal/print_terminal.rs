use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

pub static LOG_WARNING_ENABLED: AtomicBool = AtomicBool::new(true);
pub static LOG_DEBUG_ENABLED: AtomicBool = AtomicBool::new(true);
pub static LOG_INFO_ENABLED: AtomicBool = AtomicBool::new(true);

pub static TERMINAL: Mutex<Vec<LogMessage>> = Mutex::new(Vec::new());

pub enum LogType {
    Warning,
    Debug,
    Info,
}

pub struct LogMessage {
    pub log_type: LogType,
    pub log_msg: String,
}

pub fn print_logs(log_type: LogType, log_msg: impl Into<String>) {
    let is_enabled = match log_type {
        LogType::Warning => LOG_WARNING_ENABLED.load(Ordering::Relaxed),
        LogType::Debug   => LOG_DEBUG_ENABLED.load(Ordering::Relaxed),
        LogType::Info    => LOG_INFO_ENABLED.load(Ordering::Relaxed),
    };

    if !is_enabled { return; }

    if let Ok(mut logs) = TERMINAL.lock() {
        logs.push(LogMessage { 
            log_type, 
            log_msg: log_msg.into()
        });
        if logs.len() > 750 {
            logs.remove(0);
        }
    }
}