use std::io;

use async_trait::async_trait;

use crate::handler::events::EventHandler;
use crate::handler::{Input, RawMessageParser};
#[cfg(feature = "cli")]
use crate::input::cli::CliCommandReader;
#[cfg(feature = "telegram")]
use crate::input::telegram::TelegramCommandReader;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "telegram")]
mod telegram;

#[async_trait(? Send)]
pub trait CommandReader {
    fn new(controller: MainController) -> Self;
    fn name(&self) -> &str;
    async fn start(self) -> io::Result<()>;
}

pub struct MainController {
    pub(crate) parser: RawMessageParser,
    pub(crate) handler: Box<dyn EventHandler + Send + Sync>,
}

impl MainController {
    fn dispatch(&mut self, cmd: Command) -> Option<String> {
        match cmd {
            Command::RecordMessage(input) => match self.parser.handle_message(input) {
                Some(output) => {
                    let mut result = Some(output.text);
                    for event in output.events {
                        if let Err(err) = self.handler.handle_event(event) {
                            result.replace(err);
                        }
                    }
                    result
                }
                None => None,
            },
        }
    }
}

#[cfg(feature = "cli")]
pub type DefaultCommandReader = CliCommandReader;

#[cfg(feature = "telegram")]
pub type DefaultCommandReader = TelegramCommandReader;

#[derive(Debug)]
pub enum Command {
    RecordMessage(Input),
}
