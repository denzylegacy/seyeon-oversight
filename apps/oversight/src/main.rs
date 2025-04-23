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
mod data_fetcher;
use dotenv::dotenv;

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

            let fetched_data = fetch_historical_data(crypto_symbol.clone(), 2000).await?;

            let indicators = Indicators::new(fetched_data.historical);

            let df = indicators
                .calculate()
                .context("Failed to calculate indicators")?;

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

        if let Err(e) = email_config.send_daily_report(portfolio_signals).await {
            eprintln!("Failed to send email report: {}", e);
        } else {
            println!("Email report sent successfully!");
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    dotenv().ok();

    println!("\n===== Daily report will be checked and sent automatically =====");

    let rt = tokio::runtime::Runtime::new().unwrap();

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
        sleep(Duration::from_secs(60));
    }
    
    #[allow(unreachable_code)]
    Ok(())
}
