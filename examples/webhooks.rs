//! Webhooks: register a URL that Laposta calls when members change.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example webhooks`
//!
//! Registers a blocked webhook, so nothing is delivered, and deletes it again.
//! See the `webhook_receiver` example for handling the incoming requests.

use laposta_rs::{Client, ListId, NewWebhook, UpdateWebhook, WebhookEvent};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));

    let url = "https://example.net/hooks/laposta"
        .parse()
        .expect("valid url");
    let webhook = client
        .webhooks()
        .create(
            &list_id,
            &NewWebhook::new(WebhookEvent::Subscribed, url).blocked(true),
        )
        .await?;
    println!("created {}", webhook.webhook_id);

    // Keep the secret: it is what `verify_signature` needs.
    println!("has secret: {}", webhook.secret.is_some());

    client
        .webhooks()
        .update(
            &list_id,
            &webhook.webhook_id,
            &UpdateWebhook::new().event(WebhookEvent::Deactivated),
        )
        .await?;

    for webhook in client.webhooks().all(&list_id).await? {
        println!("{:?} -> {}", webhook.event, webhook.url);
    }

    client
        .webhooks()
        .delete(&list_id, &webhook.webhook_id)
        .await?;
    Ok(())
}
