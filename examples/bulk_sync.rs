//! Syncing many members in one request.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example bulk_sync`
//!
//! Adds or updates two members. It does not use `UnsubscribeExcluded`, which
//! would unsubscribe everyone who is not part of the request.

use laposta_rs::{Client, Email, ListId, SyncAction, SyncMember, SyncMembers, WritableMemberState};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));

    let sync = SyncMembers::new(SyncAction::Add)
        .action(SyncAction::Update)
        .member(SyncMember::new(Email::parse("sync-a@example.net")?).custom_field("name", "A"))
        .member(
            SyncMember::new(Email::parse("sync-b@example.net")?)
                .state(WritableMemberState::Unsubscribed),
        );

    let result = client.lists().sync_members(&list_id, &sync).await?;
    println!("{:?}", result.report);
    for failure in result.errors {
        eprintln!("failed: {failure:?}");
    }
    Ok(())
}
