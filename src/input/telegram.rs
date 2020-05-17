use std::{convert::Infallible, env, io, sync::Arc};

use log::*;
use tbot::{contexts::fields::Text, errors, prelude::*, types::update, Bot};
use tokio::{select, sync::mpsc, sync::RwLock, time};

use async_trait::async_trait;

use crate::handler::{Input, Output, RawMessageParser};
use crate::input::InputHandler;

pub struct TelegramInputHandler {
    parser: RawMessageParser,
    timeout: time::Duration,
    tx: Option<mpsc::Sender<update::Id>>,
}

#[async_trait(? Send)]
impl InputHandler for TelegramInputHandler {
    fn name(&self) -> &str {
        "Telegram"
    }

    async fn start(mut self) -> io::Result<()> {
        let timeout = self.timeout;
        info!("Start polling updates (timeout: {} sec)", timeout.as_secs());
        let (tx, mut rx) = mpsc::channel(10);
        self.tx.replace(tx);

        let stop = async {
            loop {
                if let Ok(id) = time::timeout(timeout, rx.recv()).await {
                    if let Some(id) = id {
                        debug!("Update {} handled", id.0);
                    } else {
                        info!("Sender is closed, no more updates will come");
                        break;
                    }
                } else {
                    info!("Timeout");
                    break;
                }
            }
        };

        let polling = self.poll_updates();

        select! {
            _ = stop => {
                info!("Stop polling updates");
                Ok(())
            },
            result = polling => match result {
                Ok(_) => Ok(()),
                Err(errors::PollingSetup::DeleteWebhook(err)) => {
                    Err(io::Error::new(io::ErrorKind::Other, err))
                },
                Err(errors::PollingSetup::DeleteWebhookTimeout(_)) => {
                    Err(io::Error::new(io::ErrorKind::Other, "Timeout"))
                },
            },
        }
    }
}

impl TelegramInputHandler {
    pub fn new(parser: RawMessageParser) -> Self {
        let timeout =
            env::var("BOT_TIMEOUT").map_or(5, |v| v.parse().expect("BOT_TIMEOUT must be a number"));
        TelegramInputHandler {
            parser,
            timeout: time::Duration::from_secs(timeout),
            tx: None,
        }
    }

    async fn poll_updates(self) -> Result<Infallible, errors::PollingSetup> {
        let timeout = self.timeout.as_secs() + 1;
        let mut bot = Bot::from_env("BOT_TOKEN").stateful_event_loop(RwLock::new(self));

        bot.text(|ctx, this| async move {
            this.read().await.process_text(ctx, false).await;
        });

        bot.edited_text(|ctx, this| async move {
            this.read().await.process_text(ctx, true).await;
        });

        bot.after_update(|upd, this| async move {
            let mut this = this.write().await;
            let sender = this.tx.as_mut().unwrap();
            sender.send(upd.update_id).await.unwrap();
        });

        bot.polling().timeout(timeout).start().await
    }

    async fn process_text<'s>(&self, ctx: Arc<impl Text>, edited: bool) {
        let username = ctx
            .from()
            .and_then(|user| user.username.as_deref())
            .unwrap_or_default();
        let value = &ctx.text().value;

        debug!(
            "Message #{} from {}: '{}'",
            ctx.message_id(),
            username,
            value
        );

        let output = self.parser.handle_message(Input {
            id: ctx.message_id().0 as i64,
            user: username.to_owned(),
            text: value.clone(),
            is_new: !edited,
        });

        if let Some(Output { text }) = output {
            debug!("Reply to message #{}: {:?}", ctx.message_id(), text);
            let result = ctx.send_message_in_reply(&text).call().await;
            if let Err(err) = result {
                error!("Error on reply to message #{}: {}", ctx.message_id(), err);
            }
        }
    }
}
