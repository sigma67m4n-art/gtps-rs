use core::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl Level {
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "error" => Self::Error,
            "warn" | "warning" => Self::Warn,
            "debug" | "trace" => Self::Debug,
            _ => Self::Info,
        }
    }

    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

pub fn init(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

fn enabled(level: Level) -> bool {
    (level as u8) <= LEVEL.load(Ordering::Relaxed)
}

pub fn emit(level: Level, message: &str) {
    if enabled(level) {
        println!("[{:<5}] {message}", level.tag());
    }
}

pub fn error(message: impl AsRef<str>) {
    emit(Level::Error, message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    emit(Level::Warn, message.as_ref());
}

pub fn info(message: impl AsRef<str>) {
    emit(Level::Info, message.as_ref());
}

pub fn debug(message: impl AsRef<str>) {
    emit(Level::Debug, message.as_ref());
}
