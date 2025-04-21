use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use std::str::FromStr;

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

    pub fn report_sender(
        &self,
        status_list: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut body = String::from("Seyeon report:\n\n");

        for (endpoint, status) in status_list {
            body.push_str(&format!("• {} - {}\n", endpoint, status));
        }

        let mut builder = Message::builder()
            .from(self.from_email.parse()?)
            .to(self.to_email.parse()?);

        for cc_email in &self.cc_emails {
            builder = builder.cc(Mailbox::from_str(cc_email)?);
        }

        let email = builder.subject("Seyeon Alert!").body(body)?;

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
}
