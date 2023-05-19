use std::{time::Instant, fmt::{Display}};

#[macro_export]
macro_rules! benchmark {
    ($fmt:literal, $arg:expr) => {
        Benchmark::benchmark($fmt, $arg);
    };
}

pub struct Benchmark {
    time: Instant,
    label: &'static str
}

impl Benchmark {
    pub fn start(label: &'static str) -> Self {
        Self {
            label,
            time: Instant::now()
        }
    }

    pub fn label(&self) -> &str {
        self.label
    }

    pub fn reset(&mut self) {
        self.time = Instant::now();
    }

    pub fn benchmark<F: Fn()>(label: &'static str, f: F) {
        Benchmark::start(label);
        {
            f();
        }
    }
}

impl Drop for Benchmark {
    fn drop(&mut self) {
        println!("{}: {}", self.label, self);
    }
}

impl Display for Benchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.time.elapsed();
        
        if duration.as_secs() <= 0 {
            write!(f, "{}ms", duration.as_millis())
        } else {
            if duration.as_secs() > 60 {
                write!(f, "{:0>2}:{:0>2}min", duration.as_secs() / 60, duration.as_secs() % 60)
            } else {
                write!(f, "{}ms", duration.as_millis())
            }
        }
    }
}