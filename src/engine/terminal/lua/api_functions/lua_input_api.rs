use mlua::prelude::*;

pub fn monitor_mouse_clickpos(lua: &Lua) -> LuaResult<()> {
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