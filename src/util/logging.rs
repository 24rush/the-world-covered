use std::collections::HashMap;

pub(crate) static mut LOGGER_CONFIG: once_cell::sync::Lazy<LoggingConfig> =
    once_cell::sync::Lazy::new(|| LoggingConfig::default());

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum LogLevel {
    INFO,
    VERBOSE,
}

#[macro_export]
macro_rules! logln {
    ($fmt:literal) => {
        if crate::logging::is_enabled(Self::CC) {
            println!("[{}:{}] {}", file!(), line!(), $fmt);
        }
    };
    ($fmt:literal, $($arg:tt)*) => {
        if crate::logging::is_enabled(Self::CC) {
            print!("[{}:{}] ", file!(), line!());
            println!($fmt, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! logsl {
    ($fmt:literal) => {
        if crate::logging::is_enabled(Self::CC) {
            print!("\r[{}:{}] {}", file!(), line!(), $fmt);            
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    };
    ($fmt:literal, $($arg:tt)*) => {
        if crate::logging::is_enabled(Self::CC) {                        
            print!("\r[{}:{}] ", file!(), line!());
            print!($fmt, $($arg)*);        
            
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    }
}

#[macro_export]
macro_rules! logvbln {
    ($fmt:literal) => {
        if crate::logging::is_enabled(Self::CC) && crate::logging::is_at_level(Self::CC, crate::logging::LogLevel::VERBOSE) {
            println!("[{}:{}] {}", file!(), line!(), $fmt);
        }
    };
    ($fmt:literal, $($arg:tt)*) => {
        if crate::logging::is_enabled(Self::CC) && crate::logging::is_at_level(Self::CC, crate::logging::LogLevel::VERBOSE) {
            print!("[{}:{}] ", file!(), line!());
            println!($fmt, $($arg)*);
        }
    }
}

#[macro_export]
macro_rules! logvbsl {
    ($fmt:literal) => {
        if crate::logging::is_enabled(Self::CC) && crate::logging::is_at_level(Self::CC, crate::logging::LogLevel::VERBOSE) {
            print!("[{}:{}] {}", file!(), line!(), $fmt);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    };
    ($fmt:literal, $($arg:tt)*) => {
        if crate::logging::is_enabled(Self::CC) && crate::logging::is_at_level(Self::CC, crate::logging::LogLevel::VERBOSE) {
            print!("\r[{}:{}] ", file!(), line!());
            print!($fmt, $($arg)*);        
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    }
}

pub fn is_enabled(cc: &'static str) -> bool {
    unsafe { LOGGER_CONFIG.cc_enabled(cc) }
}

pub fn is_at_level(cc: &'static str, level: LogLevel) -> bool {
    unsafe { LOGGER_CONFIG.cc_at_level(cc, level) }
}

pub fn disable_cc(cc: &'static str) {
    unsafe { LOGGER_CONFIG.disable_cc(cc) };
}

pub fn enable_cc(cc: &'static str, level: LogLevel) {
    unsafe { LOGGER_CONFIG.enable_cc(cc, level) };
}

pub fn set_global_logging(enabled: bool) {
    if enabled {
        unsafe { LOGGER_CONFIG.enable_global_tracing() };
    } else {
        unsafe { LOGGER_CONFIG.disable_global_tracing() };
    }
}

pub fn set_global_level(level: LogLevel) {
    unsafe {
        LOGGER_CONFIG.set_global_level(level);
    }
}

pub trait SettingsChangedSubscriber {
    fn on_settings_changed(&mut self, enabled: bool);
}

pub struct LoggingConfig {
    global_tracing_enabled: bool,
    global_level: LogLevel,
    flags: HashMap<&'static str, (bool, LogLevel)>, // <module name, (tracing enabled, trace level)>
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            global_tracing_enabled: true,
            global_level: LogLevel::INFO,
            flags: Default::default(),
        }
    }
}

impl LoggingConfig {
    pub fn cc_enabled(&self, cc: &'static str) -> bool {
        if !self.global_tracing_enabled {
            return false;
        }

        self.flags.get(cc).unwrap_or(&(true, LogLevel::INFO)).0
    }

    pub fn cc_at_level(&self, cc: &str, level: LogLevel) -> bool {
        if self.global_level >= level {
            return true;
        }

        self.flags.get(cc).unwrap_or(&(true, LogLevel::INFO)).1 == level
    }

    pub fn enable_cc(&mut self, cc: &'static str, level: LogLevel) {
        self.flags.entry(cc).or_insert((true, level));
    }

    pub fn disable_cc(&mut self, cc: &'static str) {
        self.flags.entry(cc).or_insert((false, LogLevel::INFO));
    }

    pub fn enable_global_tracing(&mut self) {
        self.global_tracing_enabled = true;
    }

    pub fn disable_global_tracing(&mut self) {
        self.global_tracing_enabled = false;
    }

    pub fn set_global_level(&mut self, level: LogLevel) {
        self.global_level = level;
    }
}
