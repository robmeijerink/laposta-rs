//! Paging through members and history, by hand and with streams.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example pagination`

use futures::TryStreamExt;
use laposta_rs::{
    Client, Cursor, HistoryQuery, Limit, ListId, MemberFilterState, MemberQuery, SortOrder,
};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));

    // By hand: one page at a time, following the cursor yourself.
    let mut cursor: Option<Cursor> = None;
    loop {
        let mut query = MemberQuery::new()
            .state(MemberFilterState::Active)
            .limit(Limit::new(100).expect("between 1 and 1000"));
        if let Some(cursor) = cursor.take() {
            query = query.cursor(cursor);
        }
        let page = client.members().list(&list_id, &query).await?;
        println!("page with {} members", page.data.len());
        match page.next_cursor() {
            Some(next) => cursor = Some(next.clone()),
            None => break,
        }
    }

    // With a stream: the cursors are followed for you.
    let mut members = client.members().stream(
        &list_id,
        MemberQuery::new()
            .order(SortOrder::Desc)
            .search("example.net"),
    );
    let mut count = 0;
    while let Some(_member) = members.try_next().await? {
        count += 1;
    }
    println!("{count} members matched");

    // History of a member has a stream as well, and so do campaigns.
    if let Some(member) = client
        .members()
        .stream(&list_id, MemberQuery::new())
        .try_next()
        .await?
    {
        let mut events =
            client
                .members()
                .history_stream(&list_id, &member.member_id, HistoryQuery::new());
        while let Some(event) = events.try_next().await? {
            println!("{} {}", event.event_date, event.kind);
        }
    }
    Ok(())
}
