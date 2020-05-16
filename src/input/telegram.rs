use std::time::Duration;
use std::{env, io};

use log::*;
use telegram_bot::{
    Api, CanReplySendMessage, Error, GetUpdates, MessageKind, SendMessage, Update, UpdateKind,
};
use tokio::stream::StreamExt;

use async_trait::async_trait;

use crate::handler::{Input, Output, RawMessageParser};
use crate::input::InputHandler;

pub struct TelegramInputHandler {
    parser: RawMessageParser,
    api: Api,
    mode: StartMode,
}

enum StartMode {
    ReadOnly,
    Polling,
}

#[async_trait(? Send)]
impl InputHandler for TelegramInputHandler {
    fn name(&self) -> &str {
        "Telegram"
    }

    async fn start(self) -> io::Result<()> {
        let result = match self.mode {
            StartMode::ReadOnly => self.read_updates().await,
            StartMode::Polling => self.poll_updates().await,
        };
        match result {
            Ok(_) => Ok(()),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }
}

impl TelegramInputHandler {
    pub fn new(parser: RawMessageParser) -> Self {
        let token = env::var("BOT_TOKEN").expect("BOT_TOKEN not set");
        let mode = env::var("BOT_READONLY").map_or(StartMode::Polling, |_| StartMode::ReadOnly);
        TelegramInputHandler {
            parser,
            api: Api::new(token),
            mode,
        }
    }

    async fn read_updates(&self) -> Result<(), Error> {
        debug!("Read updates only");
        let mut updates = GetUpdates::new();
        let request = updates.timeout(4);
        let result = self
            .api
            .send_timeout(request, Duration::from_secs(5))
            .await?;
        if let Some(ref updates) = result {
            for update in updates {
                if let Some(response) = self.process_update(&update) {
                    debug!("Unsent response: {:?}", response)
                }
            }
        }
        Ok(())
    }

    async fn poll_updates(&self) -> Result<(), Error> {
        debug!("Polling updates");
        let stream = self.api.stream();
        let mut stream = StreamExt::timeout(stream, Duration::from_secs(5));
        while let Some(Ok(update)) = stream.next().await {
            let update = &update?;
            if let Some(response) = self.process_update(update) {
                self.api.send(response).await?;
            }
        }
        Ok(())
    }

    fn process_update<'s>(&self, update: &Update) -> Option<SendMessage<'s>> {
        match &update.kind {
            UpdateKind::Message(message) | UpdateKind::EditedMessage(message) => match message.kind
            {
                MessageKind::Text { ref data, .. } => {
                    info!(
                        "Message #{} from {}: '{}'",
                        message.id, message.from.first_name, data
                    );
                    let username = message.from.username.as_deref().unwrap_or_default();
                    let output = self.parser.handle_message(Input {
                        id: message.id.into(),
                        user: username.to_owned(),
                        text: data.to_owned(),
                        is_new: message.edit_date.is_none(),
                    });
                    if let Some(Output { text }) = output {
                        info!("Reply to message #{}: '{:?}'", message.id, text);
                        Some(message.text_reply(text))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }
}
