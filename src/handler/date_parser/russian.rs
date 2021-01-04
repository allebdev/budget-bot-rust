use std::str::FromStr;

use chrono::{Datelike, Duration, Local};

use crate::handler::{
    date_parser::{russian::weekdayrus::WeekdayRus, WeekdayExt},
    tokenizer::{MessageTokens, Token},
};

use super::DateShiftParser;

mod weekdayrus;

pub struct RussianDateShiftParser;

impl DateShiftParser for RussianDateShiftParser {
    fn parse_date_shift(tokens: &MessageTokens) -> Option<Duration> {
        let ref mut iter = tokens.iter().enumerate();
        while let Some((i, t)) = iter.next() {
            let duration = match t {
                Token::Word(_) if t.is_word("вчера") => Some(Duration::days(1)),
                Token::Word(_) if t.is_word("позавчера") => Some(Duration::days(2)),
                Token::Word(_)
                    if tokens.len() > i + 1
                        && tokens[i].any_of_words(&["прошлый", "прошлую", "прошлое", "в"]) =>
                {
                    match tokens.get(i + 1) {
                        Some(Token::Word(w)) => match WeekdayRus::from_str(w) {
                            Ok(wd) => {
                                let x = Local::today().weekday().days_since(wd.into());
                                Some(Duration::days(if x == 0 { 7 } else { x.into() }))
                            }
                            Err(..) => None,
                        },
                        _ => None,
                    }
                }
                Token::Amount(x) => match x.as_i32() {
                    Ok(x) if tokens.len() > i + 2 && tokens[i + 2].is_word("назад") => {
                        match &tokens[i + 1] {
                            w1 if w1.any_of_words(&["неделю", "недели", "недель"]) => {
                                Some(Duration::weeks(x.into()))
                            }
                            w1 if w1.any_of_words(&["день", "дней", "дня"]) => {
                                Some(Duration::days(x.into()))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
                Token::Word(w) if tokens.len() > i + 1 && tokens[i + 1].is_word("назад") => {
                    match w {
                        _ if w.eq_ignore_ascii_case("неделю") => Some(Duration::weeks(1)),
                        _ if w.eq_ignore_ascii_case("день") => Some(Duration::days(1)),
                        _ => None,
                    }
                }
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
    use chrono::{Datelike, Local, Weekday};

    use crate::handler::tokenizer::tokenize;

    use super::*;

    type Parser = RussianDateShiftParser;

    #[test]
    fn no_shift_by_default() {
        let tokens = &tokenize("бананы 50");
        assert_eq!(Parser::parse_date_shift(tokens), None)
    }

    #[test]
    fn yesterday() {
        let tokens = &tokenize("бананы 45,50 вчера");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(1)))
    }

    #[test]
    fn the_day_before_yesterday() {
        let tokens = &tokenize("бананы 45,50 позавчера");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(2)))
    }

    #[test]
    fn a_day_ago() {
        let tokens = &tokenize("бананы 45,50 день назад");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(1)));
    }

    #[test]
    fn some_days_ago() {
        let tokens = &tokenize("бананы 45,50 2 дня назад");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(2)));
    }

    #[test]
    fn some_weeks_ago() {
        let tokens = &tokenize("бананы 45,50 2 недели назад");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::weeks(2)));
    }

    #[test]
    fn a_week_ago() {
        let tokens = &tokenize("бананы 45,50 неделю назад");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::weeks(1)));
    }

    #[test]
    fn some_days_ago_with_int_amount() {
        let tokens = &tokenize("бананы 45, 5 дней назад");
        assert_eq!(Parser::parse_date_shift(tokens), Some(Duration::days(5)));
    }

    #[test]
    fn last_monday() {
        let tokens = &tokenize("бананы 45,50 прошлый понедельник");
        let x = Local::today().weekday().num_days_from_monday();
        assert_eq!(
            Parser::parse_date_shift(tokens),
            Some(Duration::days(if x == 0 { 7 } else { x.into() }))
        );
    }

    #[test]
    fn on_last_friday() {
        let tokens = &tokenize("30 бананы в прошлую пятницу");
        let x = Local::today().weekday().days_since(Weekday::Fri);
        assert_eq!(
            Parser::parse_date_shift(tokens),
            Some(Duration::days(if x == 0 { 7 } else { x.into() }))
        );
    }

    #[test]
    fn on_thursday() {
        let tokens = &tokenize("100 бананы в четверг");
        let x = Local::today().weekday().days_since(Weekday::Thu);
        assert_eq!(
            Parser::parse_date_shift(tokens),
            Some(Duration::days(if x == 0 { 7 } else { x.into() }))
        );
    }
}
