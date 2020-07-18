pub mod english;

use chrono::{Duration, Weekday};

use crate::handler::tokenizer::*;

pub trait DateShiftParser {
    fn parse_date_shift(tokens: MessageTokens) -> Option<Duration>;
}

pub fn assert_text(tokens: &[Token], text: &str) -> bool {
    let text = text.to_lowercase();
    let expected = tokenize(&text);
    tokens.len() == expected.len() && expected.iter().zip(tokens).all(|(t1, t2)| t1 == t2)
}

/// Calculate number of days between `from` and `to` (assumed that `from` before `to`)
///
/// # Example
///
/// ```
/// use chrono::Weekday;
/// use tg_bot_playground::handler::date_parser::days_between_weekdays;
///
/// assert_eq!(days_between_weekdays(Weekday::Mon, Weekday::Fri), 4);
/// assert_eq!(days_between_weekdays(Weekday::Fri, Weekday::Mon), 3);
/// assert_eq!(days_between_weekdays(Weekday::Wed, Weekday::Wed), 0);
/// ```
pub fn days_between_weekdays(from: Weekday, to: Weekday) -> u32 {
    (7 + to.num_days_from_monday() - from.num_days_from_monday()) % 7
}
