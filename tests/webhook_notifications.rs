#![cfg(feature = "webhook")]

mod common;

use common::fixture;
use laposta_rs::WebhookEvent;
use laposta_rs::webhooks::WebhookSource;

#[test]
fn incoming_notification_is_verified_and_parsed() {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let body = fixture("webhook_payload");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"s3cret").unwrap();
    mac.update(body.as_bytes());
    let header = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

    assert!(laposta_rs::webhooks::verify_signature(
        body.as_bytes(),
        &header,
        "s3cret"
    ));
    assert!(!laposta_rs::webhooks::verify_signature(
        body.as_bytes(),
        &header,
        "wrong"
    ));

    let payload = laposta_rs::webhooks::parse_payload(body.as_bytes()).unwrap();
    let notification = &payload.data[0];
    assert_eq!(notification.event, WebhookEvent::Deactivated);
    assert_eq!(notification.info.source, WebhookSource::App);
    assert_eq!(notification.info.action, "unsubscribed");
    assert_eq!(notification.data.email, "maartje@example.net");
}

#[test]
fn empty_secret_never_verifies() {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(b"").unwrap();
    mac.update(b"{}");
    let header = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

    assert!(!laposta_rs::webhooks::verify_signature(b"{}", &header, ""));
}
