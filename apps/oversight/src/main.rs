use anyhow::Context;
use data_fetcher::Portfolio;
use data_fetcher::{fetch_historical_data, portfolio_fetcher};
use seyeon_rapidapi::fgi::FearAndGreedIndexResponse;
use seyeon_coinlore::global_market;
use seyeon_redis::{CryptoStatus, TradeAction, get_status, set_status, get_report_status, update_report_status};
use seyeon_trading_engine::{engine, indicators::Indicators};
use seyeon_email::EmailConfig;
use chrono::Local;
use std::thread::sleep;
use std::time::Duration;
mod data_fetcher;
use dotenv::dotenv;
use polars::prelude::*;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Force daily report generation
    #[arg(long)]
    force_report: bool,
    
    /// Run simulation only without sending emails
    #[arg(long)]
    simulate: bool,
    
    /// Specific crypto to simulate (e.g. BTC, ETH)
    #[arg(long)]
    crypto: Option<String>,

    /// Days to use for simulation (default: 365)
    #[arg(long, default_value = "365")]
    days: u32,
}

fn fgi_value(response: &FearAndGreedIndexResponse) -> Option<u8> {
    Some(response.fgi.now.value as u8)
}

/// Run simulation only without sending emails or updating status
async fn run_simulation(crypto_symbol: Option<String>, days: u32) -> anyhow::Result<()> {
    dotenv().ok();
    
    let fetched_portfolio: Vec<Portfolio> = portfolio_fetcher().await?;
    let mut cryptos_to_simulate = Vec::new();
    
    // If a specific symbol was provided, simulate only that one
    if let Some(symbol) = crypto_symbol {
        cryptos_to_simulate.push(symbol);
    } else {
        // Otherwise, simulate all cryptos in the portfolio
        for field in fetched_portfolio.iter() {
            for crypto in field.portfolio.iter() {
                let symbol = crypto.trim_matches('"').trim().to_string();
                cryptos_to_simulate.push(symbol);
            }
        }
    }
    
    println!("\n===== Simulation Mode =====");
    println!("Running Simulation for {} Cryptocurrencies Using {} Days of Data", 
             cryptos_to_simulate.len(), days);
    
    // Table to store results
    let mut simulation_results = Vec::new();
    
    for crypto_symbol in cryptos_to_simulate {
        println!("\n--- Simulating {} ---", crypto_symbol);
        
        // Get historical data
        let fetched_data = match fetch_historical_data(crypto_symbol.clone(), 2000).await {
            Ok(data) => data,
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("rate limit") {
                    eprintln!("Rate limit exceeded for {}, checking cache...", crypto_symbol);
                    
                    let cache_path = format!("apps/oversight/cache/{}_historical.json", crypto_symbol.to_lowercase());
                    if std::path::Path::new(&cache_path).exists() {
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
                                        eprintln!("Cache exists but cannot be parsed: {}", e);
                                        continue;
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("Cache exists but cannot be read: {}", e);
                                continue;
                            }
                        }
                    } else {
                        eprintln!("No cache available for {}", crypto_symbol);
                        continue;
                    }
                } else {
                    eprintln!("Failed to fetch data for {}: {}", crypto_symbol, error_msg);
                    continue;
                }
            }
        };
        
        let indicators = Indicators::new(fetched_data.historical);
        let df = match indicators.calculate() {
            Ok(df) => df,
            Err(e) => {
                eprintln!("Error calculating indicators for {}: {}", crypto_symbol, e);
                continue;
            }
        };
        
        let fgi_value = fetched_data.fgi.as_ref().and_then(fgi_value);
        let mut engine = engine::TradingEngine::new(crypto_symbol.clone(), df, fgi_value, engine::Params::default());
        
        println!("Running Simulation Trading for {} with {} Days of Data...", crypto_symbol, days);
        
        engine.run_simulation(Some(days as usize));
        
        let summary = engine.get_summary();
        println!("Results for {}:", crypto_symbol);
        println!("  Initial Capital: ${:.2}", summary.initial_capital);
        println!("  Final Value: ${:.2}", summary.final_portfolio_value);
        println!("  ROI: {:.2}%", summary.roi);
        println!("  Total Trades: {}", summary.num_trades);
        println!("  Total Fees Paid: ${:.2}", summary.estimated_fees_paid);
        
        simulation_results.push((
            crypto_symbol.clone(),
            summary.roi,
            summary.final_portfolio_value,
            summary.num_trades
        ));
    }
    
    simulation_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    println!("\n===== Simulation Results =====");
    println!("{:<5} {:<10} {:<15} {:<15} {:<10}", "Rank", "Crypto", "ROI", "Final Value", "# Trades");
    println!("{:-<60}", "");
    
    for (i, (symbol, roi, final_value, num_trades)) in simulation_results.iter().enumerate() {
        println!("{:<5} {:<10} {:<15.2}% {:<15.2}$ {:<10}", 
                 i+1, symbol, roi, final_value, num_trades);
    }
    
    println!("\nSimulation completed successfully!");
    
    Ok(())
}

