use crate::models::CryptoStatus;
use redis::{AsyncCommands, Client, RedisError};
use serde_json::{from_str, to_string};
use std::env;

fn get_redis_url() -> String {
    env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
}

pub async fn set_status(status: &CryptoStatus) -> redis::RedisResult<()> {
    let redis_url = get_redis_url();
    let client = Client::open(redis_url)?;
    let mut con = client.get_async_connection().await?;
    let data = to_string(status).map_err(|e| {
        RedisError::from((
            redis::ErrorKind::TypeError,
            "Serialization failed",
            e.to_string(),
        ))
    })?;
    con.set(&status.symbol, data).await
}

pub async fn get_status(symbol: &str) -> redis::RedisResult<CryptoStatus> {
    let redis_url = get_redis_url();
    let client = Client::open(redis_url)?;
    let mut con = client.get_async_connection().await?;
    let data: String = con.get(symbol).await?;
    from_str(&data).map_err(|e| {
        RedisError::from((
            redis::ErrorKind::TypeError,
            "Deserialization failed",
            e.to_string(),
        ))
    })
}

pub async fn mark_as_sent(symbol: &str) -> redis::RedisResult<()> {
    let mut status = get_status(symbol).await?;
    status.sent = true;
    set_status(&status).await
}
