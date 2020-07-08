use chrono::Local;
use log::debug;

use crate::handler::categorizer::{Categorizer, CategoryProvider};
use crate::handler::events::{Amount, BudgetRecord, HandlerEvent};

mod categorizer;
pub(crate) mod events;

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
        let text = &input.text;
        let category = self.categorizer.classify(text)?;
        let amount = RawMessageParser::parse_amount(text)?;
        let record = BudgetRecord {
            id: input.id,
            date: Local::today().naive_local(),
            category: category.name.to_owned(),
            amount,
            desc: input.text.trim().to_string(),
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

    fn parse_amount(text: &str) -> Option<Amount> {
        let trim_pattern: &[_] = &['.', ','];
        for word in text.split_whitespace() {
            let word = word.trim_end_matches(trim_pattern);
            let amount = word.parse();
            if let Ok(amount) = amount {
                return Some(amount);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::handler::events::Amount;
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
}
