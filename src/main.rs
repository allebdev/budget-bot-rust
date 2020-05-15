use tg_bot_playground::start;

#[tokio::main]
async fn main() -> Result<(), String> {
    env_logger::init();
    start().await
}
