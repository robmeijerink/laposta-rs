//! Errors: telling API failures, bad input and transport problems apart.
//!
//! `LAPOSTA_API_KEY=... cargo run --example errors`

use laposta_rs::{Client, Email, Error, ListId};

#[tokio::main]
async fn main() {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));

    // Malformed values are rejected before anything reaches the network.
    match Email::parse("not-an-email") {
        Ok(_) => unreachable!(),
        Err(e) => println!("rejected locally: {}", Error::from(e)),
    }

    match client.lists().get(&ListId::new("DoesNotExist")).await {
        Ok(list) => println!("found {}", list.name),
        Err(e) if e.is_not_found() => println!("no such list"),
        Err(e) if e.is_unauthorized() => println!("check the API key"),
        Err(e) if e.is_rate_limited() => println!("slow down, or enable `max_retries`"),
        Err(Error::Api(api)) => println!(
            "API error {} ({:?}, code {:?}) on parameter {:?}: {}",
            api.status, api.kind, api.code, api.parameter, api.message
        ),
        Err(Error::Http(e)) if e.is_timeout() => println!("timed out"),
        Err(e) => println!("other error: {e}"),
    }
}
