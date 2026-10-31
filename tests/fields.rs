mod common;

use common::{respond, setup};
use laposta_rs::fields::{DataType, DisplayType, FieldOption};
use laposta_rs::{FieldId, ListId, NewField, NewFieldKind, UpdateField};
use wiremock::Mock;
use wiremock::matchers::{body_json, body_string, method, path, query_param};

fn ids() -> (ListId, FieldId) {
    (ListId::new("BaImMu3JZA"), FieldId::new("gt2Em8vJwi"))
}

#[tokio::test]
async fn create_select_field_sends_bracketed_options() {
    let (server, client) = setup().await;
    let (list_id, _) = ids();
    Mock::given(method("POST"))
        .and(path("/v2/field"))
        .and(body_string(
            "datatype=select_single&datatype_display=radio&in_form=true&in_list=true&list_id=BaImMu3JZA&name=Color&options%5B%5D=Red&options%5B%5D=Green&required=true",
        ))
        .respond_with(respond("field"))
        .expect(1)
        .mount(&server)
        .await;

    let field = client
        .fields()
        .create(
            &list_id,
            &NewField::new(
                "Color",
                NewFieldKind::SelectSingle {
                    options: vec!["Red".into(), "Green".into()],
                    display: Some(DisplayType::Radio),
                },
            )
            .required(true),
        )
        .await
        .unwrap();

    assert_eq!(field.datatype, DataType::SelectSingle);
    assert_eq!(field.datatype_display, Some(DisplayType::Radio));
    assert_eq!(field.options_full[2], FieldOption::new("3", "Blue"));
    assert_eq!(field.tag, "{{color}}");
}

#[tokio::test]
async fn get_all_and_delete_pass_list_id_in_query() {
    let (server, client) = setup().await;
    let (list_id, field_id) = ids();
    Mock::given(method("GET"))
        .and(path("/v2/field/gt2Em8vJwi"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("field"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/field"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("fields"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v2/field/gt2Em8vJwi"))
        .and(query_param("list_id", "BaImMu3JZA"))
        .respond_with(respond("field"))
        .expect(1)
        .mount(&server)
        .await;

    let fields = client.fields();
    fields.get(&list_id, &field_id).await.unwrap();
    let all = fields.all(&list_id).await.unwrap();
    assert_eq!(all[1].datatype, DataType::Labels);
    fields.delete(&list_id, &field_id).await.unwrap();
}

#[tokio::test]
async fn unknown_display_type_cannot_be_sent() {
    let (_server, client) = setup().await;
    let (list_id, field_id) = ids();
    let err = client
        .fields()
        .update(
            &list_id,
            &field_id,
            &UpdateField::new().datatype_display(DisplayType::Unknown),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, laposta_rs::Error::InvalidInput(_)));
}

#[tokio::test]
async fn update_without_options_uses_form() {
    let (server, client) = setup().await;
    let (list_id, field_id) = ids();
    Mock::given(method("POST"))
        .and(path("/v2/field/gt2Em8vJwi"))
        .and(body_string("list_id=BaImMu3JZA&required=false"))
        .respond_with(respond("field"))
        .expect(1)
        .mount(&server)
        .await;

    client
        .fields()
        .update(&list_id, &field_id, &UpdateField::new().required(false))
        .await
        .unwrap();
}

#[tokio::test]
async fn update_with_options_full_uses_json() {
    let (server, client) = setup().await;
    let (list_id, field_id) = ids();
    Mock::given(method("POST"))
        .and(path("/v2/field/gt2Em8vJwi"))
        .and(body_json(serde_json::json!({
            "list_id": "BaImMu3JZA",
            "required": true,
            "options_full": [{"id": "1", "value": "Rood"}, {"id": "new-0", "value": "Paars"}]
        })))
        .respond_with(respond("field"))
        .expect(1)
        .mount(&server)
        .await;

    client
        .fields()
        .update(
            &list_id,
            &field_id,
            &UpdateField::new().required(true).options_full([
                FieldOption::new("1", "Rood"),
                FieldOption::new("new-0", "Paars"),
            ]),
        )
        .await
        .unwrap();
}
