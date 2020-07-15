use chrono::Local;
use log::debug;

use crate::handler::categorizer::{Categorizer, CategoryProvider};
use crate::handler::events::{Amount, BudgetRecord, HandlerEvent};
use crate::handler::tokenizer::{tokenize, MessageTokens, Token};

mod categorizer;
pub(crate) mod events;
mod tokenizer;

#[derive(Debug)]
pub struct Input {
    pub id: i64,
    pub user: String,
    pub text: String,
    pub is_new: bool,
}

#[derive(Debug)]
pub struct Output {
    pub text: String,
    pub events: Vec<HandlerEvent>,
}

pub struct RawMessageParser {
    categorizer: Categorizer,
}

impl RawMessageParser {
    pub fn new<P: CategoryProvider>(provider: &P) -> RawMessageParser {
        let mut categorizer = Categorizer::new();
        categorizer.load_categories(provider);
        RawMessageParser { categorizer }
    }

    pub fn handle_message(&mut self, input: Input) -> Option<Output> {
        debug!("{:?}", &input);
        let tokens = tokenize(&input.text);
        let record = BudgetRecord {
            id: input.id,
            date: Local::today().naive_local(),
            category: self.categorizer.classify(&tokens)?.name.to_owned(),
            amount: RawMessageParser::extract_amount(&tokens)?,
            desc: RawMessageParser::extract_description(&tokens),
            user: input.user,
        };
        let event = if input.is_new {
            HandlerEvent::AddRecord(record)
        } else {
            HandlerEvent::UpdateRecord(record)
        };
        let reply = RawMessageParser::build_reply_message(&event);
        let output = Output {
            text: reply,
            events: vec![event],
        };
        debug!("{:?}", &output);
        Some(output)
    }

    fn build_reply_message(event: &HandlerEvent) -> String {
        match event {
            HandlerEvent::AddRecord(record) => format!(
                "Added new record #{}\nDate: {}\nCategory: {}\nAmount: {}",
                record.id, record.date, record.category, record.amount,
            ),
            HandlerEvent::UpdateRecord(record) => format!(
                "Updated existed record #{}\nDate: {}\nCategory: {}\nAmount: {}",
                record.id, record.date, record.category, record.amount,
            ),
        }
    }

    fn extract_description(tokens: &MessageTokens) -> String {
        let mut result = String::new();
        let mut trailing_signs_buffer = None;
        for t in tokens {
            match t {
                Token::TrailingSigns(signs)
                    if !result.is_empty() && trailing_signs_buffer.is_none() =>
                {
                    trailing_signs_buffer = Some(signs);
                }
                Token::Word(word) => {
                    if let Some(signs) = trailing_signs_buffer.take() {
                        result.push_str(signs);
                    }
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(word);
                }
                _ => {}
            }
        }
        result
    }

    #[allow(dead_code)]
    fn parse_amount(text: &str) -> Option<Amount> {
        RawMessageParser::extract_amount(&tokenize(text))
    }

    fn extract_amount(tokens: &MessageTokens) -> Option<Amount> {
        tokens.iter().find_map(|t| match t {
            Token::Amount(amount) => Some(amount.clone()),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::handler::events::Amount;
    use crate::handler::tokenizer::tokenize;
    use crate::handler::RawMessageParser as MH;

    #[test]
    fn parse_amount_as_first_word() {
        assert_eq!(
            MH::parse_amount("10.25 for banana pie"),
            Some(Amount(String::from("10.25")))
        );
    }

    #[test]
    fn parse_amount_as_last_word() {
        assert_eq!(
            MH::parse_amount("Chocolate pie for 9,75."),
            Some(Amount(String::from("9.75")))
        );
    }

    #[test]
    fn parse_amount_take_first_matched_number() {
        assert_eq!(
            MH::parse_amount("5 for 2 kg of candies"),
            Some(Amount(String::from("5")))
        );
    }

    #[test]
    fn extract_description_with_signs_after_amount_in_the_beginning() {
        assert_eq!(
            MH::extract_description(&tokenize("9,75. Chocolate pie")),
            "Chocolate pie".to_string()
        )
    }

    #[test]
    fn extract_description_with_signs_after_amount_in_the_end() {
        assert_eq!(
            MH::extract_description(&tokenize("Chocolate pie, 9,75.")),
            "Chocolate pie".to_string()
        )
    }
}
