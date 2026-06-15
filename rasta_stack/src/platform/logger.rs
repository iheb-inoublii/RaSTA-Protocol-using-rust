// Simple logging trait for debugging in a no_std environment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

pub trait Logger {
    fn log(&self, level: LogLevel, message: &str);
}
