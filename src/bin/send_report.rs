use nonprofit_report_mailer::receipt_sender::{NonprofitNotice, ReceiptSender};
use std::{env, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(message_id) => {
            println!("sent campaign report: message_id={message_id}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<String, Box<dyn std::error::Error>> {
    let to = env::var("REPORT_EMAIL_TO")?;
    let sender = ReceiptSender::from_env()?;
    let notice = NonprofitNotice::CampaignReport {
        campaign_name: "Neighborhood Pantry".to_owned(),
        donations_cents: 248_650,
        volunteer_hours: 91,
    };
    Ok(sender.send(&to, &notice).await?)
}
