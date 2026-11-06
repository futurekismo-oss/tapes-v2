use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

struct SimpleLogger {
    file: Mutex<Option<File>>,
    stderr_level: Mutex<LevelFilter>,
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let stderr_enabled = metadata.level() <= *self.stderr_level.lock().unwrap();
        let file_enabled = self.file.lock().unwrap().is_some();
        stderr_enabled || file_enabled
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = format_current_time();

        let log_entry = format!("[{}] [{}] - {}\n", timestamp, record.level(), record.args());

        let stderr_allowed = match self.stderr_level.lock() {
            Ok(guard) => record.level() <= *guard,
            Err(_) => false,
        };

        // Standard output write
        if stderr_allowed {
            let _ = std::io::stderr().lock().write_all(log_entry.as_bytes());
        }

        // Flatten file lock & write with let-else
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        let Some(ref mut file) = *guard else {
            return;
        };

        let _ = file.write_all(log_entry.as_bytes());
    }

    fn flush(&self) {
        // Flatten mutex guard & file extraction
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        let Some(ref mut file) = *guard else {
            return;
        };

        let _ = file.flush();
    }
}

static LOGGER: SimpleLogger = SimpleLogger {
    file: Mutex::new(None),
    stderr_level: Mutex::new(LevelFilter::Warn),
};

pub fn init(logging: bool, log_file_path: Option<&Path>) {
    let default = match logging {
        true => LevelFilter::Trace,
        false => LevelFilter::Warn,
    };

    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|v| v.rsplit('=').next()?.trim().parse().ok())
        .unwrap_or(default);

    if let Ok(mut guard) = LOGGER.stderr_level.lock() {
        *guard = level;
    }

    // Separated file initialization to maintain early-returns
    setup_file_logger(log_file_path);

    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Trace);
    }
}

fn format_current_time() -> String {
    let Ok(duration) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) else {
        return "00:00:00".to_string();
    };

    let secs_since_epoch = duration.as_secs();

    let secs_per_min = 60;
    let secs_per_hour = 3600;

    let seconds = (secs_since_epoch % secs_per_min) as u32;
    let minutes = ((secs_since_epoch / secs_per_min) % 60) as u32;
    let hours = ((secs_since_epoch / secs_per_hour) % 24) as u32;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

// Maximum size before rotating (e.g., 5 MB)
const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024;

fn setup_file_logger(log_file_path: Option<&Path>) {
    let Some(path) = log_file_path else {
        return;
    };

    if let Ok(metadata) = fs::metadata(path)
        && metadata.len() >= MAX_LOG_SIZE
    {
        rotate_log_files(path);
    }

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };

    let _ = file.write_all(b"\n");

    let Ok(mut guard) = LOGGER.file.lock() else {
        return;
    };

    *guard = Some(file);
}

fn rotate_log_files(base_path: &Path) {
    let backup_path = format!("{}.bak", base_path.display());

    // Delete the old .bak backup if it exists
    let _ = fs::remove_file(&backup_path);

    // Rename the current log to .bak
    let _ = fs::rename(base_path, backup_path);
}
