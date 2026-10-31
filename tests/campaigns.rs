mod common;

use common::{respond, setup};
use futures::TryStreamExt;
use laposta_rs::{
    Audience, CampaignContent, CampaignId, Email, HistoryQuery, ListId, NewCampaign, SegmentId,
    Sender, Timestamp, UpdateCampaign,
};
use wiremock::Mock;
use wiremock::matchers::{body_string, method, path, query_param};

fn id() -> CampaignId {
    CampaignId::new("pbrqulw2tc")
}

#[tokio::test]
async fn create_sends_nested_sender_and_audiences() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign"))
        .and(body_string(
            "from%5Bemail%5D=max%40example.net&from%5Bname%5D=Max&list_ids%5B%5D=nnhnkrytua&list_ids%5B%5D=seg1&name=Test&previewtext=Preview&reply_to=reply%40example.net&stats%5Bga%5D=true&subject=Hello&type=regular",
        ))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;

    let campaign = client
        .campaigns()
        .create(
            &NewCampaign::new(
                "Test",
                "Hello",
                Sender::new("Max", Email::parse("max@example.net").unwrap()),
                ListId::new("nnhnkrytua"),
            )
            .also_to(Audience::Segment(SegmentId::new("seg1")))
            .previewtext("Preview")
            .reply_to(Email::parse("reply@example.net").unwrap())
            .google_analytics(true),
        )
        .await
        .unwrap();

    assert_eq!(campaign.campaign_id, id());
    assert_eq!(campaign.kind, "regular");
    assert_eq!(campaign.from.email, "max@example.net");
    assert_eq!(
        campaign.list_ids[&ListId::new("kxhysrt26s")][0].as_str(),
        "2noVadDLbd"
    );
    assert!(campaign.list_ids[&ListId::new("shb5vujyxj")].is_empty());
    assert_eq!(campaign.screenshot.unwrap().len(), 2);
    assert!(campaign.delivery_requested.is_none());
}

#[tokio::test]
async fn create_validates_lengths_before_sending() {
    let (_server, client) = setup().await;
    let campaign = NewCampaign::new(
        "n",
        "s",
        Sender::new("Max", Email::parse("max@example.net").unwrap()),
        ListId::new("l"),
    )
    .previewtext("x".repeat(121));
    let err = client.campaigns().create(&campaign).await.unwrap_err();
    assert!(matches!(err, laposta_rs::Error::InvalidInput(_)));
}

#[tokio::test]
async fn get_all_update_delete() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/campaign/pbrqulw2tc"))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/campaign"))
        .respond_with(respond("campaigns"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/pbrqulw2tc"))
        .and(body_string("previewtext="))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/campaign/pbrqulw2tc"))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;

    let campaigns = client.campaigns();
    campaigns.get(&id()).await.unwrap();
    assert_eq!(campaigns.all().await.unwrap()[0].name, "One");
    campaigns
        .update(&id(), &UpdateCampaign::new().previewtext(""))
        .await
        .unwrap();
    campaigns.delete(&id()).await.unwrap();
}

#[tokio::test]
async fn content_html_and_import_url() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/campaign/pbrqulw2tc/content"))
        .respond_with(respond("campaign_content"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/pbrqulw2tc/content"))
        .and(body_string("html=%3Cp%3EHi%3C%2Fp%3E&inline_css=true"))
        .respond_with(respond("campaign_content"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/pbrqulw2tc/content"))
        .and(body_string(
            "import_url=https%3A%2F%2Fexample.net%2Fnewsletter",
        ))
        .respond_with(respond("campaign_content"))
        .expect(1)
        .mount(&server)
        .await;

    let campaigns = client.campaigns();
    let content = campaigns.content(&id()).await.unwrap();
    assert_eq!(content.html.as_deref(), Some("<p>Hello</p>"));

    let result = campaigns
        .set_content(&id(), &CampaignContent::html("<p>Hi</p>").inline_css(true))
        .await
        .unwrap();
    let report = result.report.unwrap();
    assert!(report.javascript && report.no_unsubscribe);
    assert!(!report.flash);

    campaigns
        .set_content(
            &id(),
            &CampaignContent::import_url("https://example.net/newsletter".parse().unwrap()),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn send_schedule_and_test_mail() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/pbrqulw2tc/action/send"))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/pbrqulw2tc/action/schedule"))
        .and(body_string(
            "delivery_requested=2026-05-18T14%3A30%3A00%2B02%3A00",
        ))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/pbrqulw2tc/action/testmail"))
        .and(body_string("email=me%40example.net"))
        .respond_with(respond("campaign"))
        .expect(1)
        .mount(&server)
        .await;

    let campaigns = client.campaigns();
    campaigns.send(&id()).await.unwrap();
    campaigns
        .schedule(
            &id(),
            &Timestamp::parse("2026-05-18T14:30:00+02:00").unwrap(),
        )
        .await
        .unwrap();
    campaigns
        .send_test(&id(), &Email::parse("me@example.net").unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn history_never_sends_member_only_filter() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/campaign/pbrqulw2tc/history"))
        .and(wiremock::matchers::query_param_is_missing("campaign_id"))
        .and(query_param("limit", "1000"))
        .respond_with(respond("history_page2"))
        .expect(1)
        .mount(&server)
        .await;

    let events: Vec<_> = client
        .campaigns()
        .history_stream(&id(), HistoryQuery::new().campaign(id()))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn campaign_without_audiences_or_screenshots_accepts_empty_arrays() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/campaign/pbrqulw2tc"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
            r#"{"campaign":{"campaign_id":"pbrqulw2tc","list_ids":[],"screenshot":[]}}"#,
        ))
        .mount(&server)
        .await;

    let campaign = client.campaigns().get(&id()).await.unwrap();
    assert!(campaign.list_ids.is_empty());
    assert!(campaign.screenshot.unwrap_or_default().is_empty());
}
