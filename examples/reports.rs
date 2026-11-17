//! Reports: the results of campaigns that were sent.
//!
//! `LAPOSTA_API_KEY=... cargo run --example reports`

use futures::TryStreamExt;
use laposta_rs::{Client, HistoryQuery};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));

    let reports = client.reports().all().await?;
    for report in &reports {
        println!(
            "{}: sent {}, opened {:.1}%, clicked {:.1}%",
            report.campaign_id,
            report.sent,
            report.opened_ratio * 100.0,
            report.clicked_ratio * 100.0,
        );
    }

    // Details of one campaign, including everything that happened after sending.
    if let Some(first) = reports.first() {
        let report = client.reports().get(&first.campaign_id).await?;
        println!("{} unsubscribed", report.unsubscribed);

        let mut events = client
            .campaigns()
            .history_stream(&first.campaign_id, HistoryQuery::new());
        while let Some(event) = events.try_next().await? {
            println!("{} {}", event.event_date, event.kind);
        }
    }
    Ok(())
}
