use log::{LevelFilter, Log, Metadata, Record};
use std::io::Write;

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // unlike eprintln!, this shouldn't panic if stderr is closed or broken.
            let _ = writeln!(
                std::io::stderr().lock(),
                "[{}] {}",
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

pub fn init(logging: bool) {
    let default = if logging { LevelFilter::Trace } else { LevelFilter::Warn };

    let level = std::env::var("RUST_LOG")
        .ok()
        .map_or(default, |v| {
            v.rsplit('=').next().unwrap_or(&v).trim().parse().unwrap_or(LevelFilter::Warn)
        });

    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(level);
    }
}
