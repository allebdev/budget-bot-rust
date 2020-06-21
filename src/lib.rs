extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate serde_derive;

use log::*;
use tokio::sync::mpsc::Receiver;

use crate::handler::events::{DefaultEventHandler, EventHandler};
use crate::handler::RawMessageParser;
use crate::input::{Command, CommandReader, DefaultCommandReader};

mod handler;
mod input;

pub async fn start() -> Result<(), String> {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    let command_reader = DefaultCommandReader::new(cmd_tx);
    let parser = RawMessageParser::new();
    let event_handler = DefaultEventHandler::new();

    start_with(cmd_rx, command_reader, parser, event_handler).await
}

async fn start_with<CR, EH>(
    mut cmd_rx: Receiver<Command>,
    command_reader: CR,
    mut parser: RawMessageParser,
    mut event_handler: EH,
) -> Result<(), String>
where
    CR: CommandReader,
    EH: EventHandler + Send + 'static,
{
    let commands_handle = tokio::spawn(async move {
        debug!("Start listening to commands channel");
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                Command::RecordMessage(input) => {
                    if let Some(output) = parser.handle_message(input) {
                        for event in output.events {
                            event_handler.handle_event(event);
                        }
                        println!("Response: {}", output.text);
                    }
                }
            };
        }
        debug!("Stop listening to commands channel");
    });

    info!("Started with {} input handler", command_reader.name());
    let result1 = command_reader
        .start()
        .await
        .map_err(|err| format!("Reader error: {}", err));
    let result2 = commands_handle
        .await
        .map_err(|err| format!("Can't join commands handler: {}", err));
    result1.and(result2)
}

// Cli/Telegram => parse msg => update db => generate response
// CLI/Telegram => parse command => calculate stat (read db) => generate response
// parse input => upsert record => update db => generate response
// parse input => calc statistic => read db => generate response
// parse input => update config => r/w config => generate response
// [ reader ]  => [ handler ]  => [ db adapter ] => [ handler ] => [ reader ]
// [ reader ] => [ handler ] => [ config adapter ] => [ handler ] => [ reader ]
