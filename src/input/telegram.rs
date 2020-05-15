use log::*;
use std::{convert::Infallible, io, sync::Arc};
use tbot::{contexts::fields::Text, errors, prelude::*, Bot};

use async_trait::async_trait;

use crate::handler::{Input, Output, RawMessageParser};
use crate::input::InputHandler;

pub struct TelegramInputHandler {
    parser: RawMessageParser,
}

#[async_trait]
impl InputHandler for TelegramInputHandler {
    fn name(&self) -> &str {
        "Telegram"
    }

    async fn start(self) -> io::Result<()> {
        let result = self.start_polling().await;

        match result {
            Ok(_) => Ok(()),
            Err(errors::PollingSetup::DeleteWebhook(err)) => {
                Err(io::Error::new(io::ErrorKind::Other, err))
            }
            Err(errors::PollingSetup::DeleteWebhookTimeout(_)) => {
                Err(io::Error::new(io::ErrorKind::Other, "Timeout"))
            }
        }
    }
}

impl TelegramInputHandler {
    pub fn new(parser: RawMessageParser) -> Self {
        TelegramInputHandler { parser }
    }

    async fn start_polling(self) -> Result<Infallible, errors::PollingSetup> {
        let mut bot = Bot::from_env("BOT_TOKEN").stateful_event_loop(self);

        bot.text(|ctx, this| async move {
            this.process_text(ctx, false).await;
        });

        bot.edited_text(|ctx, this| async move {
            this.process_text(ctx, true).await;
        });

        bot.polling().start().await
    }

    async fn process_text<'s>(&self, ctx: Arc<impl Text>, edited: bool) {
        let from = match ctx.from() {
            Some(from) => from,
            None => return,
        };

        info!(
            "Message #{} from {}: '{}'",
            ctx.message_id(),
            from.first_name,
            &ctx.text().value
        );

        let username = from.username.as_deref().unwrap_or_default();
        let output = self.parser.handle_message(Input {
            id: ctx.message_id().0 as i64,
            user: username.to_owned(),
            text: ctx.text().value.clone(),
            is_new: !edited,
        });

        if let Some(Output { text }) = output {
            info!("Reply to message #{}: '{:?}'", ctx.message_id(), text);
            ctx.send_message_in_reply(&text).call().await;
        }
    }
}
