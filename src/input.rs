use std::io;

use async_trait::async_trait;

#[cfg(feature = "cli")]
use crate::input::cli::ConsoleInputHandler;
#[cfg(feature = "telegram")]
use crate::input::telegram::TelegramInputHandler;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "telegram")]
mod telegram;

#[async_trait]
pub trait InputHandler {
    fn name(&self) -> &str;
    async fn start(&self) -> io::Result<()>;
}

#[cfg(feature = "cli")]
pub type DefaultInputHandler = ConsoleInputHandler;

#[cfg(feature = "telegram")]
pub type DefaultInputHandler = TelegramInputHandler;
