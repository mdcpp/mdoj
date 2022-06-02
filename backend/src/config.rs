use rand::prelude::*;
use std::{fs, io::Write, path};

pub async fn init() {
    if !path::Path::new("config").exists() {
        fs::create_dir("config").unwrap();
    }
    if !path::Path::new("config/aes").exists() {
        let mut rng = thread_rng();
        let buf = rng.gen::<[u8; 32]>();

        let mut file = fs::File::create("config/aes").unwrap();

        file.write_all(&buf).unwrap();
    }
}
