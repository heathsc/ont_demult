use std::fmt;

use clap::{ValueEnum, builder::PossibleValue};

/// LogLevel
///
/// Represents minimum level of messages that will be logged
///
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    #[default]
    Start,
    Both,
    Either,
    Xor,
}

impl ValueEnum for Strategy {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Start, Self::Both, Self::Either, Self::Xor]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Start => Some(PossibleValue::new("start")),
            Self::Both => Some(PossibleValue::new("both")),
            Self::Either => Some(PossibleValue::new("either")),
            Self::Xor => Some(PossibleValue::new("xor")),
        }
    }
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Start => "start",
                Self::Both => "both",
                Self::Either => "either",
                Self::Xor => "xor",
            }
        )
    }
}
