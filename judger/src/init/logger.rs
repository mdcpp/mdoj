use log::LevelFilter;
use super::config::CONFIG;
use std::io::Write;

// logger
pub fn init() {
    let config = CONFIG.get().unwrap();

    let level = match config.log_level {
        0 => LevelFilter::Trace,
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