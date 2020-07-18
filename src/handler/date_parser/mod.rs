pub mod english;

use chrono::{Duration, Weekday};

use crate::handler::tokenizer::*;

pub trait DateShiftParser {
    fn parse_date_shift(tokens: MessageTokens) -> Option<Duration>;
}

pub trait WeekdayExt {
    /// Calculate number of days between `another` and `self`
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::Weekday;
    /// use tg_bot_playground::handler::date_parser::WeekdayExt;
    ///
    /// assert_eq!(Weekday::Fri.days_since(Weekday::Mon), 4);
    /// assert_eq!(Weekday::Mon.days_since(Weekday::Fri), 3);
    /// assert_eq!(Weekday::Wed.days_since(Weekday::Wed), 0);
    /// ```
    fn days_since(&self, another: Weekday) -> u32;
}

impl WeekdayExt for Weekday {
    fn days_since(&self, another: Weekday) -> u32 {
        (7 + self.num_days_from_monday() - another.num_days_from_monday()) % 7
    }
}

pub fn assert_text(tokens: &[Token], text: &str) -> bool {
    let text = text.to_lowercase();
    let expected = tokenize(&text);
    tokens.len() == expected.len() && expected.iter().zip(tokens).all(|(t1, t2)| t1 == t2)
}
