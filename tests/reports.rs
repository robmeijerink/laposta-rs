mod common;

use common::{respond, setup};
use laposta_rs::CampaignId;
use wiremock::Mock;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn get_decodes_counts_and_ratios() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/report/njhgaf61ye"))
        .respond_with(respond("report"))
        .expect(1)
        .mount(&server)
        .await;

    let report = client
        .reports()
        .get(&CampaignId::new("njhgaf61ye"))
        .await
        .unwrap();
    assert_eq!(report.sent, 20882);
    assert_eq!(report.opened_unique, 5711);
    assert!((report.opened_ratio - 0.28).abs() < f64::EPSILON);
}

#[tokio::test]
async fn all_accepts_numbers_encoded_as_strings() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/report"))
        .respond_with(respond("reports"))
        .expect(1)
        .mount(&server)
        .await;

    let reports = client.reports().all().await.unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[1].sent, 6989);
    assert!((reports[0].accepted_ratio - 0.5).abs() < f64::EPSILON);
}

#[tokio::test]
async fn unparsable_counts_are_an_error_not_zero() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/report/c"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_string(r#"{"report":{"sent":"n/a"}}"#),
        )
        .mount(&server)
        .await;

    let err = client
        .reports()
        .get(&CampaignId::new("c"))
        .await
        .unwrap_err();
    assert!(matches!(err, laposta_rs::Error::Decode { .. }));
}
