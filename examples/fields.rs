//! Fields: the custom data stored with each member.
//!
//! `LAPOSTA_API_KEY=... LAPOSTA_LIST_ID=... cargo run --example fields`
//!
//! Creates two fields in the list and deletes them again.

use laposta_rs::fields::{DisplayType, FieldOption, NewFieldKind, UpdateField};
use laposta_rs::{Client, ListId, NewField};

#[tokio::main]
async fn main() -> laposta_rs::Result<()> {
    let client = Client::new(std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY"));
    let list_id = ListId::new(std::env::var("LAPOSTA_LIST_ID").expect("LAPOSTA_LIST_ID"));

    // A select field carries its options, so it cannot be created without them.
    let color = client
        .fields()
        .create(
            &list_id,
            &NewField::new(
                "Favourite colour",
                NewFieldKind::SelectSingle {
                    options: vec!["Red".into(), "Green".into(), "Blue".into()],
                    display: Some(DisplayType::Radio),
                },
            )
            .required(false),
        )
        .await?;
    // `custom_name` is the key to use in `custom_field(..)` on members.
    println!(
        "created {} (use `{}` on members)",
        color.name, color.custom_name
    );

    let city = client
        .fields()
        .create(
            &list_id,
            &NewField::new("City", NewFieldKind::Text).in_list(false),
        )
        .await?;

    // A numeric id refers to an existing option; any other id adds a new one.
    let mut options: Vec<FieldOption> = color.options_full.clone();
    options.push(FieldOption::new("new-0", "Yellow"));
    client
        .fields()
        .update(
            &list_id,
            &color.field_id,
            &UpdateField::new().options_full(options),
        )
        .await?;

    for field in client.fields().all(&list_id).await? {
        println!("{}: {} ({:?})", field.field_id, field.name, field.datatype);
    }

    client.fields().delete(&list_id, &color.field_id).await?;
    client.fields().delete(&list_id, &city.field_id).await?;
    Ok(())
}
