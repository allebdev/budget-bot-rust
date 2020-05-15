use std::{env, io};

use async_trait::async_trait;

use crate::handler::{Input, RawMessageParser};
use crate::input::InputHandler;

pub struct ConsoleInputHandler {
    parser: RawMessageParser,
    user: String,
}

#[allow(clippy::new_without_default)]
impl ConsoleInputHandler {
    pub fn new(parser: RawMessageParser) -> Self {
        ConsoleInputHandler {
            parser,
            user: env::var("USER").unwrap_or("console".to_string()),
        }
    }
}

#[async_trait]
impl InputHandler for ConsoleInputHandler {
    fn name(&self) -> &str {
        "CLI"
    }

    async fn start(&self) -> io::Result<()> {
        let mut counter = 0i64;
        loop {
            counter += counter;
            let mut buffer = String::new();
            if io::stdin().read_line(&mut buffer).is_ok() && !buffer.trim().is_empty() {
                let output = self.parser.handle_message(Input {
                    id: counter,
                    user: self.user.clone(),
                    text: buffer,
                    is_new: true,
                });
                if let Some(output) = output {
                    println!("{}", output.text);
                } else {
                    println!("Error")
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}
