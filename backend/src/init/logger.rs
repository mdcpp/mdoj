use log::LevelFilter;

use super::config::CONFIG;

pub fn init() {
    let config = CONFIG.get().unwrap();

    let level = match config.log_level {
        0 => LevelFilter::Trace,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Info,
        3 => LevelFilter::Warn,
        4 => LevelFilter::Error,
        _ => LevelFilter::Off,
    };
    let mut logger = env_logger::builder();

    println!("Set up logger with {}", level);

    logger.filter_module("backend", level);
    logger.try_init().unwrap();
}
