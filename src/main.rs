use std::env;

use telegram_bot::*;
use tg_bot_playground::{read_last_update, start};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let api = Api::new(token);

    start(api).await
    // read_last_update(api).await
}
