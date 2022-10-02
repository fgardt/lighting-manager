// This is a modified version of pretty_env_logger v0.4.0 that uses Builder::from_env()

use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use env_logger::{
    fmt::{Color, Style, StyledValue},
    Builder, Env,
};
use error_stack::{IntoReport, Result, ResultExt};
use log::Level;

#[derive(Debug)]
pub struct LoggingSetupError;

impl fmt::Display for LoggingSetupError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Logging setup error")
    }
}

impl Error for LoggingSetupError {}

pub fn init(level: &str) -> Result<(), LoggingSetupError> {
    let env = Env::default().filter_or("LOG_LEVEL", level);

    Builder::from_env(env)
        .format(|buf, record| {
            use std::io::Write;

            let target = record.target();
            let max_width = max_target_width(target);

            let mut style = buf.style();
            let level = colored_level(&mut style, record.level());

            let mut style = buf.style();
            let target = style.set_bold(true).value(Padded {
                value: target,
                width: max_width,
            });

            let time = buf.timestamp_millis();
            let text = record.args().to_string();

            let target_pad = Padded {
                value: "  ",
                width: max_width,
            };

            // 24 (timestamp) + 5 (level) + 1 space = 30
            let newline_padding = format!("{:30} {}", " ", target_pad);
            let lines: Vec<_> = text.lines().collect();

            writeln!(buf, "{} {} {} > {}", time, level, target, lines[0])?;

            for line in &lines[1..] {
                writeln!(buf, "{} > {}", newline_padding, line)?;
            }

            Ok(())
        })
        .try_init()
        .report()
        .attach_printable_lazy(|| "unable to configure logger")
        .change_context(LoggingSetupError)?;

    Ok(())
}

struct Padded<T> {
    value: T,
    width: usize,
}

impl<T: fmt::Display> fmt::Display for Padded<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{: <width$}", self.value, width = self.width)
    }
}

static MAX_MODULE_WIDTH: AtomicUsize = AtomicUsize::new(0);

fn max_target_width(target: &str) -> usize {
    let max_width = MAX_MODULE_WIDTH.load(Ordering::Relaxed);
    if max_width < target.len() {
        MAX_MODULE_WIDTH.store(target.len(), Ordering::Relaxed);
        target.len()
    } else {
        max_width
    }
}

fn colored_level<'a>(style: &'a mut Style, level: Level) -> StyledValue<'a, &'static str> {
    match level {
        Level::Trace => style.set_color(Color::Magenta).value("TRACE"),
        Level::Debug => style.set_color(Color::Blue).value("DEBUG"),
        Level::Info => style.set_color(Color::Green).value("INFO "),
        Level::Warn => style.set_color(Color::Yellow).value("WARN "),
        Level::Error => style.set_color(Color::Red).value("ERROR"),
    }
}
