use super::config::CONFIG;
use log::LevelFilter;


// setup logger and panic handler
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
        .filter_module("judger", level)
        .try_init()
        .ok();
    // make panic propagate across thread to ensure safety
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        log::error!(
            "Panic at {}",
            info.location().map(|x| x.to_string()).unwrap_or_default()
        );
        std::process::exit(1);
    }));
}
