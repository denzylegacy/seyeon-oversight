use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
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
use std::path::Path;
use thiserror::Error;
use rand::seq::SliceRandom;

fn get_random_api_key(env_var_name: &str) -> anyhow::Result<String> {
    let api_keys = std::env::var(env_var_name)?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();
    
    if api_keys.is_empty() {
        return Err(anyhow::anyhow!("No valid API keys found in {}", env_var_name));
    }
    
    let mut rng = rand::thread_rng();
    let selected_key = api_keys.choose(&mut rng)
        .ok_or_else(|| anyhow::anyhow!("Failed to select a random API key"))?;
    
    println!("Selected a random API key from {} ({} keys available)", 
        env_var_name, api_keys.len());
    
    Ok(selected_key.clone())
}

#[derive(Error, Debug)]
pub enum FetchHistoricalDataError {
    #[error("Error decoding response body: {0}")]
    ResponseDecodeError(#[from] serde_json::Error),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub last_updated: DateTime<Utc>,
    pub data: Vec<DataPoint>,
}

pub async fn portfolio_fetcher() -> Result<Vec<Portfolio>> {
    let current_dir = env::current_dir()?;
    println!("pwd {:?}", current_dir);

    let file_path = std::path::Path::new("assets/options.json");
    
    let file = if file_path.exists() {
        File::open(file_path)?
    } else {
        let alt_path = std::path::Path::new("apps/oversight/assets/options.json");
        if alt_path.exists() {
            File::open(alt_path)?
        } else {
            return Err(anyhow::anyhow!("The options.json file was not found in any of the expected paths: 
                - {current_dir}/assets/options.json
                - {current_dir}/apps/oversight/assets/options.json",
                current_dir = current_dir.display()));
        }
    };
    
    let reader = BufReader::new(file);

    let portfolios: Vec<Portfolio> = serde_json::from_reader(reader)?;

    Ok(portfolios)
}

fn check_cache(symbol: &str, max_age_days: i64) -> Option<Vec<DataPoint>> {
    let cache_dir = Path::new("apps/oversight/cache");
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir).ok()?;
    }

    let cache_file = cache_dir.join(format!("{}_historical.json", symbol.to_lowercase()));
    if !cache_file.exists() {
        return None;
    }

    let file = File::open(cache_file).ok()?;
    let reader = BufReader::new(file);

    let cache_entry: CacheEntry = serde_json::from_reader(reader).ok()?;
    
    let now = Utc::now();
    let age = now.signed_duration_since(cache_entry.last_updated);
    
    if age <= Duration::days(max_age_days) {
        println!("Using cached data for {} from {}", symbol, cache_entry.last_updated);
        Some(cache_entry.data)
    } else {
        println!("Cache for {} is too old ({} days), fetching new data", 
             symbol, age.num_days());
        None
    }
}

fn save_to_cache(symbol: &str, data: &Vec<DataPoint>) -> std::io::Result<()> {
    // Use the same directory with guaranteed permissions
    let cache_dir = Path::new("apps/oversight/cache");
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir)?;
    }

    let cache_file = cache_dir.join(format!("{}_historical.json", symbol.to_lowercase()));
    
    let cache_entry = CacheEntry {
        last_updated: Utc::now(),
        data: data.clone(),
    };
    
    let json = serde_json::to_string_pretty(&cache_entry)?;
    std::fs::write(cache_file, json)?;
    
    println!("Data saved to cache for {}", symbol);
    Ok(())
}

