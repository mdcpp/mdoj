use tracing::Level;

use super::config::CONFIG;

pub fn init() {
    let config = CONFIG.get().unwrap();

    let level = match config.log_level {
        0 => Level::TRACE,
        1 => Level::DEBUG,
        2 => Level::INFO,
        3 => Level::WARN,
        4 => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .json()
        .with_max_level(level)
        .with_current_span(false)
        .try_init()
        .ok();
}
