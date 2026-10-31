mod common;

use common::{respond, setup};
use laposta_rs::{ListId, NewWebhook, UpdateWebhook, WebhookEvent, WebhookId};
use wiremock::Mock;
use wiremock::matchers::{body_string, method, path, query_param};

#[tokio::test]
async fn crud() {
    let (server, client) = setup().await;
    let list_id = ListId::new("BaImMu3JZA");
    let webhook_id = WebhookId::new("cW5ls8IVJl");

    Mock::given(method("POST")).and(path("/v2/webhook"))
        .and(body_string("blocked=false&event=subscribed&list_id=BaImMu3JZA&url=https%3A%2F%2Fexample.net%2Fhook"))
        .respond_with(respond("webhook")).expect(1).mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/v2/webhook/cW5ls8IVJl"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("webhook"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/webhook"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("webhooks"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/webhook/cW5ls8IVJl"))
        .and(body_string(
            "blocked=true&event=deactivated&list_id=BaImMu3JZA",
        ))
        .respond_with(respond("webhook"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/webhook/cW5ls8IVJl"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("webhook"))
        .expect(1)
        .mount(&server)
        .await;

    let webhooks = client.webhooks();
    let created = webhooks
        .create(
            &list_id,
            &NewWebhook::new(
                WebhookEvent::Subscribed,
                "https://example.net/hook".parse().unwrap(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(created.event, WebhookEvent::Subscribed);
    assert_eq!(created.secret.as_ref().map(|s| s.expose()), Some("s3cret"));
    assert!(!format!("{created:?}").contains("s3cret"));
    webhooks.get(&list_id, &webhook_id).await.unwrap();
    assert_eq!(webhooks.all(&list_id).await.unwrap().len(), 1);
    webhooks
        .update(
            &list_id,
            &webhook_id,
            &UpdateWebhook::new()
                .event(WebhookEvent::Deactivated)
                .blocked(true),
        )
        .await
        .unwrap();
    webhooks.delete(&list_id, &webhook_id).await.unwrap();
}
