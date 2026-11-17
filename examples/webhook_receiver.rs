//! Webhook receiver: verify the signature, then parse the payload.
//!
//! `cargo run --example webhook_receiver`
//!
//! Needs the `webhook` feature (on by default). Call `handle` from your HTTP
//! framework with the raw request body and the `X-Laposta-Signature` header;
//! `main` only feeds it a request with a wrong signature.

use laposta_rs::webhooks::{parse_payload, verify_signature};

fn handle(body: &[u8], signature_header: &str, secret: &str) -> laposta_rs::Result<bool> {
    // Verify against the raw bytes, before any parsing or re-serialising.
    if !verify_signature(body, signature_header, secret) {
        return Ok(false);
    }

    let payload = parse_payload(body)?;
    // A valid signature does not rule out a replay: check `date_requested` if it matters.
    println!("requested at {}", payload.date_requested);
    for notification in payload.data {
        println!(
            "{:?} {} ({} via {:?})",
            notification.event,
            notification.data.email,
            notification.info.action,
            notification.info.source,
        );
    }
    Ok(true)
}

fn main() -> laposta_rs::Result<()> {
    let accepted = handle(
        br#"{"data":[],"date_requested":"2030-01-01 09:00:00"}"#,
        "sha256=00",
        "secret",
    )?;
    println!("accepted: {accepted}");
    Ok(())
}
