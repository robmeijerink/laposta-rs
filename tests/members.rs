mod common;

use common::{respond, setup};
use futures::TryStreamExt;
use laposta_rs::members::MemberOrder;
use laposta_rs::{
    Cursor, Email, HistoryQuery, Limit, ListId, MemberFilterState, MemberId, MemberQuery,
    MemberState, NewMember, SortOrder, Timestamp, UpdateMember, WritableMemberState,
};
use wiremock::Mock;
use wiremock::matchers::{body_string, method, path, query_param, query_param_is_missing};

fn list_id() -> ListId {
    ListId::new("BaImMu3JZA")
}

#[tokio::test]
async fn create_encodes_nested_fields_and_options() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/member"))
        .and(body_string(
            "custom_fields%5Bname%5D=Maartje&custom_fields%5Bprefs%5D%5B%5D=a&custom_fields%5Bprefs%5D%5B%5D=b&email=maartje%40example.net&ip=198.51.100.10&list_id=BaImMu3JZA&options%5Bspam_check%5D=true&options%5Bupsert%5D=true",
        ))
        .respond_with(respond("member"))
        .expect(1)
        .mount(&server)
        .await;

    let member = client
        .members()
        .create(
            &list_id(),
            &NewMember::new(
                Email::parse("maartje@example.net").unwrap(),
                std::net::Ipv4Addr::new(198, 51, 100, 10),
            )
            .custom_field("name", "Maartje")
            .custom_field("prefs", vec!["a", "b"])
            .upsert(true)
            .spam_check(true),
        )
        .await
        .unwrap();

    assert_eq!(member.member_id.as_str(), "9978ydioiZ");
    assert_eq!(member.state, MemberState::Active);
    assert_eq!(member.custom_fields["children"], 2);
    assert_eq!(member.custom_fields["labels"][1], "Newsletter");
    assert_eq!(member.confirm_date, None);
}

