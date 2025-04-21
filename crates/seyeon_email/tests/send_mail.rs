use chrono::Local;
use tokio::test;

use seyeon_email::EmailConfig;

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

    let mut main_status = Vec::new();
    main_status.push(("SOL", "Buy -test-"));

    if let Err(e) = email_config.report_sender(main_status) {
        eprintln!("Failed to send complete report: {}", e);
    } else {
        println!("Complete report sent successfully!");
    }
}
