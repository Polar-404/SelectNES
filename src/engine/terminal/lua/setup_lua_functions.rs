use std::sync::{Arc, Mutex};
use mlua::prelude::*;
use crate::engine::{instance::EmulatorInstance};

use crate::engine::terminal::lua::api_functions::*;

pub fn lua_api_setup(lua: &Lua, emu: Arc<Mutex<EmulatorInstance>>) -> LuaResult<()> {

    lua_palette_api::set_palette_lua_op(lua, emu.clone())?;

    lua_palette_api::print_palette_lua_op(lua, emu.clone())?;

    lua_memory_api::read_mem(lua, emu.clone())?;

    lua_memory_api::read_mem_singned(lua, emu.clone())?;

    lua_memory_api::write_mem(lua, emu.clone())?;

    lua_memory_api::print_mem(lua, emu.clone())?;

    lua_memory_api::print_mem_singned(lua, emu.clone())?;

    lua_memory_api::write_mem_silent(lua, emu.clone())?;

    lua_logs_api::log_code(lua)?;

    lua_input_api::monitor_mouse_clickpos(lua)?;

    lua_draw_api::register_draw_api(lua)?;
    
    Ok(())
}