#[tokio::test]
async fn get_by_email_escapes_plus_twice() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member/a%252Bb@example.net"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("member"))
        .expect(1)
        .mount(&server)
        .await;

    client
        .members()
        .get(&list_id(), Email::parse("a+b@example.net").unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn update_and_delete_by_id() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/member/9978ydioiZ"))
        .and(body_string(
            "custom_fields%5Blabels%5D=&list_id=BaImMu3JZA&state=unsubscribed",
        ))
        .respond_with(respond("member"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/member/9978ydioiZ"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("member"))
        .expect(1)
        .mount(&server)
        .await;

    let id = MemberId::new("9978ydioiZ");
    client
        .members()
        .update(
            &list_id(),
            &id,
            &UpdateMember::new()
                .state(WritableMemberState::Unsubscribed)
                .custom_field("labels", Vec::<String>::new()),
        )
        .await
        .unwrap();
    client.members().delete(&list_id(), &id).await.unwrap();
}

#[tokio::test]
async fn list_without_paging_sends_only_filters() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .and(query_param("state", "active"))
        .and(query_param("order_by", "email,signup_date"))
        .and(query_param("order", "desc"))
        .and(query_param("search", "maartje"))
        .and(query_param("label", "Customer"))
        .and(query_param_is_missing("limit"))
        .and(query_param_is_missing("cursor"))
        .respond_with(respond("members_page3"))
        .expect(1)
        .mount(&server)
        .await;

    let query = MemberQuery::new()
        .state(MemberFilterState::Active)
        .order_by(MemberOrder::Email)
        .order_by(MemberOrder::SignupDate)
        .order(SortOrder::Desc)
        .search("maartje")
        .label("Customer");
    let page = client.members().list(&list_id(), &query).await.unwrap();
    assert_eq!(page.data.len(), 2);
    assert!(page.next_cursor().is_none());
}

#[tokio::test]
async fn list_rejects_overlong_search() {
    let (_server, client) = setup().await;
    let err = client
        .members()
        .list(&list_id(), &MemberQuery::new().search("x".repeat(256)))
        .await
        .unwrap_err();
    assert!(matches!(err, laposta_rs::Error::InvalidInput(_)));
}

#[tokio::test]
async fn stream_follows_cursors_across_pages() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member"))
        .and(query_param("limit", "1000"))
        .and(query_param_is_missing("cursor"))
        .respond_with(respond("members_page1"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/member"))
        .and(query_param("limit", "1000"))
        .and(query_param("cursor", "cursor1"))
        .respond_with(respond("members_page2"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/member"))
        .and(query_param("limit", "1000"))
        .and(query_param("cursor", "cursor2"))
        .respond_with(respond("members_page3"))
        .expect(1)
        .mount(&server)
        .await;

    let ids: Vec<String> = client
        .members()
        .stream(&list_id(), MemberQuery::new())
        .map_ok(|m| m.member_id.to_string())
        .try_collect()
        .await
        .unwrap();
    assert_eq!(ids, ["m1a", "m1b", "m2a", "m2b", "m3a", "m3b"]);
}

#[tokio::test]
async fn paged_list_exposes_next_cursor() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member"))
        .and(query_param("limit", "2"))
        .respond_with(respond("members_page1"))
        .expect(1)
        .mount(&server)
        .await;

    let page = client
        .members()
        .list(
            &list_id(),
            &MemberQuery::new().limit(Limit::new(2).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(page.next_cursor(), Some(&Cursor::new("cursor1")));
    assert_eq!(page.pagination.unwrap().limit, 2);
}

#[tokio::test]
async fn history_applies_filters_and_decodes_events() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member/9978ydioiZ/history"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .and(query_param("campaign_id", "pbrqulw2tc"))
        .and(query_param("event_date_from", "2026-01-01 00:00:00"))
        .and(query_param("order", "asc"))
        .respond_with(respond("history_page1"))
        .expect(1)
        .mount(&server)
        .await;

    let history = client
        .members()
        .history(
            &list_id(),
            MemberId::new("9978ydioiZ"),
            &HistoryQuery::new()
                .campaign("pbrqulw2tc".into())
                .from(Timestamp::parse("2026-01-01 00:00:00").unwrap())
                .order(SortOrder::Asc),
        )
        .await
        .unwrap();

    assert_eq!(history.events.len(), 2);
    assert_eq!(history.events[0].kind, "opened");
    assert_eq!(
        history.events[0].campaign.as_ref().unwrap().name,
        "Test via API"
    );
    assert!(history.events[1].campaign.is_none());
    assert!(history.pagination.has_more);
    assert_eq!(history.next_cursor().unwrap().as_str(), "abc+def");
}

#[tokio::test]
async fn history_stream_passes_cursor_unchanged() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member/9978ydioiZ/history"))
        .and(query_param_is_missing("cursor"))
        .respond_with(respond("history_page1"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/member/9978ydioiZ/history"))
        .and(query_param("cursor", "abc+def"))
        .respond_with(respond("history_page2"))
        .expect(1)
        .mount(&server)
        .await;

    let kinds: Vec<String> = client
        .members()
        .history_stream(&list_id(), MemberId::new("9978ydioiZ"), HistoryQuery::new())
        .map_ok(|e| e.kind)
        .try_collect()
        .await
        .unwrap();
    assert_eq!(kinds, ["opened", "subscribed", "unsubscribed"]);
}

#[tokio::test]
async fn member_id_dot_is_rejected_without_a_request() {
    let (server, client) = setup().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let err = client
        .members()
        .get(&list_id(), MemberId::new("."))
        .await
        .unwrap_err();
    assert!(matches!(err, laposta_rs::Error::InvalidInput(_)));
}

#[tokio::test]
async fn stream_stops_when_the_cursor_does_not_advance() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member"))
        .respond_with(respond("members_page1"))
        .expect(2)
        .mount(&server)
        .await;

    let result: Result<Vec<_>, _> = client
        .members()
        .stream(&list_id(), MemberQuery::new())
        .try_collect()
        .await;
    assert!(matches!(result, Err(laposta_rs::Error::Decode { .. })));
}

#[tokio::test]
async fn php_style_empty_array_is_an_empty_map() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/member/9978ydioiZ"))
        .respond_with(respond("member_empty_fields"))
        .mount(&server)
        .await;

    let member = client
        .members()
        .get(&list_id(), MemberId::new("9978ydioiZ"))
        .await
        .unwrap();
    assert!(member.custom_fields.is_empty());
}

#[tokio::test]
async fn ipv6_addresses_are_accepted() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/member"))
        .and(wiremock::matchers::body_string_contains(
            "ip=2001%3Adb8%3A%3A1",
        ))
        .respond_with(respond("member"))
        .expect(1)
        .mount(&server)
        .await;

    let ip: std::net::IpAddr = "2001:db8::1".parse().unwrap();
    client
        .members()
        .create(
            &list_id(),
            &NewMember::new(Email::parse("maartje@example.net").unwrap(), ip),
        )
        .await
        .unwrap();
}
