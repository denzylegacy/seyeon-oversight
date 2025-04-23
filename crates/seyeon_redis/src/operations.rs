use crate::models::{CryptoStatus, ReportStatus};
use redis::{AsyncCommands, Client, RedisError};
use serde_json::{from_str, to_string};
use std::env;

const REPORT_STATUS_KEY: &str = "seyeon:report_status";

fn get_redis_url() -> String {
    env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
}

pub async fn get_client() -> Result<Client, RedisError> {
    let redis_url = get_redis_url();
    Client::open(redis_url)
}

pub async fn set_status(status: &CryptoStatus) -> Result<(), RedisError> {
    let client = get_client().await?;
    let mut con = client.get_async_connection().await?;
    let data = to_string(status).map_err(|e| {
        RedisError::from((
            redis::ErrorKind::TypeError,
            "Serialization failed",
            e.to_string(),
        ))
    })?;
    let _: () = con.set(&status.symbol, data).await?;
    Ok(())
}

pub async fn get_status(symbol: &str) -> Result<CryptoStatus, RedisError> {
    let client = get_client().await?;
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

pub async fn mark_as_sent(symbol: &str) -> Result<(), RedisError> {
    let mut status = get_status(symbol).await?;
    status.sent = true;
    set_status(&status).await
}

pub async fn get_report_status() -> Result<ReportStatus, RedisError> {
    let client = get_client().await?;
    let mut connection = client.get_async_connection().await?;
    
    let exists: bool = connection.exists(REPORT_STATUS_KEY).await?;
    
    if exists {
        let status_json: String = connection.get(REPORT_STATUS_KEY).await?;
        let status: ReportStatus = serde_json::from_str(&status_json)
            .map_err(|e| RedisError::from((redis::ErrorKind::IoError, "Serde error", e.to_string())))?;
        Ok(status)
    } else {
        let default_status = ReportStatus::default();
        set_report_status(&default_status).await?;
        Ok(default_status)
    }
}

pub async fn set_report_status(status: &ReportStatus) -> Result<(), RedisError> {
    let client = get_client().await?;
    let mut connection = client.get_async_connection().await?;
    
    let status_json = serde_json::to_string(&status)
        .map_err(|e| RedisError::from((redis::ErrorKind::IoError, "Serde error", e.to_string())))?;
    
    let _: () = connection.set(REPORT_STATUS_KEY, status_json).await?;
    
    Ok(())
}

pub async fn update_report_status(date: &str, sent: bool) -> Result<(), RedisError> {
    let mut status = get_report_status().await?;
    status.last_report_date = date.to_string();
    status.report_sent_today = sent;
    set_report_status(&status).await?;
    Ok(())
}
