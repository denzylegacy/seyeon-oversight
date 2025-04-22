use anyhow::Result;
use chrono::DateTime;
use serde::Deserialize;
use serde_json;
use seyeon_cryptocompare::CryptocompareClient;
use seyeon_cryptocompare::histoday::{Histoday, HistodayParams};
use seyeon_rapidapi::RapidApiClient;
use seyeon_rapidapi::fgi::{FearAndGreedIndex, FearAndGreedIndexResponse};
use seyeon_trading_engine::data_point::DataPoint;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::{Write, stdout};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchHistoricalDataError {
    #[error("Error decoding response body: {0}")]
    ResponseDecodeError(#[from] serde_json::Error),
    #[error("API error: {0}")]
    ApiError(String),
    // #[error("Other error: {0}")]
    // Other(anyhow::Error),
}

#[derive(Debug, Deserialize)]
pub struct Portfolio {
    pub portfolio: Vec<String>,
}

#[derive(Debug)]
pub struct FetchedData {
    pub historical: Vec<DataPoint>,
    pub fgi: Option<FearAndGreedIndexResponse>,
}

pub async fn portfolio_fetcher() -> Result<Vec<Portfolio>> {
    if let Ok(current_dir) = env::current_dir() {
        println!("pwd {:?}", current_dir);
    }

    let file = File::open("assets/options.json")?;
    let reader = BufReader::new(file);

    let portfolios: Vec<Portfolio> = serde_json::from_reader(reader)?;

    Ok(portfolios)
}

pub async fn fetch_historical_data(symbol: String, days: u32) -> anyhow::Result<FetchedData> {
    println!("Symbol being fetched: '{}'", symbol);

    let symbol = symbol.trim().to_string();

    print!("Fetching historical data of {} (please, wait!)...", symbol);

    stdout().flush()?;

    let cc_client = CryptocompareClient::new(std::env::var("CRYPTOCOMPARE_API_KEY")?.as_str());

    let data = cc_client
        .call::<Histoday>(
            HistodayParams::builder()
                .source_sym(symbol)
                .target_sym("USD")
                .limit(days)
                .build(),
        )
        .await
        .map_err(|err| {
            eprintln!("Error fetching data: {}", err);
            FetchHistoricalDataError::ApiError(err.to_string())
        })?;

    print!(" CC ");
    stdout().flush()?;

    let historical = data
        .data
        .data
        .iter()
        .map(|d| DataPoint {
            datetime: DateTime::from_timestamp(d.time as i64, 0).unwrap(),
            price: d.close,
            high: d.high,
            low: d.low,
            open: d.open,
            volume: d.volumefrom,
        })
        .collect();

    let fgi_client = RapidApiClient::new(std::env::var("RAPIDAPI_KEY")?.as_str());

    let fgi_data = match fgi_client.call0::<FearAndGreedIndex>().await {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!("Failed to fetch FGI: {}", e);
            None
        }
    };

    println!(" FGI ");

    Ok(FetchedData {
        historical,
        fgi: fgi_data,
    })
}
