mod common;

use std::time::Duration;

use common::{error, respond, setup};
use laposta_rs::{Client, Error, ErrorCode, ErrorType, ListId};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn invalid_input_error_is_typed() {
    let (server, client) = setup().await;
    Mock::given(method("GET")).and(path("/v2/list/x"))
        .respond_with(error(400, r#"{"error":{"type":"invalid_input","message":"Unknown list","code":203,"parameter":"list_id"}}"#))
        .mount(&server).await;

    let err = client.lists().get(&ListId::new("x")).await.unwrap_err();
    let api = err.as_api().unwrap();
    assert_eq!(api.status, 400);
    assert_eq!(api.kind, ErrorType::InvalidInput);
    assert_eq!(api.code, Some(ErrorCode::UnknownObject));
    assert_eq!(api.parameter.as_deref(), Some("list_id"));
    assert!(err.is_not_found());
}

#[tokio::test]
async fn unauthorized() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(error(
            401,
            r#"{"error":{"type":"invalid_request","message":"Invalid API key"}}"#,
        ))
        .mount(&server)
        .await;

    let err = client.lists().all().await.unwrap_err();
    assert!(err.is_unauthorized());
    assert_eq!(err.as_api().unwrap().code, None);
}

#[tokio::test]
async fn processor_agreement_missing() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/campaign/c/action/send"))
        .respond_with(error(
            402,
            r#"{"error":{"type":"invalid_request","message":"No agreement","code":214}}"#,
        ))
        .mount(&server)
        .await;

    let err = client.campaigns().send(&"c".into()).await.unwrap_err();
    assert_eq!(
        err.as_api().unwrap().code,
        Some(ErrorCode::ProcessorAgreementMissing)
    );
}

#[tokio::test]
async fn non_json_error_body_is_kept() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream exploded"))
        .mount(&server)
        .await;

    let err = client.lists().all().await.unwrap_err();
    let api = err.as_api().unwrap();
    assert_eq!(api.status, 500);
    assert_eq!(api.kind, ErrorType::Unknown);
    assert_eq!(api.message, "upstream exploded");
}

#[tokio::test]
async fn malformed_success_body_is_a_decode_error() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    assert!(matches!(
        client.lists().all().await.unwrap_err(),
        Error::Decode { .. }
    ));
}

#[tokio::test]
async fn rate_limited_without_retries_fails_with_retry_after() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(
            error(
                429,
                r#"{"error":{"type":"invalid_request","message":"Too many requests"}}"#,
            )
            .insert_header("Retry-After", "7"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let err = client.lists().all().await.unwrap_err();
    assert!(err.is_rate_limited());
    assert_eq!(
        err.as_api().unwrap().retry_after,
        Some(Duration::from_secs(7))
    );
}

#[tokio::test]
async fn rate_limit_is_retried_when_enabled() {
    let (server, _) = setup().await;
    let client = Client::builder()
        .api_key("test-key")
        .base_url(format!("{}/v2/", server.uri()))
        .max_retries(2)
        .build()
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(
            error(429, r#"{"error":{"message":"slow down"}}"#).insert_header("Retry-After", "0"),
        )
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(respond("lists"))
        .expect(1)
        .mount(&server)
        .await;

    assert_eq!(client.lists().all().await.unwrap().len(), 2);
}

#[tokio::test]
async fn gives_up_when_retries_are_exhausted() {
    let (server, _) = setup().await;
    let client = Client::builder()
        .api_key("test-key")
        .base_url(format!("{}/v2/", server.uri()))
        .max_retries(1)
        .build()
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(
            error(429, r#"{"error":{"message":"slow down"}}"#).insert_header("Retry-After", "0"),
        )
        .expect(2)
        .mount(&server)
        .await;

    assert!(client.lists().all().await.unwrap_err().is_rate_limited());
}

#[test]
fn builder_requires_an_api_key_and_a_usable_base_url() {
    assert!(matches!(
        Client::builder().build(),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        Client::builder().api_key("k").base_url("not a url").build(),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        Client::builder()
            .api_key("k")
            .base_url("mailto:x@y.z")
            .build(),
        Err(Error::InvalidInput(_))
    ));
}

#[test]
fn debug_output_hides_the_api_key() {
    let client = Client::new("super-secret-key");
    assert!(!format!("{client:?}").contains("super-secret-key"));
    let builder = Client::builder().api_key("super-secret-key");
    assert!(!format!("{builder:?}").contains("super-secret-key"));
}

#[test]
fn plain_http_is_only_allowed_on_loopback() {
    let build = |url: &str| Client::builder().api_key("k").base_url(url).build();
    assert!(matches!(
        build("http://api.laposta.nl/v2/"),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        build("ftp://example.org/v2/"),
        Err(Error::InvalidInput(_))
    ));
    for ok in [
        "https://example.org/v2/",
        "http://localhost:8080/v2/",
        "http://127.0.0.1:8080/v2/",
        "http://[::1]:8080/v2/",
    ] {
        assert!(build(ok).is_ok(), "{ok}");
    }
}

#[tokio::test]
async fn redirects_are_not_followed() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v2/list"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/v2/elsewhere"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/v2/elsewhere"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let err = client
        .lists()
        .create(&laposta_rs::NewList::new("x"))
        .await
        .unwrap_err();
    assert_eq!(err.as_api().unwrap().status, 302);
}

#[tokio::test]
async fn slow_responses_time_out() {
    let (server, _) = setup().await;
    let client = Client::builder()
        .api_key("test-key")
        .base_url(format!("{}/v2/", server.uri()))
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/v2/list"))
        .respond_with(respond("lists").set_delay(Duration::from_millis(500)))
        .mount(&server)
        .await;

    match client.lists().all().await.unwrap_err() {
        Error::Http(e) => assert!(e.is_timeout()),
        other => panic!("expected a timeout, got {other:?}"),
    }
}
