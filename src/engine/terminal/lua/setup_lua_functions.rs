use std::sync::{Arc, Mutex};
use mlua::prelude::*;
use crate::engine::{instance::EmulatorInstance, terminal::{lua::functions, print_terminal::*}};

pub fn lua_api_setup(lua: &Lua, emu: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {

    set_palette_lua_op(lua, emu.clone())?;

    print_palette_lua_op(lua, emu.clone())?;

    read_mem(lua, emu.clone())?;

    read_mem_singned(lua, emu.clone())?;

    write_mem(lua, emu.clone())?;

    log_code(lua)?;

    monitor_mouse_clickpos(lua)?;

    functions::lua_draw_api::register_draw_api(lua)?;
    
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

fn read_mem(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let read_mem = lua.create_function(move |_, addr: u16| {

        let mut val = 0;

        if let Ok(emulator) = instance.lock() {
            val = emulator.cpu.bus.peek(addr);

            print_logs(LogType::Code, format!("0x{:02X}", val));
        }
        
        Ok(val)
    })?;

    lua.globals().set("read_mem", read_mem)?;
    Ok(())
}

fn read_mem_singned(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
    let read_mem_singned = lua.create_function(move |_, addr: u16| {

        let mut val = 0;

        if let Ok(emulator) = instance.lock() {
            val = emulator.cpu.bus.peek(addr) as i8;

            print_logs(LogType::Code, format!("0x{:02X}", val));
        }

        Ok(val)
    })?;
    
    lua.globals().set("read_mem_signed", read_mem_singned)?;
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

fn log_code(lua: &Lua) -> LuaResult<()> {
    let log_code = lua.create_function(|_, message: String| {
        print_logs(LogType::Code, message);
        Ok(())
    })?;

    lua.globals().set("log_code", log_code)?;
    Ok(())
}

fn monitor_mouse_clickpos(lua: &Lua) -> LuaResult<()> {
    let mouse_clicked = lua.create_function(|lua, ()| {
        let globals = lua.globals();
        if let Ok(inpt) = globals.get::<mlua::Table>("inpt") {
            let just_clicked: bool = inpt.get("just_clicked").unwrap_or(false);
            return Ok(just_clicked);
        }
        Ok(false)
    })?;
    lua.globals().set("mouse_clicked", mouse_clicked)?;
    Ok(())
}

