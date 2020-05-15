#[macro_use]
extern crate lazy_static;
extern crate regex;

use log::*;

use crate::handler::RawMessageParser;
use crate::input::{DefaultInputHandler, InputHandler};

mod handler;
mod input;

pub async fn start() -> Result<(), String> {
    let message_handler = RawMessageParser::new();
    let handler = DefaultInputHandler::new(message_handler);
    info!("Started with {} input handler", handler.name());
    match handler.start().await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}
