#[macro_use]
extern crate lazy_static;
extern crate regex;

use std::time::Duration;

use log::{info, warn};
use telegram_bot::*;
use tokio::stream::StreamExt;

use crate::handler::{Input, MessageHandler, Output};

mod handler;

pub async fn start(api: Api) -> Result<(), Error> {
    // Fetch new updates via long poll method
    let stream = api.stream();
    let mut stream = StreamExt::timeout(stream, Duration::from_secs(5));
    let handler = &MessageHandler::new();
    while let Some(Ok(update)) = stream.next().await {
        let update = &update?;
        if let Some(response) = process_update(update, handler) {
            api.send(response).await?;
        }
    }
    Ok(())
}

pub async fn read_last_update(api: Api) -> Result<(), Error> {
    let mut updates = GetUpdates::new();
    let request = updates.offset(-1).timeout(4);
    let result = api.send_timeout(request, Duration::from_secs(5)).await?;
    if let Some(ref updates) = result {
        let handler = &MessageHandler::new();
        for update in updates {
            if let Some(response) = process_update(&update, handler) {
                warn!("READONLY RUN! WILL NOT SEND RESPONSE: {:?}", response)
                // api.send(request).await?;
            }
        }
    }
    Ok(())
}

fn process_update<'s>(update: &Update, handler: &MessageHandler) -> Option<SendMessage<'s>> {
    match &update.kind {
        UpdateKind::Message(message) | UpdateKind::EditedMessage(message) => match message.kind {
            MessageKind::Text { ref data, .. } => {
                info!(
                    "Message #{} from {}: '{}'",
                    message.id, message.from.first_name, data
                );
                let username = message.from.username.as_deref().unwrap_or_default();
                let output = handler.handle_message(Input {
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
