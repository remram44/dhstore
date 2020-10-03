//! Log utilities.
//!
//! This provides the log implementation that uses `termcolor` to log to the
//! terminal with colors.

use std::io::Write;

use log::{Log, Level, LevelFilter, Metadata, Record,
          SetLoggerError, set_boxed_logger, set_max_level};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// The logger that writes to stderr.
///
/// This is an internal object passed to the `log` crate; you only have to use
/// the `init()` function to make this work.
struct StderrLogger {
    stderr: StandardStream,
    level: Level,
}

impl StderrLogger {
    fn new(level: Level) -> StderrLogger {
        StderrLogger {
            stderr: StandardStream::stdout(ColorChoice::Auto),
            level: level,
        }
    }
}

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut stderr = self.stderr.lock();
            let color = match record.metadata().level() {
                Level::Error => Color::Red,
                Level::Warn => Color::Yellow,
                Level::Info => Color::White,
                Level::Debug => Color::Cyan,
                Level::Trace => Color::Blue,
            };
            stderr.set_color(ColorSpec::new().set_fg(Some(color))).unwrap();
            writeln!(stderr, "{} - {}", record.target(), record.args())
                .unwrap();
            stderr.reset().unwrap();
        }
    }

    fn flush(&self) {
        let mut stderr = self.stderr.lock();
        stderr.flush().unwrap();
    }
}

/// Sets up the logger object to log on stderr with the given log level.
pub fn init(level: Level) -> Result<(), SetLoggerError> {
    set_max_level(LevelFilter::Info);
    set_boxed_logger(Box::new(StderrLogger::new(level)))
}
