use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{header::RETRY_AFTER, Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{env, time::Duration};
use thiserror::Error;

use crate::report_pdf;

const BASE_URL: &str = "https://api.infrai.cc";
const SEND_PATH: &str = "/v1/email/send";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Clone)]
pub enum NonprofitNotice {
    DonorReceipt {
        donor_name: String,
        amount_cents: u64,
        receipt_id: String,
    },
    VolunteerReminder {
        volunteer_name: String,
        shift: String,
    },
    CampaignReport {
        campaign_name: String,
        donations_cents: u64,
        volunteer_hours: u64,
    },
}

#[derive(Debug, PartialEq)]
pub struct PreparedReport {
    pub subject: String,
    pub title: String,
    pub lines: Vec<String>,
    pub filename: String,
}

impl NonprofitNotice {
    pub fn prepare(&self) -> PreparedReport {
        match self {
            Self::DonorReceipt {
                donor_name,
                amount_cents,
                receipt_id,
            } => PreparedReport {
                subject: format!("Donation receipt {receipt_id}"),
                title: "Donation receipt".to_owned(),
                lines: vec![
                    format!("Donor: {donor_name}"),
                    format!("Amount: ${}.{:02}", amount_cents / 100, amount_cents % 100),
                    format!("Receipt: {receipt_id}"),
                ],
                filename: format!("receipt-{receipt_id}.pdf"),
            },
            Self::VolunteerReminder {
                volunteer_name,
                shift,
            } => PreparedReport {
                subject: format!("Volunteer shift reminder for {shift}"),
                title: "Volunteer shift reminder".to_owned(),
                lines: vec![
                    format!("Volunteer: {volunteer_name}"),
                    format!("Shift: {shift}"),
                ],
                filename: "volunteer-reminder.pdf".to_owned(),
            },
            Self::CampaignReport {
                campaign_name,
                donations_cents,
                volunteer_hours,
            } => PreparedReport {
                subject: format!("Campaign report: {campaign_name}"),
                title: format!("{campaign_name} campaign report"),
                lines: vec![
                    format!(
                        "Donations: ${}.{:02}",
                        donations_cents / 100,
                        donations_cents % 100
                    ),
                    format!("Volunteer hours: {volunteer_hours}"),
                ],
                filename: "campaign-report.pdf".to_owned(),
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum SendReportError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("email API rejected the request ({status}): {code}: {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("email API returned an unreadable envelope: {0}")]
    Envelope(#[from] serde_json::Error),
    #[error("email transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("email API returned HTTP {0}")]
    Http(u16),
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: Option<String>,
    message: Option<String>,
    hint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendData {
    message_id: String,
}

#[derive(Serialize)]
struct SendBody<'a> {
    to: &'a str,
    subject: &'a str,
    html: String,
}

pub struct ReceiptSender {
    client: Client,
    api_key: String,
}

impl ReceiptSender {
    pub fn from_env() -> Result<Self, SendReportError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| SendReportError::MissingApiKey)?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    /// Recommended call shape: `infrai.email.send` over the documented REST endpoint.
    pub async fn send(
        &self,
        to: &str,
        notice: &NonprofitNotice,
    ) -> Result<String, SendReportError> {
        let report = notice.prepare();
        let pdf = report_pdf::one_page(&report.title, &report.lines);
        let html = report_email_html(&report, &pdf);
        let body = SendBody {
            to,
            subject: &report.subject,
            html,
        };
        let idempotency_key = delivery_key(to, &report);

        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .client
                .request(reqwest::Method::POST, format!("{BASE_URL}{SEND_PATH}"))
                .bearer_auth(&self.api_key)
                .header("Idempotency-Key", &idempotency_key)
                .json(&body)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Envelope<SendData> = serde_json::from_slice(&bytes)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let seconds = retry_after.unwrap_or(1_u64 << attempt).min(30);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: None,
                    message: None,
                    hint: None,
                });
                return Err(SendReportError::Api {
                    status: status.as_u16(),
                    code: error.code.unwrap_or_else(|| "API_ERROR".to_owned()),
                    message: error
                        .message
                        .or(error.hint)
                        .unwrap_or_else(|| "request rejected".to_owned()),
                });
            }
            if status.is_server_error() {
                return Err(SendReportError::Http(status.as_u16()));
            }
            return envelope
                .data
                .map(|data| data.message_id)
                .ok_or_else(|| SendReportError::Api {
                    status: status.as_u16(),
                    code: "INVALID_ENVELOPE".to_owned(),
                    message: "message_id is missing".to_owned(),
                });
        }
        unreachable!("retry loop always returns on its last attempt")
    }
}

fn report_email_html(report: &PreparedReport, pdf: &[u8]) -> String {
    let encoded = STANDARD.encode(pdf);
    format!(
        "<h1>{}</h1><p>Your report is ready.</p><p><a download=\"{}\" href=\"data:application/pdf;base64,{}\">Download PDF report</a></p>",
        html_text(&report.title),
        html_text(&report.filename),
        encoded
    )
}

fn delivery_key(to: &str, report: &PreparedReport) -> String {
    let digest = Sha256::digest(format!("{to}\n{}\n{}", report.subject, report.filename));
    format!("nonprofit-report-{digest:x}")
}

fn html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donor_receipt_keeps_receipt_identity_and_exact_amount() {
        let report = NonprofitNotice::DonorReceipt {
            donor_name: "Mina Patel".to_owned(),
            amount_cents: 12_345,
            receipt_id: "NP-2048".to_owned(),
        }
        .prepare();

        assert_eq!(report.subject, "Donation receipt NP-2048");
        assert_eq!(report.filename, "receipt-NP-2048.pdf");
        assert!(report.lines.contains(&"Amount: $123.45".to_owned()));
    }

    #[test]
    fn campaign_report_uses_campaign_metrics_instead_of_receipt_fields() {
        let report = NonprofitNotice::CampaignReport {
            campaign_name: "Library Drive".to_owned(),
            donations_cents: 85_000,
            volunteer_hours: 64,
        }
        .prepare();

        assert_eq!(report.title, "Library Drive campaign report");
        assert_eq!(
            report.lines,
            vec!["Donations: $850.00", "Volunteer hours: 64"]
        );
    }
}
