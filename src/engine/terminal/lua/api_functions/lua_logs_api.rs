
use mlua::prelude::*;
use crate::engine::terminal::print_terminal::*;

pub fn log_code(lua: &Lua) -> LuaResult<()> {
    let log_code = lua.create_function(|_, message: String| {
        print_logs(LogType::Code, message);
        Ok(())
    })?;

    lua.globals().set("log_code", log_code)?;
    Ok(())
}