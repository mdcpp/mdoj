use log::Level;
use super::config::CONFIG;

// logger
pub fn init() {
    let config = CONFIG.get().unwrap();

    let level = match config.log_level {
        0 => Level::Trace,
        1 => Level::Debug,
        2 => Level::Info,
        3 => Level::Warn,
        4 => Level::Error,
        _ => Level::Info,
    };

    simple_logger::init_with_level(level).unwrap();
}