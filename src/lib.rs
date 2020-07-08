extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate serde_derive;

use log::*;

use crate::{
    handler::{events::DefaultEventHandler, RawMessageParser},
    input::{CommandReader, DefaultCommandReader, MainController},
};

mod handler;
mod input;

pub async fn start() -> Result<(), String> {
    let handler = DefaultEventHandler::new();
    let command_reader = DefaultCommandReader::new(MainController {
        parser: RawMessageParser::new(&handler),
        handler: Box::new(handler),
    });

    info!("Started with {} input handler", command_reader.name());
    command_reader
        .start()
        .await
        .map_err(|err| format!("Reader error: {}", err))
}

// Cli/Telegram => parse msg => update db => generate response
// CLI/Telegram => parse command => calculate stat (read db) => generate response
// parse input => upsert record => update db => generate response
// parse input => calc statistic => read db => generate response
// parse input => update config => r/w config => generate response
// [ reader ]  => [ handler ]  => [ db adapter ] => [ handler ] => [ reader ]
// [ reader ] => [ handler ] => [ config adapter ] => [ handler ] => [ reader ]
