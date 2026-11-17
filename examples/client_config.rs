//! Configuring the client: retries, timeouts and a custom HTTP client.
//!
//! `LAPOSTA_API_KEY=... cargo run --example client_config`

use std::time::Duration;

use laposta_rs::Client;

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let api_key = std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY");

    // Retry after HTTP 429, waiting for the `Retry-After` duration (at most 60 seconds).
    // Large bulk syncs may need a longer timeout than the default 60 seconds.
    let client = Client::builder()
        .api_key(api_key)
        .max_retries(3)
        .timeout(Duration::from_secs(120))
        .build()?;

    let lists = client.lists().all().await?;
    println!("{} lists", lists.len());
    Ok(())
}
