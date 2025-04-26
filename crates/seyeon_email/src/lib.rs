use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use seyeon_redis::{CryptoStatus, TradeAction};
use std::env;
use std::str::FromStr;
use chrono::Local;
use polars::prelude::*;

#[derive(Debug, Clone)]
pub struct AssetPerformance {
    pub symbol: String,
    pub roi: f64,
}

#[derive(Debug, Clone)]
pub struct FearAndGreedData {
    pub value: u8,
    pub classification: String,
    pub timestamp: String,
}

pub struct EmailConfig {
    from_email: String,
    to_email: String,
    cc_emails: Vec<String>,
    smtp_password: String,
}

impl EmailConfig {
    pub fn new() -> Result<Self, String> {
        let from_email = env::var("SMTP_FROM_EMAIL")
            .map_err(|_| "SMTP_FROM_EMAIL environment variable not found")?;

        let to_email = env::var("SMTP_TO_EMAIL")
            .map_err(|_| "SMTP_TO_EMAIL environment variable not found")?;

        let cc_emails = env::var("SMTP_CC_EMAILS")
            .map_err(|_| "SMTP_CC_EMAILS environment variable not found")?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let smtp_password = env::var("SMTP_PASSWORD")
            .map_err(|_| "SMTP_PASSWORD environment variable not found")?;

        Ok(EmailConfig {
            from_email,
            to_email,
            cc_emails,
            smtp_password,
        })
    }

