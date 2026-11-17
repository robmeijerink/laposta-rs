//! Campaigns: create, fill, test and (optionally) send.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... LAPOSTA_SENDER_EMAIL=... cargo run --example campaigns`
//!
//! `LAPOSTA_SENDER_EMAIL` must be an approved sender in the account. The
//! example sends a test mail to that address and deletes the campaign; it
//! never sends to the list.

use laposta_rs::campaigns::CampaignContent;
use laposta_rs::{Client, Email, ListId, NewCampaign, Sender};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));
    let sender =
        Email::parse(std::env::var("LAPOSTA_SENDER_EMAIL").expect("LAPOSTA_SENDER_EMAIL"))?;

    // A campaign needs at least one audience: a list, or a segment of a list.
    let campaign = client
        .campaigns()
        .create(
            &NewCampaign::new(
                "laposta-rs example",
                "Hello from Rust",
                Sender::new("Example", sender.clone()),
                list_id,
            )
            .previewtext("A short preview")
            .google_analytics(false),
        )
        .await?;
    println!("created {}", campaign.campaign_id);

    client
        .campaigns()
        .set_content(
            &campaign.campaign_id,
            &CampaignContent::html("<h1>Hello</h1><p>Sent through laposta-rs.</p>")
                .inline_css(true),
        )
        .await?;

    client
        .campaigns()
        .send_test(&campaign.campaign_id, &sender)
        .await?;

    // To really deliver the campaign:
    //
    //     client.campaigns().send(&campaign.campaign_id).await?;
    //
    // or later, with a timestamp in the account timezone:
    //
    //     let when = laposta_rs::Timestamp::parse("2030-01-01T09:00:00+01:00")?;
    //     client.campaigns().schedule(&campaign.campaign_id, &when).await?;

    client.campaigns().delete(&campaign.campaign_id).await?;
    Ok(())
}