pub async fn fetch_historical_data(symbol: String, days: u32) -> anyhow::Result<FetchedData> {
    let symbol = symbol.trim_matches(|c| c == '"' || c == '\'' || c == ' ').to_string();
    println!("Symbol being fetched: '{}'", symbol);
    
    if let Some(cached_data) = check_cache(&symbol, 1) {
        println!("Using cached data for {}", symbol);
        
        let rapid_api_key = get_random_api_key("RAPIDAPI_KEY")?;
        let fgi_client = RapidApiClient::new(&rapid_api_key);
        let fgi_data = match fgi_client.call0::<FearAndGreedIndex>().await {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Failed to fetch FGI: {}", e);
                None
            }
        };
        
        return Ok(FetchedData {
            historical: cached_data,
            fgi: fgi_data,
        });
    }

    print!("Fetching historical data of {} (please, wait!)...", symbol);
    stdout().flush()?;

    let api_key = get_random_api_key("CRYPTOCOMPARE_API_KEY")?;
    println!("Using API key: {}...", &api_key.chars().take(5).collect::<String>());
    
    let cc_client = CryptocompareClient::new(&api_key);

    let days_to_request = days; // std::cmp::min(days, 60);
    
    let params = HistodayParams::builder()
        .source_sym(symbol.clone())
        .target_sym("USD")
        .limit(days_to_request)
        .build();
        
    println!("Calling API with reduced params: source_sym={}, target_sym=USD, limit={} (reduced from {})", 
             symbol, days_to_request, days);

    let data = match cc_client.call::<Histoday>(params).await {
        Ok(data) => {
            if data.response == "Error" {
                if data.message.contains("rate limit") {
                    return Err(FetchHistoricalDataError::RateLimitError(data.message).into());
                } else {
                    return Err(FetchHistoricalDataError::ApiError(data.message).into());
                }
            }
            
            if data.data.is_none() {
                return Err(FetchHistoricalDataError::ApiError("No data returned by API".to_string()).into());
            }
            
            data
        }
        Err(err) => {
            eprintln!("\nAPI call failed: {}", err);
            
            println!("Attempting to get raw response...");
            
            let url = format!(
                "https://min-api.cryptocompare.com/data/v2/histoday?fsym={}&tsym=USD&limit={}",
                symbol, days_to_request
            );
            
            let client = reqwest::Client::new();
            let response = match client
                .get(&url)
                .header("Authorization", format!("Apikey {}", api_key))
                .send()
                .await {
                    Ok(resp) => resp,
                    Err(e) => {
                        eprintln!("Raw HTTP request failed: {}", e);
                        return Err(FetchHistoricalDataError::ApiError(format!("HTTP request failed: {}", e)).into());
                    }
                };
                
            if !response.status().is_success() {
                let status = response.status();
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(_) => String::from("Failed to read response body")
                };
                eprintln!("API returned error status {}: {}", status, body);
                
                if body.contains("rate limit") {
                    return Err(FetchHistoricalDataError::RateLimitError(body).into());
                }
                
                return Err(FetchHistoricalDataError::ApiError(format!("API returned status {}: {}", status, body)).into());
            }
            
            let body = match response.text().await {
                Ok(body) => body,
                Err(e) => {
                    eprintln!("Failed to read response body: {}", e);
                    return Err(FetchHistoricalDataError::ApiError(format!("Failed to read response body: {}", e)).into());
                }
            };
            
            println!("Raw API response (first 200 chars): {}", &body.chars().take(200).collect::<String>());
            
            if body.contains("rate limit") {
                return Err(FetchHistoricalDataError::RateLimitError(body).into());
            }
            
            return Err(FetchHistoricalDataError::ApiError(err.to_string()).into());
        }
    };

    print!(" CC \n\n");
    stdout().flush()?;

    let historical = match &data.data {
        Some(data_container) => {
            data_container.data
                .iter()
                .map(|d| DataPoint {
                    datetime: DateTime::from_timestamp(d.time as i64, 0).unwrap(),
                    price: d.close,
                    high: d.high,
                    low: d.low,
                    open: d.open,
                    volume: d.volumefrom,
                })
                .collect()
        },
        None => return Err(FetchHistoricalDataError::ApiError("No data available".to_string()).into()),
    };

    if let Err(e) = save_to_cache(&symbol, &historical) {
        eprintln!("Warning: Failed to save data to cache: {}", e);
    }

    let rapid_api_key = get_random_api_key("RAPIDAPI_KEY")?;
    let fgi_client = RapidApiClient::new(&rapid_api_key);

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
