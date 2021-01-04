use std::str::FromStr;

use chrono::{Datelike, Duration, Local, Weekday};

use crate::handler::{
    date_parser::WeekdayExt,
    tokenizer::{MessageTokens, Token},
};

use super::DateShiftParser;

pub struct EnglishDateShiftParser;

impl DateShiftParser for EnglishDateShiftParser {
    fn parse_date_shift(tokens: &MessageTokens) -> Option<Duration> {
        let ref mut iter = tokens.iter().enumerate();
        while let Some((i, t)) = iter.next() {
            let duration = match t {
                Token::Word(_) if t.is_word("yesterday") => Some(Duration::days(1)),
                Token::Word(_) if t.any_of_words(&["last", "on"]) && tokens.len() > i + 1 => {
                    match tokens.get(i + 1) {
                        Some(Token::Word(w)) => match Weekday::from_str(w) {
                            Ok(wd) => {
                                let x = Local::today().weekday().days_since(wd);
                                Some(Duration::days(if x == 0 { 7 } else { x.into() }))
                            }
                            Err(..) => None,
                        },
                        _ => None,
                    }
                }
                Token::Amount(x) => match x.as_i32() {
                    Ok(x) if tokens.len() > i + 2 && tokens[i + 2].is_word("ago") => {
                        match tokens[i + 1] {
                            _ if tokens[i + 1].is_word("days") => Some(Duration::days(x.into())),
                            _ if tokens[i + 1].is_word("weeks") => Some(Duration::weeks(x.into())),
                            _ => None,
                        }
                    }
                    _ => None,
                },
                Token::Word(w) if tokens.len() > i + 1 && tokens[i + 1].is_word("ago") => match w {
                    _ if w.eq_ignore_ascii_case("week") => Some(Duration::weeks(1)),
                    _ if w.eq_ignore_ascii_case("day") => Some(Duration::days(1)),
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
    use chrono::{Datelike, Local};

    use crate::handler::tokenizer::tokenize;

    use super::*;

    type Parser = EnglishDateShiftParser;

    #[test]
    fn no_shift_by_default() {
        let tokens = &tokenize("banana 4.5");
        assert_eq!(Parser::parse_date_shift(tokens), None)
    }

    #[test]
    fn yesterday() {
        let tokens = &tokenize("banana 4.5 yesterday");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(1)))
    }

    #[test]
    fn some_days_ago() {
        let tokens = &tokenize("banana 4.5 2 days ago");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(2)));
    }

    #[test]
    fn some_days_ago_with_int_amount() {
        let tokens = &tokenize("banana 4, 5 days ago");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(5)));
    }

    #[test]
    fn a_week_ago() {
        let tokens = &tokenize("banana 4.5 a week ago");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::weeks(1)));
    }

    #[test]
    fn some_weeks_ago() {
        let tokens = &tokenize("banana 4.5 2 weeks ago");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::weeks(2)));
    }

    #[test]
    fn last_monday() {
        let tokens = &tokenize("banana 4.5 last Monday");
        let x = Local::today().weekday().num_days_from_monday();
        assert_eq!(
            Parser::parse_date_shift(tokens),
            Some(Duration::days(if x == 0 { 7 } else { x.into() }))
        );
    }
}
