use std::io;

use tokio::sync::mpsc::Sender;

use async_trait::async_trait;

use crate::handler::Input;
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
    fn new(commands: Sender<Command>) -> Self;
    fn name(&self) -> &str;
    async fn start(self) -> io::Result<()>;
}

#[cfg(feature = "cli")]
pub type DefaultCommandReader = ConsoleInputHandler;

#[cfg(feature = "telegram")]
pub type DefaultCommandReader = TelegramInputHandler;

#[derive(Debug)]
pub enum Command {
    RecordMessage(Input),
}
