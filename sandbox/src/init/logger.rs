use log::LevelFilter;

use super::config::CONFIG;

pub fn init() {
    let config = CONFIG.get().unwrap();

    let level = match config.log_level {
        0 => LevelFilter::Off,
        1 => LevelFilter::Trace,
        2 => LevelFilter::Debug,
        3 => LevelFilter::Info,
        4 => LevelFilter::Warn,
        _ => LevelFilter::Error,
    };
    let mut logger = env_logger::builder();

    println!("Set up logger with {}", level);

    logger.filter_module("sandbox", level);
    logger.try_init().unwrap();
}