async fn startup(
    daily_report: bool,
    days: u32,
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
                            
                            // Manually deserialize the cache to work around the error
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
            
            let fgi_value = fetched_data.fgi.as_ref().and_then(fgi_value);
            
            let engine = engine::TradingEngine::new(crypto_symbol.clone(), df, fgi_value, engine::Params::default());
            
            let last_event = engine.poll_event();

            let action = match last_event.signal {
                engine::Signal::Buy => TradeAction::Buy,
                engine::Signal::Sell => TradeAction::Sell,
                engine::Signal::Hold => TradeAction::Hold,
            };

            let status = CryptoStatus {
                symbol: crypto_symbol.clone(),
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
        
        let performance_results = engine::TradingEngine::compare_assets_performance(&assets_data_refs, days as usize);
        
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

        let fgi_data = match fetch_historical_data("BTC".to_string(), 1).await {
            Ok(data) => {
                if let Some(fgi_response) = data.fgi {
                    println!("\nFGI data fetched successfully: {} ({})", 
                             fgi_response.fgi.now.value, 
                             fgi_response.fgi.now.value_text);
                    
                    Some(seyeon_email::FearAndGreedData {
                        value: fgi_response.fgi.now.value as u8,
                        classification: fgi_response.fgi.now.value_text,
                        timestamp: fgi_response.last_updated.human_date,
                    })
                } else {
                    println!("\nNo FGI data available");
                    None
                }
            },
            Err(e) => {
                eprintln!("\nFailed to fetch FGI data: {}", e);
                None
            }
        };
        
        // Fetch global cryptocurrency market data
        println!("\n===== Fetching Global Market Data =====");
        let global_market_data = match global_market::get_global_data().await {
            Ok(data) => {
                println!("Global market data fetched successfully");
                Some(data)
            },
            Err(e) => {
                eprintln!("Failed to fetch global market data: {}", e);
                None
            }
        };

        if let Err(e) = email_config.send_daily_report(
            portfolio_signals, 
            correlation_df, 
            if !performance_data.is_empty() { Some(performance_data) } else { None },
            fgi_data,
            global_market_data
        ).await {
            eprintln!("Failed to send email report: {}", e);
        } else {
            println!("\nDaily report with correlation, performance analysis, market sentiment, and global cryptocurrency market data sent successfully by email!");
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    let args = Args::parse();
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    if args.simulate {
        if let Err(e) = rt.block_on(async {
            run_simulation(args.crypto, args.days).await
        }) {
            eprintln!("Error during simulation: {}", e);
            return Err(e);
        }
        
        println!("\nSimulation completed.");
        return Ok(());
    }
    
    if args.force_report {
        println!("\n===== Forcing daily report generation =====");
        
        if let Err(e) = rt.block_on(async {
            startup(true, args.days).await
        }) {
            eprintln!("Error during forced report generation: {}", e);
            return Err(e);
        }
        
        println!("\nForced report generation completed.");
        return Ok(());
    }
    
    // Default behavior - automatic report check
    println!("\n===== Daily report will be checked and sent automatically =====");
    
    loop {
        let now = Local::now();
        let current_date = now.date_naive();
        let current_date_str = current_date.format("%Y-%m-%d").to_string();
        
        let report_status = match rt.block_on(get_report_status()) {
            Ok(status) => status,
            Err(e) => {
                eprintln!("Erro ao obter status do relatório do Redis: {}", e);
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
            startup(daily_report, args.days).await
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
        sleep(Duration::from_secs(600));
    }
}
