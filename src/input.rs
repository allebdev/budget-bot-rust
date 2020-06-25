use std::io;

use async_trait::async_trait;

use crate::handler::events::EventHandler;
use crate::handler::{Input, RawMessageParser};
#[cfg(feature = "cli")]
use crate::input::cli::ConsoleInputHandler;
#[cfg(feature = "telegram")]
use crate::input::telegram::TelegramInputHandler;

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
    pub(crate) handler: Box<dyn EventHandler>,
}

impl MainController {
    fn dispatch(&mut self, cmd: Command) -> Result<String, ()> {
        match cmd {
            Command::RecordMessage(input) => {
                if let Some(output) = self.parser.handle_message(input) {
                    for event in output.events {
                        self.handler.handle_event(event);
                    }
                    Ok(output.text)
                } else {
                    Err(())
                }
            }
        }
    }
}

#[cfg(feature = "cli")]
pub type DefaultCommandReader = ConsoleInputHandler;

#[cfg(feature = "telegram")]
pub type DefaultCommandReader = TelegramInputHandler;

#[derive(Debug)]
pub enum Command {
    RecordMessage(Input),
}
