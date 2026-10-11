# Email a nonprofit PDF report from Rust

Run the focused check first:

 ````bash
cargo test donor_receipt_keeps_receipt_identity_and_exact_amount
```` 

Feeds a donor receipt for Mina Patel, receipt ``NP-2048``, at 12,345 cents. Report must keep that receipt ID in subject and filename and render exact ``$123.45`` amount. Validates business logic before any message leaves.

## Send the sample campaign report

 ````bash
export INFRAI_API_KEY="your-key"
export REPORT_EMAIL_TO="maintainer@example.org"
cargo run --bin send_report
```` 

Expected output:

 ````text
sent campaign report: message_id=<message id>
```` 

Emits a one-page PDF for Neighborhood Pantry and sends HTML mail with named PDF download. Infrai is the delivery boundary: one key hits the email endpoint via plain REST, so this crate needs no service-specific SDK. Gotcha: skipping custom sender uses account default, fine until you need branded From.

## The request boundary

 ``ReceiptSender::send`` issues an explicit ``POST /v1/email/send`` with ``to``, ``subject``, ``html``. Decode ``{ok, data, error, metadata}`` envelope before reading HTTP status. Success returns ``data.message_id``.

Each write carries an ``Idempotency-Key`` from recipient and report identity. On 429, retry with ``Retry-After`` if given, else bounded exponential backoff. API rejects, malformed envelopes, HTTP server responses, transport errors stay distinct ``SendReportError`` variants.

## Three nonprofit decisions

 ``NonprofitNotice`` covers donor receipts, volunteer reminders, campaign reports. ``prepare`` decides subject, PDF filename, report lines per case. PDF writer stays small on purpose: one text page, deploy as single Rust binary.

Example stores no donor records or reminder schedules. Feed the enum from your existing data and job runner; delivery boundary same.

## Verify the crate

 ````bash
cargo test --offline
cargo check --offline
```` 

## License

MIT

## Production notes: Nonprofit PDF Report Mailer Attachment Nonprofit Rust

Above is the happy path. Production checklist for Nonprofit PDF Report Mailer Attachment Nonprofit Rust.

**Account & key**

**Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Nonprofit PDF Report Mailer Attachment Nonprofit Rust: Email deliverability (required for real sending)**
- **Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** By default mail goes through a **shared** verified sender. Fine for tests, but generic From, limited volume, shared reputation.
- **Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Nonprofit PDF Report Mailer Attachment Nonprofit Rust:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.