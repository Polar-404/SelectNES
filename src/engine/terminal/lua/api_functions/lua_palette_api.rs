use std::sync::{Arc, Mutex};
use mlua::prelude::*;
use crate::engine::{instance::EmulatorInstance, terminal::struct_terminal::*};

pub fn set_palette_lua_op(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let set_palette = lua.create_function(move |_, (index, color): (u8, u8)| {
        if let Ok(mut emulator) = instance.lock() {
            emulator.cpu.bus.ppu.ppubus.palette_ram[index as usize] = color;
        }
        Ok(())
    })?;

    lua.globals().set("set_palette", set_palette)?;
    Ok(())
}

pub fn print_palette_lua_op(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let print_palette = lua.create_function(move |_, ()| {
        if let Ok(emulator) = instance.lock() {
            let palette = &emulator.cpu.bus.ppu.ppubus.palette_ram;
            print_logs(LogType::Info, "=== PALETTE RAM ===");
            for (index, &color) in palette.iter().enumerate() {
                print_logs(LogType::Code, format!("  [{:02}] 0x{:02X}", index, color));
            }
            print_logs(LogType::Info, "===================");
        }
        Ok(())
    })?;

    lua.globals().set("print_palette", print_palette)?;
    Ok(())
}