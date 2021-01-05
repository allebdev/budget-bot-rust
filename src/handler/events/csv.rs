use std::fs::{File, OpenOptions};

use csv;

use crate::handler::categorizer::{Category, CategoryProvider};
use crate::handler::events::{EventHandler, HandlerEvent};

pub struct CsvEventHandler {
    writer: csv::Writer<File>,
}

impl CategoryProvider for CsvEventHandler {
    fn categories(&self) -> Vec<Category> {
        let file = OpenOptions::new()
            .read(true)
            .open("categories.csv")
            .expect("Can't read categories.csv");
        let mut reader = csv::ReaderBuilder::new().delimiter(b';').from_reader(file);
        let mut iter = reader.deserialize();
        let mut categories = vec![];
        while let Some(Ok(category)) = iter.next() {
            categories.push(category);
        }
        categories
    }
}

impl CsvEventHandler {
    pub fn new() -> Self {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open("records.csv")
            .expect("Can't create or read records.csv");
        let is_empty = file.metadata().map(|meta| meta.len() == 0).unwrap_or(true);
        CsvEventHandler {
            writer: csv::WriterBuilder::new()
                .has_headers(is_empty)
                .from_writer(file),
        }
    }
}

impl EventHandler for CsvEventHandler {
    fn handle_event(&mut self, event: HandlerEvent) -> Result<(), String> {
        match event {
            HandlerEvent::AddRecord(record) => self
                .writer
                .serialize(record)
                .map_err(|_| "Error during save record".to_string()),
            HandlerEvent::UpdateRecord(_) => {
                Err("Update record is not implemented yet".to_string())
            }
        }
    }
}
