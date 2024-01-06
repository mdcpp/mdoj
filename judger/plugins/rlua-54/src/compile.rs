use std::{
    fs,
    io::{stdin, Read},
};

pub fn compile() {
    let mut buf = Vec::new();
    stdin().read_to_end(&mut buf).unwrap();

    fs::write(crate::LUA_SRC, buf).unwrap();
}
