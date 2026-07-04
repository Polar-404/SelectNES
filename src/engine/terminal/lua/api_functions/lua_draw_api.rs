use mlua::prelude::*;

pub fn register_draw_api(lua: &Lua) -> LuaResult<()> {
    let draw_box = lua.create_function(|lua, (x1, y1, x2, y2, color): (i32, i32, i32, i32, String)| {
        let globals = lua.globals();
        let commands: mlua::Table = globals.get("_draw_commands").unwrap_or_else(|_| {
            let t = lua.create_table().unwrap();
            let _ = globals.set("_draw_commands", t.clone());
            t
        });

        let cmd = lua.create_table()?;
        cmd.set("type", "box")?;
        cmd.set("x1", x1)?;
        cmd.set("x2", x2)?;
        cmd.set("y1", y1)?;
        cmd.set("y2", y2)?;
        cmd.set("color", "color")?;

        let len = commands.len()?;
        commands.set(len + 1, cmd)?;

        Ok(())
    })?;

    let draw_text = lua.create_function(|lua, (x, y, text): (i32, i32, String)| {
        let globals = lua.globals();
        let commands: mlua::Table = globals.get("_draw_commands").unwrap_or_else(|_| {
            let t = lua.create_table().unwrap();
            let _ = globals.set("_draw_commands", t.clone());
            t
        });

        let cmd = lua.create_table()?;
        cmd.set("type", "text")?;
        cmd.set("x", x)?;
        cmd.set("y", y)?;
        cmd.set("text", text)?;

        let len = commands.len()?;
        commands.set(len + 1, cmd)?;

        Ok(())
    })?;

    let draw_line = lua.create_function(|lua, (x1, y1, x2, y2, color): (i32, i32, i32, i32, String)| {
        let globals = lua.globals();
        let commands: mlua::Table = globals.get("_draw_commands").unwrap_or_else(|_| {
            let t = lua.create_table().unwrap();
            let _ = globals.set("_draw_commands", t.clone());
            t
        });

        let cmd = lua.create_table()?;
        cmd.set("type", "box")?; 
        cmd.set("x1", x1)?;
        cmd.set("y1", y1)?;
        cmd.set("x2", x2)?;
        cmd.set("y2", y2)?;
        cmd.set("color", color)?;

        let len = commands.len()?;
        commands.set(len + 1, cmd)?;
        Ok(())
    })?;

    let draw_pixel = lua.create_function(|lua, (x, y, color): (i32, i32, String)| {
        let globals = lua.globals();
        let commands: mlua::Table = globals.get("_draw_commands").unwrap_or_else(|_| {
            let t = lua.create_table().unwrap();
            let _ = globals.set("_draw_commands", t.clone());
            t
        });

        let cmd = lua.create_table()?;
        cmd.set("type", "box")?;
        cmd.set("x1", x)?;
        cmd.set("y1", y)?;
        cmd.set("x2", x)?; // Força tamanho 1x1
        cmd.set("y2", y)?;
        cmd.set("color", color)?;

        let len = commands.len()?;
        commands.set(len + 1, cmd)?;
        Ok(())
    })?;
    
    lua.globals().set("draw_box", &draw_box)?;
    lua.globals().set("draw_text", &draw_text)?;

    lua.globals().set("box", &draw_box)?;
    lua.globals().set("text", &draw_text)?;

    lua.globals().set("line", draw_line)?;
    lua.globals().set("pixel", draw_pixel)?;

    Ok(())
}