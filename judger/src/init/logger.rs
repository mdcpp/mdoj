use super::config::CONFIG;
use log::LevelFilter;
use std::io::Write;

// logger
pub fn init() {
    let config = CONFIG.get().unwrap();

    let level = match config.log_level {
        #[cfg(debug_assertions)]
        0 => LevelFilter::Trace,
        #[cfg(not(debug_assertions))]
        0 => LevelFilter::Debug,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Info,
        3 => LevelFilter::Warn,
        4 => LevelFilter::Error,
        _ => LevelFilter::Info,
    };
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter(Some("judger"), level)
        .init();
}
