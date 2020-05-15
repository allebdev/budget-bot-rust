use std::convert::TryInto;

use chrono::Local;
use log::debug;

use crate::handler::categorizer::{load_categories, Categorizer};
use crate::handler::events::google_docs::GoogleDocsEventHandler;
use crate::handler::events::{Amount, BudgetRecord, EventHandler, HandlerEvent};

mod categorizer;
mod events;

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
}

pub struct RawMessageParser {
    categorizer: Categorizer,
    event_handler: GoogleDocsEventHandler,
}

impl RawMessageParser {
    pub fn new() -> RawMessageParser {
        let mut categorizer = Categorizer::new();
        load_categories(&mut categorizer);
        RawMessageParser {
            categorizer,
            event_handler: GoogleDocsEventHandler::new(),
        }
    }

    pub fn handle_message(&self, input: Input) -> Option<Output> {
        debug!("{:?}", &input);
        let text = &input.text;
        let category = self.categorizer.classify(text)?;
        let amount = RawMessageParser::parse_amount(text)?;
        let record = BudgetRecord {
            id: input.id,
            date: Local::today().naive_local(),
            category: category.name.to_owned(),
            amount,
            desc: input.text,
            user: input.user,
        };
        let mut events = Vec::new();
        if input.is_new {
            events.push(HandlerEvent::AddRecord(record))
        } else {
            events.push(HandlerEvent::UpdateRecord(record))
        }
        let reply = RawMessageParser::build_reply_message(events.iter().next().as_ref().unwrap());
        self.handle_events(events);
        let output = Output { text: reply };
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
            let amount = word.try_into();
            if let Ok(amount) = amount {
                return Some(amount);
            }
        }
        None
    }

    fn handle_events(&self, events: Vec<HandlerEvent>) {
        for event in events {
            self.event_handler.handle_event(event);
        }
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
