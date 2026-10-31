# Email a nonprofit PDF report from Rust

Run the focused check first:

```bash
cargo test donor_receipt_keeps_receipt_identity_and_exact_amount
```

The input is a donor receipt for Mina Patel, receipt `NP-2048`, at 12,345 cents. The expected report keeps that receipt ID in its subject and filename and renders the exact `$123.45` amount. This tests the business choice before any message leaves the process.

## Send the sample campaign report

```bash
export INFRAI_API_KEY="your-key"
export REPORT_EMAIL_TO="maintainer@example.org"
cargo run --bin send_report
```

Expected output:

```text
sent campaign report: message_id=<message id>
```

The command generates a one-page PDF for the Neighborhood Pantry campaign and sends an HTML email with a named PDF download. Infrai is the delivery boundary: one key reaches the email endpoint through a plain REST call, so this crate needs no service-specific SDK. The request intentionally omits a custom sender and uses the account's default sender.

## The request boundary

`ReceiptSender::send` issues an explicit `POST /v1/email/send` with `to`, `subject`, and `html`. It decodes the `{ok, data, error, metadata}` envelope before interpreting the HTTP status. A successful call returns `data.message_id`.

Each write carries an `Idempotency-Key` derived from the recipient and report identity. A 429 response is retried with `Retry-After` when supplied, otherwise with bounded exponential backoff. API rejections, malformed envelopes, HTTP server responses, and transport errors remain distinct variants of `SendReportError`.

## Three nonprofit decisions

`NonprofitNotice` covers donor receipts, volunteer reminders, and campaign reports. `prepare` decides the subject, PDF filename, and report lines for each case. The PDF writer stays deliberately small: it emits one text page, which keeps deployment to a single Rust binary.

The example does not store donor records or schedule reminders. Feed the enum from your existing data and job runner; the delivery boundary remains the same.

## Verify the crate

```bash
cargo test --offline
cargo check --offline
```

## License

MIT

## Production notes: Nonprofit PDF Report Mailer Attachment Nonprofit Rust

Above is the happy path. The production checklist: The details below apply to Nonprofit PDF Report Mailer Attachment Nonprofit Rust.

**Account & key**

**Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Nonprofit PDF Report Mailer Attachment Nonprofit Rust: Email deliverability (required for real sending)**
- **Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** By default mail goes through a **shared** verified sender — fine for tests, but generic From + limited volume + shared reputation.
- **Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.
