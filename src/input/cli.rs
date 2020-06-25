use std::{env, io};

use log::*;

use async_trait::async_trait;

use crate::handler::Input;
use crate::input::{Command, CommandReader, MainController};
use std::io::Write;

pub struct CliCommandReader {
    ctrl: MainController,
    user: String,
}

#[async_trait(? Send)]
impl CommandReader for CliCommandReader {
    fn new(controller: MainController) -> Self {
        CliCommandReader {
            ctrl: controller,
            user: env::var("USER").unwrap_or("console".to_string()),
        }
    }

    fn name(&self) -> &str {
        "CLI"
    }

    async fn start(mut self) -> io::Result<()> {
        loop {
            print!("-> ");
            io::stdout().flush()?;
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
                match self.ctrl.dispatch(cmd) {
                    Ok(response) => println!("<- {}", response),
                    Err(_) => error!("Unknown command"),
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}
