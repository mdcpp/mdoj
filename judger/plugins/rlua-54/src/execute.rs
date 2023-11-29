use std::{
    fs,
    io::{stdin, BufRead, Read},
};

use rlua::{prelude::*, Context, Lua, ToLua, Value, Variadic};

fn lua_write(_: Context, strings: Variadic<String>) -> rlua::Result<bool> {
    for s in strings {
        print!("{}", String::from_utf8_lossy(s.as_bytes()));
    }
    Ok(true)
}

fn lua_read(ctx: Context, string: String) -> rlua::Result<LuaValue> {
    match string.as_str() {
        "*all" | "*a" => {
            let mut buf = Vec::new();
            stdin().lock().read_to_end(&mut buf).unwrap();
            let s = ctx.create_string(&buf)?;
            s.to_lua(ctx)
        }
        "*line" | "*l" => {
            let mut buf = Vec::new();
            stdin().lock().read_until(b'\n', &mut buf).ok();
            let s = ctx.create_string(&buf)?;
            s.to_lua(ctx)
        }
        "*number" | "*n" => {
            let mut reader = stdin().lock();
            let mut is_float = false;
            let mut result: Vec<u8> = Vec::new();

            loop {
                let mut buf = vec![0; 1];
                if reader.read_exact(&mut buf).is_ok() {
                    let b = buf[0];
                    match b {
                        b'0'..=b'9' => result.push(b),
                        b'.' => {
                            if is_float {
                                break;
                            }
                            is_float = true;
                            result.push(b);
                        }
                        _ => break,
                    }
                }
            }

            String::from_utf8(result)
                .unwrap()
                .parse::<f64>()
                .unwrap()
                .to_lua(ctx)
        }
        _ => match string.parse::<usize>() {
            Ok(n) => {
                let mut buf = vec![0; n];
                stdin().read_exact(&mut buf).unwrap();
                let s = ctx.create_string(&buf)?;
                s.to_lua(ctx)
            }
            Err(_) => Ok(Value::Nil),
        },
    }
}

pub fn execute() {
    let lua = Lua::new();

    lua.context(|ctx| {
        let printf = ctx.create_function(lua_write).unwrap();
        let write = ctx.create_function(lua_write).unwrap();
        let read = ctx.create_function(lua_read).unwrap();

        let io_table= ctx.create_table().unwrap();
        io_table.set("write", write).unwrap();
        io_table.set("read", read).unwrap();

        let globals = ctx.globals();
        globals.set("printf", printf).unwrap();
        globals.set("io", io_table).unwrap();

        let source = fs::read(crate::LUA_SRC).unwrap();
        let code = ctx.load(&source);

        code.exec().unwrap();
    });
}
