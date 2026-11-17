//! Segments: saved selections of members within a list.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example segments`
//!
//! A segment definition is in Laposta's own JSON format. The easiest way to
//! get a valid one is to build a segment in the application and read it back,
//! which is what this example does: it copies the first segment of the list.

use laposta_rs::{Client, ListId, NewSegment, UpdateSegment};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));

    let segments = client.segments().all(&list_id).await?;
    for segment in &segments {
        println!("{}: {}", segment.segment_id, segment.name);
    }
    let Some(original) = segments.first() else {
        println!("no segments in this list; create one in Laposta first");
        return Ok(());
    };

    let copy = client
        .segments()
        .create(
            &list_id,
            &NewSegment::new("Copy", original.definition.clone()),
        )
        .await?;
    let copy = client
        .segments()
        .update(
            &list_id,
            &copy.segment_id,
            &UpdateSegment::new().name("Renamed copy"),
        )
        .await?;
    println!("created {}", copy.name);

    client.segments().delete(&list_id, &copy.segment_id).await?;
    Ok(())
}
