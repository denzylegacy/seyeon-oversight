use anyhow::Context;
use data_fetcher::Portfolio;
use data_fetcher::{fetch_historical_data, portfolio_fetcher};
use seyeon_rapidapi::fgi::FearAndGreedIndexResponse;
use seyeon_redis::{CryptoStatus, set_status, get_status, TradeAction};
use seyeon_trading_engine::{engine, indicators::Indicators};
mod data_fetcher;

fn fgi_value(response: &FearAndGreedIndexResponse) -> Option<u8> {
    Some(response.fgi.now.value as u8)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let fetched_portfolio: Vec<Portfolio> = portfolio_fetcher().await?;

    for field in fetched_portfolio.iter() {
        for crypto in field.portfolio.iter() {
            let crypto_symbol = crypto.trim_matches('"').trim().to_string();
            
            let current_status: CryptoStatus = get_status(&crypto_symbol)
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

            set_status(&status).await?;
        }
    }

    Ok(())
}
