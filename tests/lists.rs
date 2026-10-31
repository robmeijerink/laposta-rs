mod common;

use common::{AUTH, respond, setup};
use laposta_rs::lists::ListState;
use laposta_rs::{Email, ListId, NewList, SyncAction, SyncMember, SyncMembers, UpdateList};
use wiremock::matchers::{body_json, body_string, header, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn create_sends_form_and_decodes_list() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/list"))
        .and(header("authorization", AUTH))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string(
            "locked=false&name=Testlist&remarks=A+list&subscribe_notification_email=subscription%40example.net",
        ))
        .respond_with(respond("list").set_delay(std::time::Duration::ZERO))
        .expect(1)
        .mount(&server)
        .await;

    let list = client
        .lists()
        .create(
            &NewList::new("Testlist")
                .locked(false)
                .remarks("A list")
                .subscribe_notification_email(Email::parse("subscription@example.net").unwrap()),
        )
        .await
        .unwrap();

    assert_eq!(list.list_id.as_str(), "BaImMu3JZA");
    assert_eq!(list.state, ListState::Active);
    assert_eq!(list.members.unwrap().active, 12);
    assert_eq!(list.modified.as_deref(), Some("2012-06-02 14:07:20"));
}

#[tokio::test]
async fn get_all_update_delete_purge() {
    let (server, client) = setup().await;
    let id = ListId::new("BaImMu3JZA");

    Mock::given(method("GET"))
        .and(path("/v2/list/BaImMu3JZA"))
        .respond_with(respond("list"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(respond("lists"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/list/BaImMu3JZA"))
        .and(body_string("name=Renamed"))
        .respond_with(respond("list"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/list/BaImMu3JZA"))
        .respond_with(respond("list"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/list/BaImMu3JZA/members"))
        .respond_with(respond("list"))
        .expect(1)
        .mount(&server)
        .await;

    let lists = client.lists();
    assert_eq!(lists.get(&id).await.unwrap().name, "Testlist");
    let all = lists.all().await.unwrap();
    assert_eq!(all.len(), 2);
    assert!(all[1].locked);
    lists
        .update(&id, &UpdateList::new().name("Renamed"))
        .await
        .unwrap();
    lists.delete(&id).await.unwrap();
    lists.purge_members(&id).await.unwrap();
}

#[tokio::test]
async fn sync_members_sends_json_and_decodes_report() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/list/BaImMu3JZA/members"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({
            "actions": ["add", "update"],
            "members": [
                {"email": "member3@example.org", "custom_fields": {"my_required_field": "value"}},
                {"member_id": "9978ydioiZ", "state": "unsubscribed"}
            ]
        })))
        .respond_with(respond("sync_result"))
        .expect(1)
        .mount(&server)
        .await;

    let sync = SyncMembers::new(SyncAction::Add)
        .action(SyncAction::Update)
        .action(SyncAction::Add)
        .member(
            SyncMember::new(Email::parse("member3@example.org").unwrap())
                .custom_field("my_required_field", "value"),
        )
        .member(
            SyncMember::existing(laposta_rs::MemberId::new("9978ydioiZ"))
                .state(laposta_rs::WritableMemberState::Unsubscribed),
        );
    let result = client
        .lists()
        .sync_members(&ListId::new("BaImMu3JZA"), &sync)
        .await
        .unwrap();

    assert_eq!(result.report.provided_count, 3);
    assert_eq!(result.report.added_count, 1);
    assert_eq!(
        result.errors[0].error.code,
        Some(laposta_rs::ErrorCode::ParameterEmpty)
    );
    assert_eq!(
        result.errors[0].error.parameter.as_deref(),
        Some("my_required_field")
    );
    assert_eq!(
        result.skipped[0].member.as_ref().unwrap().email,
        "member2@example.org"
    );
    assert_eq!(
        result.added[0].member.as_ref().unwrap().member_id.as_str(),
        "vnvj2cfQ3a"
    );
    assert!(result.edited.is_empty());
}

#[tokio::test]
async fn sync_members_rejects_empty_request_without_calling_the_api() {
    let (server, client) = setup().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let err = client
        .lists()
        .sync_members(
            &ListId::new("BaImMu3JZA"),
            &SyncMembers::new(SyncAction::Add),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, laposta_rs::Error::InvalidInput(_)));
}

#[tokio::test]
async fn ids_that_would_change_the_route_are_rejected_without_a_request() {
    let (server, client) = setup().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    for bad in ["", ".", "..", "a\nb"] {
        let id = ListId::new(bad);
        assert!(
            matches!(
                client.lists().get(&id).await,
                Err(laposta_rs::Error::InvalidInput(_))
            ),
            "{bad:?}"
        );
        assert!(
            matches!(
                client.lists().delete(&id).await,
                Err(laposta_rs::Error::InvalidInput(_))
            ),
            "{bad:?}"
        );
    }
}