    pub async fn report_sender(
        &self,
        crypto_status: &CryptoStatus
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
        
        let html_body = format!(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    color: #333333;
                    max-width: 600px;
                    margin: 0 auto;
                    padding: 20px;
                }}
                .header {{
                    background-color: #1a5fb4;
                    color: white;
                    padding: 15px;
                    text-align: center;
                    font-size: 24px;
                    font-weight: bold;
                    border-radius: 5px 5px 0 0;
                }}
                .content {{
                    padding: 15px;
                    background-color: #f9f9f9;
                    border-left: 1px solid #ddd;
                    border-right: 1px solid #ddd;
                }}
                table {{
                    width: 100%;
                    border-collapse: collapse;
                }}
                td, th {{
                    padding: 8px;
                    text-align: left;
                    border-bottom: 1px solid #ddd;
                }}
                .buy {{
                    color: #2ecc71;
                    font-weight: bold;
                }}
                .sell {{
                    color: #e74c3c;
                    font-weight: bold;
                }}
                .hold {{
                    color:rgba(241, 178, 41, 0.84);
                    font-weight: bold;
                }}
                .dcabuy {{
                    color: #27ae60;
                    font-weight: bold;
                }}
                .dcasell {{
                    color:rgb(180, 59, 46);
                    font-weight: bold;
                }}
                .footer {{
                    background-color: #eeeeee;
                    padding: 15px;
                    text-align: center;
                    font-size: 12px;
                    color: #666666;
                    border-radius: 0 0 5px 5px;
                    border: 1px solid #ddd;
                }}
                .time-info {{
                    font-style: italic;
                    color: #666666;
                    font-size: 12px;
                    margin-bottom: 10px;
                }}
            </style>
        </head>
        <body>
            <div class="header">
                세연 SEYEON OVERSIGHT - ALERT!
            </div>
            <div class="content">
                <p class="time-info">Generated at: {now}</p>
                <p>A signal change has been detected:</p>
                <table>
                    <tr>
                        <th>Cryptocurrency</th>
                        <th>Signal</th>
                    </tr>
                    <tr>
                        <td><strong>{}</strong></td>
                        <td class="{}">{:?}</td>
                    </tr>
                </table>
            </div>
            <div class="footer">
                © 2025 Seyeon Oversight - Cryptocurrency Monitoring System<br>
                This is an automated message. Please do not reply to this email.
            </div>
        </body>
        </html>
        "#, crypto_status.symbol, 
        crypto_status.action.to_string().to_lowercase(), 
        crypto_status.action);

        let mut builder = Message::builder()
            .from(self.from_email.parse()?)
            .to(self.to_email.parse()?);

        for cc_email in &self.cc_emails {
            builder = builder.cc(Mailbox::from_str(cc_email)?);
        }

        let email = builder
            .subject(format!("🚨 Seyeon Alert: New signal for {}!", crypto_status.symbol))
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::plain(format!("New signal detected for {}:\n\n{:?}", 
                            crypto_status.symbol, crypto_status.action))
                    )
                    .singlepart(
                        SinglePart::html(html_body)
                    )
            )?;

        let creds = Credentials::new(self.from_email.clone(), self.smtp_password.clone());

        let mailer = SmtpTransport::relay("smtp.gmail.com")?
            .credentials(creds)
            .build();

        mailer.send(&email)?;
        println!(
            "\nStatus report sent by email to {} and {} CCs!",
            self.to_email,
            self.cc_emails.len()
        );

        Ok(())
    }

    pub async fn send_daily_report(
        &self, 
        status_list: Vec<(String, TradeAction)>,
        correlation_data: Option<DataFrame>,
        performance_data: Option<Vec<AssetPerformance>>,
        fgi_data: Option<FearAndGreedData>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
        let date_today = Local::now().format("%d/%m/%Y").to_string();
        
        let mut html_body = format!(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    color: #333333;
                    max-width: 800px;
                    margin: 0 auto;
                    padding: 20px;
                }}
                .header {{
                    background-color: rgb(54, 88, 130);
                    color: white;
                    padding: 15px;
                    text-align: center;
                    font-size: 24px;
                    font-weight: bold;
                    border-radius: 5px 5px 0 0;
                }}
                .content {{
                    padding: 15px;
                    background-color: #f9f9f9;
                    border-left: 1px solid #ddd;
                    border-right: 1px solid #ddd;
                }}
                table {{
                    width: 100%;
                    border-collapse: collapse;
                    margin-bottom: 20px;
                }}
                td, th {{
                    padding: 8px;
                    text-align: left;
                    border-bottom: 1px solid #ddd;
                }}
                .buy {{
                    color: #2ecc71;
                    font-weight: bold;
                }}
                .sell {{
                    color: #e74c3c;
                    font-weight: bold;
                }}
                .hold {{
                    color: #f39c12;
                    font-weight: bold;
                }}
                .dcabuy {{
                    color: #27ae60;
                    font-weight: bold;
                }}
                .dcasell {{
                    color: #c0392b;
                    font-weight: bold;
                }}
                .footer {{
                    background-color: #eeeeee;
                    padding: 15px;
                    text-align: center;
                    font-size: 12px;
                    color: #666666;
                    border-radius: 0 0 5px 5px;
                    border: 1px solid #ddd;
                }}
                .time-info {{
                    font-style: italic;
                    color: #666666;
                    font-size: 12px;
                    margin-bottom: 10px;
                }}
                .section-header {{
                    background-color: #f0f0f0;
                    padding: 10px;
                    margin-top: 20px;
                    font-weight: bold;
                    border-left: 4px solid rgb(54, 88, 130);
                }}
                .correlation-table {{
                    font-size: 12px;
                }}
                .correlation-high {{
                    background-color: rgba(46, 204, 113, 0.3);
                }}
                .correlation-medium {{
                    background-color: rgba(243, 156, 18, 0.3);
                }}
                .correlation-low {{
                    background-color: rgba(231, 76, 60, 0.3);
                }}
                .correlation-neutral {{
                    background-color: rgba(189, 195, 199, 0.3);
                }}
                .performance-positive {{
                    color: #2ecc71;
                    font-weight: bold;
                }}
                .performance-negative {{
                    color: #e74c3c;
                    font-weight: bold;
                }}
                /* Fear and Greed Index styling */
                .fgi-container {{
                    max-width: 600px;
                    margin: 15px auto;
                }}
                .fgi-table {{
                    width: 100%;
                    border-collapse: collapse;
                    border-radius: 8px;
                    overflow: hidden;
                    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
                    margin-bottom: 15px;
                }}
                .fgi-value {{
                    font-size: 46px;
                    font-weight: bold;
                    text-align: center;
                    padding: 25px 15px 15px 15px;
                }}
                .fgi-classification {{
                    font-size: 20px;
                    text-align: center;
                    padding: 10px;
                }}
                .fgi-timestamp {{
                    font-size: 12px;
                    text-align: center;
                    font-style: italic;
                    padding: 5px 5px 15px 5px;
                    color: #666;
                    border-top: 1px solid rgba(0,0,0,0.05);
                }}
                .extreme-fear {{
                    background-color: rgba(231, 76, 60, 0.85);
                    color: white;
                }}
                .fear {{
                    background-color: rgba(230, 126, 34, 0.85);
                    color: white;
                }}
                .neutral {{
                    background-color: rgba(241, 196, 15, 0.85);
                    color: #333;
                }}
                .greed {{
                    background-color: rgba(46, 204, 113, 0.85);
                    color: white;
                }}
                .extreme-greed {{
                    background-color: rgba(39, 174, 96, 0.85);
                    color: white;
                }}
                .gauge-container {{
                    width: 100%;
                    height: 18px;
                    background: linear-gradient(to right, 
                        rgba(231, 76, 60, 0.75) 0%, 
                        rgba(230, 126, 34, 0.75) 25%, 
                        rgba(241, 196, 15, 0.75) 50%, 
                        rgba(46, 204, 113, 0.75) 75%, 
                        rgba(39, 174, 96, 0.75) 100%);
                    border-radius: 9px;
                    position: relative;
                    margin: 20px 0 10px 0;
                }}
                .gauge-labels {{
                    display: flex;
                    justify-content: space-between;
                    font-size: 12px;
                    margin: 5px 0 0 0;
                    color: #555;
                }}
            </style>
        </head>
        <body>
            <div class="header">
                세연 SEYEON OVERSIGHT - DAILY REPORT
            </div>
            <div class="content">
                <p class="time-info">Generated at: {now}</p>
                
                <div class="section-header">Signal report for {date_today}:</div>
                <table>
                    <tr>
                        <th>Cryptocurrency</th>
                        <th>Signal</th>
                    </tr>
        "#);

        for (crypto, action) in &status_list {
            let action_str = format!("{:?}", action);
            let class = action_str.to_lowercase();
            html_body.push_str(&format!(
                r#"<tr>
                    <td><strong>{}</strong></td>
                    <td class="{}">{}</td>
                </tr>"#,
                crypto, class, action_str
            ));
        }

        html_body.push_str(r#"
                </table>
                <p>Recommendations based on technical analysis and market indicators.</p>
        "#);

        if let Some(perf_data) = &performance_data {
            html_body.push_str(r#"<div class="section-header">Performance Analysis</div>"#);
            html_body.push_str(r#"<p>This table shows the performance of your assets based on simulated trading using our algorithm:</p>"#);
            
            html_body.push_str(r#"<table>"#);
            html_body.push_str(r#"<tr><th>Rank</th><th>Asset</th><th>ROI</th></tr>"#);
            
            for (i, perf) in perf_data.iter().enumerate() {
                let roi_class = if perf.roi >= 0.0 {
                    "performance-positive"
                } else {
                    "performance-negative"
                };
                
                html_body.push_str(&format!(
                    r#"<tr>
                        <td>{}</td>
                        <td><strong>{}</strong></td>
                        <td class="{}">{:.2}%</td>
                    </tr>"#,
                    i + 1, perf.symbol, roi_class, perf.roi
                ));
            }
            
            html_body.push_str("</table>");
            html_body.push_str("<p><em>Note: ROI (Return on Investment) is calculated using historical data and our trading algorithm. Past performance is not indicative of future results.</em></p>");
        }

        if let Some(fgi) = &fgi_data {
            // Determine class based on value
            let fgi_class = if fgi.value <= 20 {
                "extreme-fear"
            } else if fgi.value <= 40 {
                "fear"
            } else if fgi.value <= 60 {
                "neutral"
            } else if fgi.value <= 80 {
                "greed"
            } else {
                "extreme-greed"
            };
            
            html_body.push_str(r#"<div class="section-header">Market Sentiment (Fear & Greed Index)</div>"#);
            html_body.push_str(r#"<p>The Fear & Greed Index measures market sentiment. Extreme fear can indicate buying opportunities, while extreme greed may suggest a market correction is coming.</p>"#);
            
            html_body.push_str(r#"<div class="fgi-container">"#);
            html_body.push_str(&format!(r#"<table class="fgi-table">
                <tr>
                    <td class="fgi-value {}">{}</td>
                </tr>
                <tr>
                    <td class="fgi-classification {}">{}</td>
                </tr>
                <tr>
                    <td class="fgi-timestamp">Last updated: {}</td>
                </tr>
            </table>"#, fgi_class, fgi.value, fgi_class, fgi.classification, fgi.timestamp));
            
            // Add gauge visualization (only the gradient bar, no indicator)
            html_body.push_str(r#"<div class="gauge-container"></div>
            <div class="gauge-labels">
                <span>Extreme Fear</span>
                <span>Fear</span>
                <span>Neutral</span>
                <span>Greed</span>
                <span>Extreme Greed</span>
            </div>
            </div>"#);
        }

        if let Some(ref corr_df) = correlation_data {
            html_body.push_str(r#"<div class="section-header">Correlation Analysis</div>"#);
            html_body.push_str(r#"<p>This matrix shows the correlation between different assets. Values close to 1 indicate high positive correlation, while values close to -1 indicate high negative correlation.</p>"#);
            
            let column_names = corr_df.get_column_names();
            
            html_body.push_str(r#"<table class="correlation-table">"#);
            
            html_body.push_str("<tr><th></th>");
            for name in column_names.iter() {
                html_body.push_str(&format!("<th>{}</th>", name));
            }
            html_body.push_str("</tr>");
            
            for (i, row_name) in column_names.iter().enumerate() {
                html_body.push_str(&format!("<tr><th>{}</th>", row_name));
                
                for j in 0..column_names.len() {
                    let corr_value = corr_df
                        .column(column_names[j])
                        .unwrap()
                        .f64()
                        .unwrap()
                        .get(i)
                        .unwrap_or(0.0);
                    
                    let cell_class = if i == j {
                        "correlation-neutral"
                    } else if corr_value > 0.7 {
                        "correlation-high"
                    } else if corr_value > 0.3 {
                        "correlation-medium"
                    } else if corr_value < -0.3 {
                        "correlation-low"
                    } else {
                        "correlation-neutral"
                    };
                    
                    let formatted_value = if i == j {
                        "1.00".to_string()
                    } else {
                        format!("{:.2}", corr_value)
                    };
                    
                    html_body.push_str(&format!(
                        r#"<td class="{}">{}</td>"#,
                        cell_class, formatted_value
                    ));
                }
                
                html_body.push_str("</tr>");
            }
            
            html_body.push_str("</table>");
            html_body.push_str("<p><em>Note: High positive correlation (>0.7) indicates assets that tend to move together. Negative correlation indicates assets that tend to move in opposite directions, which can be useful for portfolio diversification.</em></p>");
        }

        html_body.push_str(r#"
            </div>
            <div class="footer">
                © 2025 Seyeon Oversight - Cryptocurrency Monitoring System<br>
                This is an automated message. Please do not reply to this email.
            </div>
        </body>
        </html>
        "#);

        let mut plain_text = String::from("SEYEON OVERSIGHT - DAILY REPORT\n\n");
        plain_text.push_str(&format!("Generated at: {}\n\n", now));
        plain_text.push_str("Signal report:\n\n");
        
        for (crypto, action) in &status_list {
            plain_text.push_str(&format!("{}: {:?}\n", crypto, action));
        }

        plain_text.push_str("\nRecommendations based on technical analysis and market indicators.\n");

        if let Some(ref corr_df) = correlation_data {
            plain_text.push_str("\nCorrelation Analysis:\n");
            plain_text.push_str("This matrix shows the correlation between different assets. Values close to 1 indicate high positive correlation, while values close to -1 indicate high negative correlation.\n\n");
            
            let column_names = corr_df.get_column_names();
            
            for (i, row_name) in column_names.iter().enumerate() {
                plain_text.push_str(&format!("{}: ", row_name));
                
                for j in 0..column_names.len() {
                    let corr_value = corr_df
                        .column(column_names[j])
                        .unwrap()
                        .f64()
                        .unwrap()
                        .get(i)
                        .unwrap_or(0.0);
                    
                    let formatted_value = if i == j {
                        "1.00".to_string()
                    } else {
                        format!("{:.2}", corr_value)
                    };
                    
                    plain_text.push_str(&format!("{} ", formatted_value));
                }
                
                plain_text.push_str("\n");
            }
            
            plain_text.push_str("Note: High positive correlation (>0.7) indicates assets that tend to move together. Negative correlation indicates assets that tend to move in opposite directions, which can be useful for portfolio diversification.\n");
        }

        if let Some(perf_data) = &performance_data {
            plain_text.push_str("\nPerformance Analysis:\n");
            plain_text.push_str("This table shows the performance of your assets based on simulated trading using our algorithm:\n\n");
            
            for (i, perf) in perf_data.iter().enumerate() {
                plain_text.push_str(&format!("{}: {} - ROI: {:.2}%\n", i + 1, perf.symbol, perf.roi));
            }
            
            plain_text.push_str("Note: ROI (Return on Investment) is calculated using historical data and our trading algorithm. Past performance is not indicative of future results.\n");
        }

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(self.to_email.parse()?)
            .subject(format!("Daily Report - {}", date_today))
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(plain_text))
                    .singlepart(SinglePart::html(html_body))
            )?;

        let creds = Credentials::new(self.from_email.clone(), self.smtp_password.clone());

        let mailer = SmtpTransport::relay("smtp.gmail.com")?
            .credentials(creds)
            .build();

        mailer.send(&email)?;

        println!("Daily report sent!");

        Ok(())
    }
}