use std::{fs, path::Path};

use mlua::prelude::*;

pub fn load_lua_script_file(lua: &Lua, path: &Path) -> LuaResult<()>{
    let user_script = fs::read_to_string(path).expect("An error occurred while trying to read the file");

    lua.load(&user_script).exec()?;

    if let Ok(on_init) = lua.globals().get::<mlua::Function>("on_init") {
        on_init.call::<()>(())?;
    }

    Ok(())
}