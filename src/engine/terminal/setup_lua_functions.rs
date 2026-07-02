use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::engine::{state::EmulatorState, terminal::print_terminal::*};

pub fn lua_api_setup(lua: &Lua, emu_state: Arc<Mutex<EmulatorState>>) -> LuaResult<()> {
    let globals = lua.globals();

    let state: Arc<Mutex<EmulatorState>> = emu_state.clone();



    Ok(())
}

fn set_palette_lua_op(lua: &Lua, state: Arc<Mutex<EmulatorState>>) -> LuaResult<()> {

    let set_palette = lua.create_function(move |_, (index, color): (u8, u8)| {
        let mut guard = state.lock().unwrap();
        if let EmulatorState::Running { emulator_instance, .. } = &mut *guard {
            emulator_instance.cpu.bus.ppu.ppubus.palette_ram[index as usize] = color;
        }
        Ok(())
    })?;

    Ok(())
}

fn print_palette_lua_op(lua: &Lua, state: Arc<Mutex<EmulatorState>>) -> LuaResult<()> {
    let print_palette = lua.create_function(move |_, ()| {
        let guard = state.lock().unwrap();
        if let EmulatorState::Running { emulator_instance, .. } = &*guard {
            let palette = &emulator_instance.cpu.bus.ppu.ppubus.palette_ram;
            print_logs(LogType::Info, "=== PALETTE RAM ===");
            for (index, &color) in palette.iter().enumerate() {
                print_logs(LogType::Info, format!("  [{:02}] 0x{:02X}", index, color));
            }
            print_logs(LogType::Info, "===================");
        }
        Ok(())
    })?;

    lua.globals().set("print_palette", print_palette)?;
    Ok(())
}