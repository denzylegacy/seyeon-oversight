use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use seyeon_redis::{CryptoStatus, TradeAction};
use std::env;
use std::str::FromStr;
use chrono::Local;

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
                    color: #f39c12;
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

    pub async fn send_daily_report(&self, status_list: Vec<(String, TradeAction)>) -> Result<(), Box<dyn std::error::Error>> {
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
                    max-width: 600px;
                    margin: 0 auto;
                    padding: 20px;
                }}
                .header {{
                    background-color:rgb(54, 88, 130);
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
                    color: #f39c12;
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
                세연 SEYEON OVERSIGHT - DAILY REPORT
            </div>
            <div class="content">
                <p class="time-info">Generated at: {now}</p>
                <p>Signal report for {date_today}:</p>
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
            plain_text.push_str(&format!("• {} - {:?}\n", crypto, action));
        }
        
        plain_text.push_str("\n\n© 2025 Seyeon Oversight - Cryptocurrency Monitoring System");

        let mut builder = Message::builder()
            .from(self.from_email.parse()?)
            .to(self.to_email.parse()?);

        for cc_email in &self.cc_emails {
            builder = builder.cc(Mailbox::from_str(cc_email)?);
        }

        let email = builder
            .subject(format!("🔍 Seyeon Oversight - Daily Report {}", date_today))
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::plain(plain_text)
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
        println!("\nStatus report sent by email to {} and {} CCs!", self.to_email, self.cc_emails.len());
        
        Ok(())
    }
}
