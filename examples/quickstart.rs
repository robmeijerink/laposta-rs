//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example quickstart`

use futures::TryStreamExt;
use laposta_rs::{Client, ListId, MemberFilterState, MemberQuery};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));

    let list = client.lists().get(&list_id).await?;
    println!("{} ({:?})", list.name, list.members.map(|m| m.active));

    let mut members = client.members().stream(
        &list_id,
        MemberQuery::new().state(MemberFilterState::Active),
    );
    while let Some(member) = members.try_next().await? {
        println!("{}", member.email);
    }
    Ok(())
}
