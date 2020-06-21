use std::fs::{File, OpenOptions};

use csv;

use crate::handler::events::{EventHandler, HandlerEvent};

pub struct CsvEventHandler {
    writer: csv::Writer<File>,
}

impl EventHandler for CsvEventHandler {
    fn new() -> Self {
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

    fn handle_event(&mut self, event: HandlerEvent) {
        match event {
            HandlerEvent::AddRecord(record) => {
                self.writer
                    .serialize(record)
                    .expect("Error during save record");
            }
            HandlerEvent::UpdateRecord(_) => {}
        }
    }
}
