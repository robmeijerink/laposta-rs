//! Lists: create, read, update and delete.
//!
//! `LAPOSTA_API_KEY=... cargo run --example lists`
//!
//! Creates a throwaway list and deletes it again.

use laposta_rs::{Client, Email, NewList, UpdateList};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));

    let list = client
        .lists()
        .create(
            &NewList::new("laposta-rs example")
                .remarks("Created by the lists example")
                .subscribe_notification_email(Email::parse("notify@example.net")?),
        )
        .await?;
    println!("created {} ({})", list.name, list.list_id);

    let list = client
        .lists()
        .update(
            &list.list_id,
            &UpdateList::new().name("laposta-rs example (renamed)"),
        )
        .await?;
    println!("renamed to {}", list.name);

    for list in client.lists().all().await? {
        println!("{}: {}", list.list_id, list.name);
    }

    // `purge_members` removes every member but keeps the list.
    client.lists().delete(&list.list_id).await?;
    Ok(())
}
