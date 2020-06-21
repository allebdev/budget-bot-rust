use std::{env, io};

use log::*;
use tokio::sync::mpsc::Sender;

use async_trait::async_trait;

use crate::handler::Input;
use crate::input::{Command, CommandReader};

pub struct ConsoleInputHandler {
    commands: Sender<Command>,
    user: String,
}

#[allow(clippy::new_without_default)]
impl ConsoleInputHandler {}

#[async_trait(? Send)]
impl CommandReader for ConsoleInputHandler {
    fn new(commands: Sender<Command>) -> Self {
        ConsoleInputHandler {
            commands,
            user: env::var("USER").unwrap_or("console".to_string()),
        }
    }

    fn name(&self) -> &str {
        "CLI"
    }

    async fn start(mut self) -> io::Result<()> {
        loop {
            let mut text = String::new();
            if io::stdin().read_line(&mut text).is_ok() && !text.trim().is_empty() {
                let id = chrono::Utc::now().timestamp();
                let input = Input {
                    id,
                    user: self.user.clone(),
                    text,
                    is_new: true,
                };
                let cmd = Command::RecordMessage(input);
                if let Err(err) = self.commands.send(cmd).await {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error during send command: {}", err),
                    ));
                } else {
                    info!("Command {} has been sent", id);
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}
