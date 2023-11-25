use std::{
    fs,
    io::{stdin, Read},
};

use rlua::{Lua, Variadic};

const LUA_SRC: &str = "/src/code.txt";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).unwrap().as_str() {
        "compile" => compile(),
        "execute" => execute(),
        _ => println!("4: Invalid command"),
    };
}

fn compile() {
    let mut buf = Vec::new();
    stdin().read_to_end(&mut buf).unwrap();

    fs::write(LUA_SRC, buf).unwrap();
}

fn execute() {
    let lua = Lua::new();
    lua.context(|ctx| {
        let printf = ctx.create_function(|_, strings: Variadic<String>| {
            for s in strings {
                print!("{}", String::from_utf8_lossy(s.as_bytes()));
            }
            Ok(1)
        }).unwrap();
        let globals = ctx.globals();
        globals.set("printf", printf).unwrap();

        let source = fs::read(LUA_SRC).unwrap();
        let code=ctx.load(&source);

        code.exec().unwrap();
    });
}
