use std::sync::{Arc, Mutex};
use mlua::prelude::*;
use crate::engine::{instance::EmulatorInstance, terminal::print_terminal::*};

pub fn lua_api_setup(lua: &Lua, emu: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {

    set_palette_lua_op(lua, emu.clone())?;

    print_palette_lua_op(lua, emu.clone())?;

    peek_mem(lua, emu.clone())?;

    write_mem(lua, emu.clone())?;

    Ok(())
}

fn set_palette_lua_op(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let set_palette = lua.create_function(move |_, (index, color): (u8, u8)| {
        if let Ok(mut emulator) = instance.lock() {
            emulator.cpu.bus.ppu.ppubus.palette_ram[index as usize] = color;
        }
        Ok(())
    })?;

    lua.globals().set("set_palette", set_palette)?;
    Ok(())
}

fn print_palette_lua_op(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
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

fn peek_mem(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let peek_mem = lua.create_function(move |_, addr: u16| {

        let mut val = 0;

        if let Ok(emulator) = instance.lock() {
            val = emulator.cpu.bus.peek(addr);

            print_logs(LogType::Code, format!("0x{:02X}", val));
        }
        
        Ok(val)
    })?;

    lua.globals().set("read_mem", peek_mem)?;
    Ok(())
}

fn write_mem(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let write_mem = lua.create_function(move |_, (addr, val): (u16, u8) | {
        if let Ok(mut emulator) = instance.lock() {

            emulator.cpu.bus.mem_write(addr, val);

            print_logs(LogType::Code, format!("value 0x{:02X} wrote at address 0x{:02X}", val, addr));
        }
        
        Ok(())
    })?;

    lua.globals().set("write_mem", write_mem)?;
    Ok(())
}