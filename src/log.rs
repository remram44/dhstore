use std::io::Write;
use std::sync::Mutex;

use log_crate::{Log, LogLevel, LogMetadata, LogRecord,
                SetLoggerError, set_logger};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

struct StderrLogger {
    stderr: Mutex<StandardStream>,
    level: LogLevel,
}

impl StderrLogger {
    fn new(level: LogLevel) -> StderrLogger {
        StderrLogger {
            stderr: Mutex::new(StandardStream::stdout(ColorChoice::Auto)),
            level: level,
        }
    }
}

impl Log for StderrLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            let mut stderr = self.stderr.lock().unwrap();
            let color = match record.metadata().level() {
                LogLevel::Error => Color::Red,
                LogLevel::Warn => Color::Magenta,
                LogLevel::Info => Color::Yellow,
                LogLevel::Debug => Color::White,
                LogLevel::Trace => Color::Blue,
            };
            stderr.set_color(&ColorSpec::new().set_fg(Some(color))).unwrap();
            writeln!(stderr, "{} - {}", record.target(), record.args())
                .unwrap();
            stderr.reset().unwrap();
            stderr.flush().unwrap();
        }
    }
}

pub fn init(level: LogLevel) -> Result<(), SetLoggerError> {
    set_logger(|max_log_level| {
        max_log_level.set(level.to_log_level_filter());
        Box::new(StderrLogger::new(level))
    })
}
