use chrono::Local;
use tokio::test;
use seyeon_email::EmailConfig;
use seyeon_redis::models::{CryptoStatus, TradeAction};

#[test]
pub async fn report_sender() {
    let email_config = match EmailConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading email configuration: {}", e);
            return;
        }
    };

    let now = Local::now();
    let current_time = now.format("%B %d, %Y %I:%M:%S %p").to_string();

    println!("\n=== EXECUTING SCHEDULED DAILY REPORT ===");
    println!("Time: {}", current_time);
    println!("=====================================\n");

    let status = CryptoStatus {
        symbol: "SOL".to_string(),
        action: TradeAction::Buy,
        sent: false,
    };

    if let Err(e) = email_config.report_sender(&status).await {
        eprintln!("Failed to send complete report: {}", e);
    } else {
        println!("Complete report sent successfully!");
    }
}
