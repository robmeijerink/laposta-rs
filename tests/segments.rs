mod common;

use common::{respond, setup};
use laposta_rs::{ListId, NewSegment, SegmentId, UpdateSegment};
use wiremock::Mock;
use wiremock::matchers::{body_string, method, path, query_param};

#[tokio::test]
async fn crud() {
    let (server, client) = setup().await;
    let list_id = ListId::new("BaImMu3JZA");
    let segment_id = SegmentId::new("pDrp5KOD0z");

    Mock::given(method("POST"))
        .and(path("/v2/segment"))
        .and(body_string(
            "definition=%7B%22postcode%22%7D+%3D+%271234AB%27&list_id=BaImMu3JZA&name=Postcode",
        ))
        .respond_with(respond("segment"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/segment/pDrp5KOD0z"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("segment"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/segment"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("segments"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/segment/pDrp5KOD0z"))
        .and(body_string("list_id=BaImMu3JZA&name=Renamed"))
        .respond_with(respond("segment"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/segment/pDrp5KOD0z"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("segment"))
        .expect(1)
        .mount(&server)
        .await;

    let segments = client.segments();
    let created = segments
        .create(
            &list_id,
            &NewSegment::new("Postcode", "{\"postcode\"} = '1234AB'"),
        )
        .await
        .unwrap();
    assert_eq!(created.segment_id, segment_id);
    assert_eq!(created.definition, "{\"postcode\"} = '1234AB'");
    segments.get(&list_id, &segment_id).await.unwrap();
    assert_eq!(segments.all(&list_id).await.unwrap().len(), 1);
    segments
        .update(&list_id, &segment_id, &UpdateSegment::new().name("Renamed"))
        .await
        .unwrap();
    segments.delete(&list_id, &segment_id).await.unwrap();
}
