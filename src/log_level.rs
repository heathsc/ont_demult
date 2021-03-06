use std::fmt;
use std::str::FromStr;

use clap::ArgMatches;

#[derive(Debug, Clone, Copy)]
pub struct LogLevel {
    pub level: usize,
}

impl FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel { level: 0 }),
            "warn" => Ok(LogLevel { level: 1 }),
            "info" => Ok(LogLevel { level: 2 }),
            "debug" => Ok(LogLevel { level: 3 }),
            "trace" => Ok(LogLevel { level: 4 }),
            "none" => Ok(LogLevel { level: 5 }),
            _ => Err("no match"),
        }
    }
}

impl LogLevel {
    pub fn is_none(&self) -> bool {
        self.level > 4
    }
    pub fn get_level(&self) -> usize {
        if self.level > 4 {
            0
        } else {
            self.level
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level_str = ["error", "warn", "info", "debug", "trace", "none"];
        if self.level < 6 {
            write!(f, "{}", level_str[self.level])
        } else {
            write!(f, "unknown")
        }
    }
}

pub fn init_log(m: &ArgMatches) {
    let verbose: LogLevel = m.value_of_t("loglevel")
        .unwrap_or_else(|_| LogLevel::from_str("info").expect("Could not set loglevel info"));

    stderrlog::new()
        .verbosity(verbose.get_level())
        .init()
        .unwrap();
}
