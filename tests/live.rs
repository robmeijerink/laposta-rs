//! Runs against the real API. Needs `LAPOSTA_API_KEY`:
//! `LAPOSTA_API_KEY=... cargo test --test live -- --ignored`

use laposta_rs::{
    Client, Email, NewField, NewFieldKind, NewList, NewMember, UpdateMember, WritableMemberState,
};

#[tokio::test]
#[ignore = "talks to the real Laposta API"]
async fn list_field_member_round_trip() {
    let key = std::env::var("LAPOSTA_API_KEY").expect("LAPOSTA_API_KEY must be set");
    let client = Client::new(key);

    let list = client
        .lists()
        .create(&NewList::new("laposta-rs live test"))
        .await
        .unwrap();
    let result = async {
        let field = client
            .fields()
            .create(
                &list.list_id,
                &NewField::new("Nickname", NewFieldKind::Text),
            )
            .await?;
        let email = Email::parse("live-test@example.net").unwrap();
        let member = client
            .members()
            .create(
                &list.list_id,
                &NewMember::new(email.clone(), "198.51.100.10".parse().unwrap())
                    .custom_field(field.custom_name.clone(), "Tester")
                    .ignore_doubleoptin(true),
            )
            .await?;
        assert_eq!(member.custom_fields[&field.custom_name], "Tester");

        let fetched = client.members().get(&list.list_id, &email).await?;
        assert_eq!(fetched.member_id, member.member_id);

        let updated = client
            .members()
            .update(
                &list.list_id,
                &member.member_id,
                &UpdateMember::new().state(WritableMemberState::Unsubscribed),
            )
            .await?;
        assert_eq!(updated.state, laposta_rs::MemberState::Unsubscribed);
        Ok::<_, laposta_rs::Error>(())
    }
    .await;

    client.lists().delete(&list.list_id).await.unwrap();
    result.unwrap();
}
