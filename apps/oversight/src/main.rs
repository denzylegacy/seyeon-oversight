use anyhow::Context;
use data_fetcher::Portfolio;
use data_fetcher::{fetch_historical_data, portfolio_fetcher};
use seyeon_rapidapi::fgi::FearAndGreedIndexResponse;
use seyeon_redis::{CryptoStatus, TradeAction, get_status, set_status, get_report_status, update_report_status};
use seyeon_trading_engine::{engine, indicators::Indicators};
use seyeon_email::EmailConfig;
use chrono::Local;
use std::thread::sleep;
use std::time::Duration;
use std::env;
mod data_fetcher;
use dotenv::dotenv;
use polars::prelude::*;

fn fgi_value(response: &FearAndGreedIndexResponse) -> Option<u8> {
    Some(response.fgi.now.value as u8)
}

async fn startup(
    daily_report: bool,
) -> anyhow::Result<()> {
    dotenv().ok();

    let email_config = match EmailConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading email configuration: {}", e);
            return Err(anyhow::anyhow!("Failed to load email configuration: {}", e));
        }
    };
    
    let fetched_portfolio: Vec<Portfolio> = portfolio_fetcher().await?;

    let mut portfolio_signals: Vec<(String, TradeAction)> = Vec::new();
    
    let mut symbols: Vec<String> = Vec::new();
    let mut prices: Vec<Vec<f64>> = Vec::new();
    let mut assets_data: Vec<(String, DataFrame)> = Vec::new();

    for field in fetched_portfolio.iter() {
        for crypto in field.portfolio.iter() {
            let crypto_symbol = crypto.trim_matches('"').trim().to_string();
            
            let current_status: CryptoStatus =
                get_status(&crypto_symbol)
                    .await
                    .unwrap_or_else(|_| CryptoStatus {
                        symbol: crypto_symbol.clone(),
                        action: TradeAction::Any,
                        sent: false,
                    });
            println!("Current status: {:#?}", current_status);

            let fetched_data = match fetch_historical_data(crypto_symbol.clone(), 2000).await {
                Ok(data) => data,
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("rate limit") {
                        eprintln!("Rate limit exceeded for {}, checking cache...", crypto_symbol);
                        
                        let cache_path = format!("apps/oversight/cache/{}_historical.json", crypto_symbol.to_lowercase());
                        if std::path::Path::new(&cache_path).exists() {
                            eprintln!("Using cache as fallback for {}", crypto_symbol);
                            
                            // Desserializar manualmente o cache para contornar o erro
                            match std::fs::read_to_string(&cache_path) {
                                Ok(cache_content) => {
                                    match serde_json::from_str::<data_fetcher::CacheEntry>(&cache_content) {
                                        Ok(cache_entry) => {
                                            eprintln!("Using cached data from {} for {}", 
                                                     cache_entry.last_updated, 
                                                     crypto_symbol);
                                                     
                                            data_fetcher::FetchedData {
                                                historical: cache_entry.data,
                                                fgi: None,
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("Cache file exists but couldn't be parsed: {}", e);
                                            return Err(anyhow::anyhow!("API rate limit exceeded and cache fallback failed: {}", e));
                                        }
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Cache file exists but couldn't be read: {}", e);
                                    return Err(anyhow::anyhow!("API rate limit exceeded and cache fallback failed: {}", e));
                                }
                            }
                        } else {
                            eprintln!("No cache available for {}", crypto_symbol);
                            return Err(anyhow::anyhow!("API rate limit exceeded and no cache available: {}", error_msg));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Failed to fetch data for {}: {}", crypto_symbol, error_msg));
                    }
                }
            };

            if daily_report {
                let asset_prices: Vec<f64> = fetched_data.historical
                    .iter()
                    .map(|dp| dp.price)
                    .collect();
                
                symbols.push(crypto_symbol.clone());
                prices.push(asset_prices);
            }

            let indicators = Indicators::new(fetched_data.historical);

            let df = indicators
                .calculate()
                .context("Failed to calculate indicators")?;
            
            if daily_report {
                assets_data.push((crypto_symbol.clone(), df.clone()));
            }

            let params = engine::Params::default();

            let fgi_value = fetched_data.fgi.as_ref().and_then(fgi_value);
            let engine = engine::TradingEngine::new(df, fgi_value, params);
            let last_event = engine.poll_event();

            println!("Engine Event: {:#?}", last_event);

            let action = match last_event.signal {
                engine::Signal::Buy => TradeAction::Buy,
                engine::Signal::Sell => TradeAction::Sell,
                engine::Signal::Hold => TradeAction::Hold,
            };

            let status = CryptoStatus {
                symbol: crypto_symbol,
                action,
                sent: false,
            };

            if &current_status.action != &status.action {
                println!("Signal changed for {}: {:?}", status.symbol, status.action);
                
                if let Err(e) = email_config.report_sender(&status).await {
                    eprintln!("Failed to send email report: {}", e);
                } else {
                    println!("Email report sent successfully!");
                }

            } else {
                println!("No change in signal for {}", status.symbol);
            }

            set_status(&status).await?;

            portfolio_signals.push((status.symbol.clone(), status.action));
        }
    }

    if daily_report {
        println!("\n===== Daily Report =====");
        for (symbol, action) in &portfolio_signals {
            println!("{}: {:?}", symbol, action);
        }

        let price_data_refs: Vec<(&str, &Vec<f64>)> = symbols.iter()
            .zip(prices.iter())
            .map(|(sym, price_vec)| (sym.as_str(), price_vec))
            .collect();
        
        println!("\n===== Generating Correlation Matrix =====");
        
        let correlation_df = match engine::TradingEngine::calculate_correlation_matrix(&price_data_refs) {
            Ok(df) => {
                println!("Correlation matrix calculated successfully");
                Some(df)
            },
            Err(e) => {
                eprintln!("Failed to calculate correlation matrix: {}", e);
                None
            }
        };
        
        let assets_data_refs: Vec<(&str, DataFrame)> = assets_data.iter()
            .map(|(sym, df)| (sym.as_str(), df.clone()))
            .collect();
        
        println!("\n===== Analyzing Asset Performance =====");
        
        let performance_results = engine::TradingEngine::compare_assets_performance(&assets_data_refs, 365);
        
        let performance_data = performance_results.into_iter()
            .map(|result| seyeon_email::AssetPerformance {
                symbol: result.symbol.to_string(),
                roi: result.roi,
            })
            .collect::<Vec<_>>();
        
        if !performance_data.is_empty() {
            println!("\nTop performers:");
            for (i, result) in performance_data.iter().take(5).enumerate() {
                println!("{}. {} - ROI: {:.2}%", i+1, result.symbol, result.roi);
            }
        }

        if let Err(e) = email_config.send_daily_report(
            portfolio_signals, 
            correlation_df, 
            if !performance_data.is_empty() { Some(performance_data) } else { None }
        ).await {
            eprintln!("Failed to send email report: {}", e);
        } else {
            println!("\nDaily report with correlation and performance analysis sent successfully by email!");
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    let args: Vec<String> = env::args().collect();
    
    let force_report = args.iter().any(|arg| arg == "--force-report");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    if force_report {
        println!("\n===== Forcing daily report generation =====");
        
        if let Err(e) = rt.block_on(async {
            startup(true).await
        }) {
            eprintln!("Error during forced report generation: {}", e);
            return Err(e);
        }
        
        println!("\nForced report generation completed.");
        return Ok(());
    }
    
    println!("\n===== Daily report will be checked and sent automatically =====");
    
    loop {
        let now = Local::now();
        let current_date = now.date_naive();
        let current_date_str = current_date.format("%Y-%m-%d").to_string();
        
        let report_status = match rt.block_on(get_report_status()) {
            Ok(status) => status,
            Err(e) => {
                eprintln!("Error getting report status from Redis: {}", e);
                seyeon_redis::models::ReportStatus::default()
            }
        };

        println!(
            "\nCurrent date: {} | Last report date: {} | Report sent today: {}",
            current_date_str, report_status.last_report_date, report_status.report_sent_today
        );

        let daily_report = if report_status.last_report_date != current_date_str {
            true
        } else {
            !report_status.report_sent_today
        };
        
        if let Err(e) = rt.block_on(async {
            startup(daily_report).await
        }) {
            eprintln!("Error during startup: {}", e);
        }
        
        if daily_report {
            if let Err(e) = rt.block_on(update_report_status(&current_date_str, true)) {
                eprintln!("Error updating report status in Redis: {}", e);
            } else {
                println!("Report status updated in Redis: date={}, sent=true", current_date_str);
            }
        }

        println!("\nWaiting for next check...");
        sleep(Duration::from_secs(300));
    }
}
