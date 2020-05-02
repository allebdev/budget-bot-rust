use log::debug;

use crate::handler::events::{EventHandler, HandlerEvent};

pub struct GoogleDocsEventHandler {}

impl EventHandler for GoogleDocsEventHandler {
    fn handle_event(&self, event: HandlerEvent) {
        debug!("Handle event: {:?}", event)
        // TODO: get connect to GD and add event's record
    }
}

impl GoogleDocsEventHandler {
    pub fn new() -> Self {
        GoogleDocsEventHandler {}
    }
}
