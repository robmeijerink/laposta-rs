//! Members: add, look up, update, inspect and remove a subscriber.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example members`
//!
//! Adds `example@example.net` to the list and deletes it again.

use std::net::Ipv4Addr;

use laposta_rs::{
    Client, Email, HistoryQuery, ListId, NewMember, UpdateMember, WritableMemberState,
};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));
    let email = Email::parse("example@example.net")?;

    // The IP address is the one the subscriber signed up from; the API requires it.
    let member = client
        .members()
        .create(
            &list_id,
            &NewMember::new(email.clone(), Ipv4Addr::new(198, 51, 100, 10))
                .custom_field("name", "Example")
                .upsert(true)
                .suppress_email_notification(true),
        )
        .await?;
    println!("{} is {:?}", member.email, member.state);

    // A member can be addressed by id or by email address.
    let member = client.members().get(&list_id, &email).await?;
    let member = client.members().get(&list_id, member.member_id).await?;

    let member = client
        .members()
        .update(
            &list_id,
            &member.member_id,
            &UpdateMember::new()
                .custom_field("name", "Renamed")
                .state(WritableMemberState::Active),
        )
        .await?;

    let history = client
        .members()
        .history(&list_id, &member.member_id, &HistoryQuery::new())
        .await?;
    for event in history.events {
        println!("{} {}", event.event_date, event.kind);
    }

    client.members().delete(&list_id, &email).await?;
    Ok(())
}
