use super::{assert_text, days_between_weekdays, DateShiftParser};
use crate::handler::tokenizer::{MessageTokens, Token};
use chrono::{Datelike, Duration, Local, Weekday};
use std::str::FromStr;

pub struct EnglishDateShiftParser;

impl DateShiftParser for EnglishDateShiftParser {
    fn parse_date_shift(tokens: MessageTokens) -> Option<Duration> {
        let ref mut iter = tokens.iter().enumerate();
        while let Some((i, t)) = iter.next() {
            let duration = match t {
                Token::Word(w) if w.to_lowercase() == "yesterday" => Some(Duration::days(1)),
                Token::Word(w) if w.to_lowercase() == "last" => match tokens.get(i + 1) {
                    Some(Token::Word(w)) => match Weekday::from_str(w) {
                        Ok(wd) => {
                            let x = days_between_weekdays(wd, Local::today().weekday());
                            Some(Duration::days(if x == 0 { 7 } else { x.into() }))
                        }
                        Err(..) => None,
                    },
                    _ => None,
                },
                Token::Amount(x) => match x.as_i32() {
                    Ok(x) if x > 1 && assert_text(&tokens[i + 1..=i + 2], "days ago") => {
                        Some(Duration::days(x.into()))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some(_) = duration {
                return duration;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::tokenizer::tokenize;
    use chrono::{Datelike, Local};

    type Parser = EnglishDateShiftParser;

    #[test]
    fn no_shift_by_default() {
        let tokens = tokenize("banana 4.5");
        assert_eq!(Parser::parse_date_shift(tokens), None)
    }

    #[test]
    fn yesterday() {
        let tokens = tokenize("banana 4.5 yesterday");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(1)))
    }

    #[test]
    fn some_days_ago() {
        let tokens = tokenize("banana 4.5 2 days ago");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(2)));
    }

    #[test]
    fn some_days_ago_with_int_amount() {
        let tokens = tokenize("banana 4, 5 days ago");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(5)));
    }

    #[test]
    fn last_monday() {
        let tokens = tokenize("banana 4.5 last Monday");
        let x = Local::today().weekday().num_days_from_monday();
        assert_eq!(
            Parser::parse_date_shift(tokens),
            Some(Duration::days(if x == 0 { 7 } else { x.into() }))
        );
    }
}
