use std::error::Error;

use lambda_runtime::{error::HandlerError, lambda, Context};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use tg_bot_playground::start;

#[derive(Deserialize, Serialize, Clone)]
struct LambdaRequest {
    id: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct LambdaResponse {
    success: bool,
}

impl LambdaResponse {
    fn success() -> LambdaResponse {
        LambdaResponse { success: true }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    lambda!(lambda_handler);
    Ok(())
}

fn lambda_handler(_req: LambdaRequest, _c: Context) -> Result<LambdaResponse, HandlerError> {
    let mut runtime = Runtime::new().unwrap();
    let result = runtime.block_on(start());
    result.map_err(|err| HandlerError::from(err.as_ref()))?;
    Ok(LambdaResponse::success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Timelike};

    #[test]
    fn test_deserialize_request() {
        let request = r#"{"id":"123-456-789", "anotherField": 123}"#;
        let result: Result<LambdaRequest, _> = serde_json::from_str(request);
        if let Err(ref err) = result {
            eprintln!("{:?}", err);
            result.unwrap();
        }
    }

    #[test]
    fn test_serialize_response() {
        let value = LambdaResponse::success();
        let result = serde_json::to_string(&value);
        match result {
            Err(ref err) => {
                eprintln!("{:?}", err);
                result.unwrap();
            }
            Ok(s) => {
                assert_eq!(r#"{"success":true}"#, s);
            }
        }
    }
}
