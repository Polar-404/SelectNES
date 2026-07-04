
use std::sync::{Arc, Mutex};
use mlua::prelude::*;

use crate::engine::{instance::EmulatorInstance, terminal::print_terminal::*};

pub fn read_mem(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
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

pub fn read_mem_singned(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
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

pub fn write_mem(lua: &Lua, instance: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {
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